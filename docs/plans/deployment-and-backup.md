---
title: "hKask Deployment & Multi-User Plan"
audience: [architects, developers]
last_updated: 2026-06-17
version: "0.27.0"
status: "Draft — Planning Phase"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
anchored_on: [PRINCIPLES.md §0, P1, P2, P3, P9, P12]
reviewed_via: [pragmatic-laziness, essentialist, grill-me, coding-guidelines]
---

# hKask Deployment & Multi-User Plan

**Purpose:** Define the cloud server deployment model, multi-user OAuth sign-in, terminal session provisioning, the backup-as-portable-sovereignty-archive model, and the server migration procedure.

**Decision:** There is no separate client binary. Users sign in via OAuth (GitHub/Google) and access hKask through a terminal session on the cloud server — either SSH or a browser-based terminal. The "client" is a session, not a binary.

**Status:** Planning phase. Converged design after multi-perspective review. No implementation has begun.

---

## 1. Architecture — Single Node, Multi-User

### 1.1 Deployment Model

One binary (`kask`), one server, many users. Each user gets a terminal session scoped to their WebID.

```
┌──────────────────────────────────────────────────────────────┐
│                    CLOUD SERVER (single hKask install)        │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────────────────────┐  │
│  │  Caddy            │  │  hKask Core                      │  │
│  │  (Docker)         │  │  (daemon + MCP servers + agents) │  │
│  │  TLS + proxy      │  │                                  │  │
│  │  ports 80, 443    │  │  hkask-mcp-communication         │  │
│  └────────┬──────────┘  │  └─ MatrixTransport (SDK)        │  │
│           │             │                                  │  │
│  ┌────────┴──────────┐  │  hkask-api (REST)                │  │
│  │  Conduit           │  │  └─ OAuth callback endpoints    │  │
│  │  (Docker)         │◄─┤  └─ Session management           │  │
│  │  homeserver       │  │  └─ Backup export endpoints      │  │
│  │  localhost:8008   │  │                                  │  │
│  └───────────────────┘  │  Curator → Agent Pods             │  │
│                         │  Wallet (cloud-only)              │  │
│                         │  Multi-user TripleStore           │  │
│                         │  (scoped by owner_webid)          │  │
│                         └──────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
         │
         │ HTTPS (TLS via Caddy)
         │
   ┌─────┴──────┬──────────────┐
   │            │              │
┌──▼──┐   ┌────▼─────┐  ┌─────▼─────┐
│SSH  │   │Browser   │  │Matrix     │
│term │   │terminal  │  │client     │
│     │   │(WebTTY)  │  │(Element)  │
└─────┘   └──────────┘  └───────────┘
```

### 1.2 Caddy + Conduit Sidecars

The cloud server includes two Docker sidecars. hKask generates configuration; the user runs Docker.

| Sidecar | Role | Binary Size |
|---------|------|-------------|
| **Caddy** | TLS termination (auto Let's Encrypt), reverse proxy, `/.well-known` delegation | ~20 MB |
| **Conduit** | Matrix homeserver (Rust-native, SQLite-backed) | ~50 MB |

Deployed via `kask matrix deploy-sidecar --domain <domain>`. Full architecture: [`docs/architecture/matrix-integration-architecture.md`](../architecture/matrix-integration-architecture.md). Implementation: [`crates/hkask-cli/src/commands/matrix.rs`](../../crates/hkask-cli/src/commands/matrix.rs).

### 1.3 Wallet — Cloud-Only

Wallet operations (rJoule payments, multi-chain deposits, API key issuance, withdrawals) run exclusively on the server. Rationale: crypto signing keys on a managed server have a smaller attack surface than on user devices. Blockchain RPC access may be IP-firewalled to the server.

---

## 2. Multi-User Sign-In — OAuth + Terminal Session

### 2.1 Sign-In Flow

```
User visits https://my-server.hkask.example/login
  │
  ├── "Sign in with GitHub" → redirect to GitHub OAuth
  ├── "Sign in with Google"  → redirect to Google OAuth
  │
  ▼
OAuth provider authenticates user, returns authorization code
  │
  ▼
Server exchanges code for access token, retrieves user profile
  (email, provider user ID, display name)
  │
  ▼
Server looks up or creates HumanUser record keyed by (provider, provider_user_id)
  │  ┌─ New user: provision WebID, create default replicant, assign wallet
  │  └─ Returning user: load existing WebID, replicants, session
  │
  ▼
Server creates UserSession { session_id, webid, expires_at }
  Sets session cookie or returns session token
  │
  ▼
User lands in terminal session:
  - SSH: server provisions a restricted shell session (or the user SSHes with an authorized key)
  - Browser: WebTTY terminal embedded in the page, connected to server-side PTY
  │
  ▼
User runs `kask` within the session. All operations scoped to their WebID.
```

### 2.2 Existing Infrastructure for Auth

| Component | What It Does | Crate |
|-----------|-------------|-------|
| `UserStore` | Human user registration, Argon2id passphrase hashing, encrypted PII, session management | `hkask-storage` |
| `UserSession` | Session ID, WebID, expiry, last_active | `hkask-types` |
| `ReplicantIdentity` | Replicant name, WebID, wallet ID, persona, login tracking | `hkask-types` |
| `AuthService` | Capability token verification (Ed25519), revocation tracking | `hkask-api` |
| `AuthContext` | Attached to request extensions after auth middleware passes | `hkask-api` |

### 2.3 What's New for OAuth

| Addition | Description |
|----------|-------------|
| OAuth provider config | `OAUTH_GITHUB_CLIENT_ID`, `OAUTH_GITHUB_CLIENT_SECRET`, `OAUTH_GOOGLE_CLIENT_ID`, `OAUTH_GOOGLE_CLIENT_SECRET` — stored in server config or OS keychain |
| `/api/v1/auth/login` | Initiates OAuth flow — returns redirect URL for chosen provider |
| `/api/v1/auth/callback` | OAuth callback — exchanges code for token, creates/loads user, starts session |
| `OAuthProvider` enum | `GitHub`, `Google` — maps to provider-specific token exchange and profile fetch |
| `HumanUser.provider` / `HumanUser.provider_user_id` | Links hKask identity to OAuth identity |
| Session cookie or bearer token | Returned after OAuth callback — used for subsequent API calls and terminal auth |

### 2.4 Terminal Session Provisioning

Two options, not mutually exclusive:

| Mode | How It Works | Infrastructure |
|------|-------------|---------------|
| **SSH** | User adds their SSH public key via the web dashboard. Server provisions a restricted shell that drops them into `kask repl` scoped to their WebID. | Standard SSH with `ForceCommand` or a custom shell binary. |
| **Web terminal** | Browser-based PTY using WebTTY or ttyd. Server spawns a `kask repl` process per connected user. | Lightweight WebTTY container or embedded terminal in the hKask web dashboard. |

**CNS span:** `SessionOpen { user_id, provider, mode }`, `SessionClose { user_id, duration }`.

### 2.5 Multi-Tenant Scoping

Every operation in the server is scoped to the authenticated user's WebID:

- **TripleStore queries:** `WHERE owner_webid = ?` — users can only access their own triples.
- **Agent pods:** spawned under the user's WebID, resource-limited per user.
- **Inference:** quota-tracked per user (CNS energy budget, wallet balance).
- **Wallet:** deposits, balances, API keys are per-user.
- **Backup:** export is scoped to the authenticated WebID.

---

## 3. Install Process

### 3.1 Server Install

```bash
# Build
cargo build --release --bin kask

# Install
cp target/release/kask /usr/local/bin/kask

# Initialize
kask init --profile server
# Prompts for:
#   - Server master passphrase (Argon2id → HKDF key derivation)
#   - Admin WebID creation
#   - Data directory (default: /var/lib/hkask/)
#   - Domain name (for Caddy TLS + OAuth redirect URIs)
#   - OAuth provider credentials (GitHub and/or Google client ID + secret)
```

**What this creates:**
- `~/.config/hkask/` — server config, keystore version file, OAuth provider config
- `/var/lib/hkask/` — SQLCipher database (all user data), git backup repository
- OS keychain entry: `hkask-master` (master passphrase)
- OAuth credentials stored in OS keychain: `hkask-oauth-github`, `hkask-oauth-google`

**After init, deploy sidecars:**
```bash
kask matrix deploy-sidecar --domain my-server.hkask.example
cd ~/.config/hkask/sidecar && docker compose up -d
```

### 3.2 User Onboarding

No install. Users visit `https://my-server.hkask.example/login`, sign in with GitHub or Google, and get a terminal session. First sign-in provisions their WebID, default replicant, and wallet.

---

## 4. Backup Model — Server-Side Export as Portable Sovereignty Archive

### 4.1 What the Backup Is

The backup archive is a **single SQLCipher-encrypted SQLite file** containing:

1. The user's full live triple set (post-pruning, post-consolidation) — a snapshot of the server's current state for that user
2. A `backup_meta` table describing the archive

The archive is the **P1 data portability artifact** — the user downloads it from one hKask server, uploads it to another, and resumes. No client binary. No sync protocol. No pull schedule. A file.

### 4.2 Archive Schema

```sql
CREATE TABLE backup_meta (
    webid TEXT NOT NULL,
    source_server_url TEXT NOT NULL,
    exported_at TEXT NOT NULL,
    triple_count INTEGER NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1
);

-- Reuses existing TripleStore schema
```

### 4.3 Export (Download)

```
User runs: kask backup export
  │
  ▼
Server: SELECT * FROM triples
        WHERE owner_webid = ? AND tombstone = false
  │
  ▼
Server writes to SQLCipher database, encrypted with user-provided passphrase
  (user enters passphrase at export time — not stored on server)
  │
  ▼
Server writes backup_meta, returns file path
  │
  ▼
User downloads via: scp, kask backup download, or GET /api/v1/backup/export
  │
  ▼
CnsSpan::BackupExport { triple_count, bytes, duration_ms }
```

**Why user-provided passphrase at export time:**
- The server never stores the user's backup password.
- Each export can use a different passphrase.
- The archive is useless without the passphrase — P1 sovereignty even when the file is in transit.

### 4.4 Scheduled Exports (Optional)

The server can be configured to periodically prepare an encrypted backup archive for each user:

```bash
kask config set backup.auto-export.frequency daily
kask config set backup.auto-export.retention 7  # keep last 7 exports
```

Archives are stored in `/var/lib/hkask/exports/{webid}/` and available for download via the API. Each archive is encrypted with a key derived from the user's session — the user provides their passphrase at download time to decrypt.

**CNS span:** `BackupAutoExport { webid, triple_count, bytes, duration_ms }`

### 4.5 CNS Observability

| Span | Tracks | Alert |
|------|--------|-------|
| `BackupExport` | `triple_count`, `bytes`, `duration_ms` | Informational |
| `BackupAutoExport` | `triple_count`, `bytes`, `duration_ms`, `webid` | Failure alert if scheduled export fails |
| `BackupUpload` | `triple_count`, `bytes_sent`, `duration_ms` (migration) | Informational |

---

## 5. Server Migration

### 5.1 Flow

```
User has: backup archive downloaded from old server

kask backup upload --server https://new-server.hkask.example
  │
  ▼
User authenticates to new server (OAuth sign-in — creates account if new)
  │
  ▼
User provides archive file + passphrase
  │
  ▼
New server decrypts archive, checks schema_version, verifies webid match
  │
  ▼
For each replicant entity in the archive:
  ├── Name collision with existing replicant on new server?
  │     YES → auto-rename: "ada" → "ada-migrated-20260617"
  │     NO  → import as-is
  │
  ▼
All triples upserted into new server's TripleStore (idempotent)
  │
  ▼
New server returns MigrationReceipt { triple_count, renamed_replicants: [...] }
  │
  ▼
User sees: "Archive imported. X triples.
  Renamed replicants: ada → ada-migrated-20260617
  Run `kask replicate merge --from ada-migrated-20260617 --into ada` to reconcile."
```

### 5.2 Replicant Operations

| Command | What It Does |
|---------|-------------|
| `kask replicate rename <from> <to>` | Rename a replicant entity |
| `kask replicate merge --from <source> --into <target>` | Upsert all triples from source entity into target entity. Source unchanged. |
| `kask replicate delete <name>` | Remove a replicant and all its triples |

**Merge semantics:** `INSERT OR REPLACE` by TripleID. Idempotent — running merge twice produces the same result. CNS span: `ReplicantMerge`.

### 5.3 Fault Tolerance

The CRDT merge (`INSERT OR REPLACE` by TripleID) is associative, commutative, and idempotent. Export and upload are resumable by retry:

- **Interrupted export:** Re-run `kask backup export`. Fresh snapshot.
- **Interrupted upload:** Re-run `kask backup upload`. 73% of triples already merged (no-ops), 27% fill in. Converged.

No progress tracking. No chunking protocol. The merge is the fault tolerance.

### 5.4 No Server-to-Server Protocol

Servers never communicate. Migration is user-mediated: download archive from old server, upload to new server. The archive file is the bridge. P1: the user controls the transfer.

---

## 6. Encryption Model

### 6.1 Two Layers, Two Keys

| Layer | Key | Purpose |
|-------|-----|---------|
| Transport | Caddy TLS (Let's Encrypt) | HTTPS for all browser/API traffic. SSH for terminal sessions. |
| Storage | User-provided passphrase at export time | Encrypts the backup archive (SQLCipher). Server never stores this. |

### 6.2 Server-Side Encryption

- **TripleStore:** SQLCipher-encrypted, key derived from server master passphrase (Argon2id → AES-256).
- **Git backup:** Already implemented — AES-256-GCM encrypted blobs before CAS storage.
- **PII:** Encrypted in `UserStore` with per-user PII key.

### 6.3 Key Rotation

Server key rotation follows the existing `hkask-keystore::master_key` pattern: increment `key_version`, old-version keys remain derivable. Zero new crypto code.

---

## 7. CLI Command Surface

```
kask init --profile server
    Initialize server, configure domain, OAuth providers, master passphrase.

kask matrix deploy-sidecar --domain <domain> [--with-web-client]
    Generate Caddy + Conduit docker-compose and config files.

kask matrix status-sidecar
    Health check Caddy, Conduit, and Hydrogen containers.

kask backup export [--passphrase <passphrase>]
    Export encrypted backup archive for the authenticated user.

kask backup download [--path <path>]
    Download the latest backup archive (API endpoint wrapper).

kask backup upload --server <url> [--archive <path>]
    Upload backup archive to a new server for migration.

kask backup auto-export --frequency <daily|weekly> --retention <days>
    Configure scheduled server-side backup archive generation.

kask replicate rename <from> <to>
kask replicate merge --from <source> --into <target>
kask replicate delete <name>
    Manage replicants after migration.
```

---

## 8. Type Summary

### 8.1 New Types

| Type | Crate | Fields / Variants |
|------|-------|-------------------|
| `OAuthProvider` | `hkask-api` | `GitHub`, `Google` |
| `OAuthConfig` | `hkask-api` | `client_id: String`, `client_secret: SecretRef`, `redirect_uri: String` |
| `OAuthUserProfile` | `hkask-api` | `provider: OAuthProvider`, `provider_user_id: String`, `email: String`, `display_name: String` |
| `BackupArchive` | `hkask-storage` | Wraps `Database` (SQLCipher) — methods: `create(user_passphrase, triples)`, `open(user_passphrase)`, `metadata()` |
| `MigrationReceipt` | `hkask-storage` | `triple_count: u64`, `renamed_replicants: Vec<(String, String)>` |
| `MergeReceipt` | `hkask-storage` | `triple_count: u64`, `source: String`, `target: String` |

### 8.2 CNS Span Additions

```rust
CnsSpan::SessionOpen,      // { user_id, provider, mode }
CnsSpan::SessionClose,     // { user_id, duration }
CnsSpan::BackupExport,     // { triple_count, bytes, duration }
CnsSpan::BackupAutoExport, // { webid, triple_count, bytes, duration }
CnsSpan::BackupUpload,     // { triple_count, bytes, duration }
CnsSpan::ReplicantMerge,   // { source, target, triple_count, duration }
```

### 8.3 API Endpoints (New)

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/v1/auth/login?provider=github` | Initiate OAuth flow |
| `GET` | `/api/v1/auth/callback?provider=github&code=...` | OAuth callback |
| `GET` | `/api/v1/auth/session` | Return current session info |
| `POST` | `/api/v1/auth/logout` | Destroy session |
| `POST` | `/api/v1/backup/export` | Generate and return encrypted backup archive |
| `GET` | `/api/v1/backup/download` | Download latest backup archive |
| `POST` | `/api/v1/backup/upload` | Upload backup archive for migration |
| `POST` | `/api/v1/replicants/merge` | Merge two replicants |
| `POST` | `/api/v1/replicants/rename` | Rename a replicant |
| `DELETE` | `/api/v1/replicants/{name}` | Delete a replicant |

---

## 9. Existing Infrastructure Reused

| Infrastructure | Used For | Crate |
|---------------|----------|-------|
| `UserStore` + `HumanUser` + `UserSession` | User identity, sessions, PII encryption | `hkask-storage` |
| `AuthService` + `AuthContext` | Session auth middleware, token verification | `hkask-api` |
| `TripleStore` + `Triple` | User data query, backup archive creation, migration import | `hkask-storage` |
| `Database::open_impl` | SQLCipher-encrypted backup archive | `hkask-storage` |
| `ConsentStore` | `BackupRestore` scope for migration authorization | `hkask-storage` |
| `Keychain` | Server master passphrase, OAuth client secrets | `hkask-keystore` |
| `EncryptionService` (AES-256-GCM) | Git backup encryption | `hkask-keystore` |
| `derive_sub_key_with_version` | Server-side key derivation with rotation | `hkask-keystore` |
| `hkask-api` (axum) | OAuth endpoints, backup endpoints, session management | `hkask-api` |
| `CnsSpan` + `AlgedonicManager` + `SetPoints` | Session and backup observability | `hkask-cns`, `hkask-types` |
| `hkask-memory` (consolidation, salience, condensation) | Bounds backup archive size | `hkask-memory` |
| Git backup (`BackupService`) | Server-side operational backup (complementary) | `hkask-services` |
| Caddy + Conduit sidecar | TLS, Matrix homeserver | Docker (config generated by `kask matrix deploy-sidecar`) |

---

## 10. What Is NOT Being Built

Explicit exclusions — considered and rejected:

- **No separate client binary.** The "client" is a terminal session (SSH or browser-based).
- **No feature gating or Cargo features.** Single binary, all crates compiled.
- **No SyncPort trait.** No client-server sync protocol — backup is a file export.
- **No client registration protocol.** Users authenticate via OAuth. Sessions are server-managed.
- **No CRDT pull/upload streaming protocol.** Backup is a file. CRDT idempotence provides fault tolerance for migration uploads.
- **No client-side encryption key management.** User provides passphrase at export time. Server never stores it.
- **No server-to-server protocol.** Migration is user-mediated via downloadable archive.
- **No conflict resolution UI.** Replicant merge is user-initiated, idempotent upsert.
- **No backup pruning code.** Server's memory pipeline handles pruning.
- **No artifact replication (LORA, research files).** Out of scope. Backup covers triples only.

---

## 11. Success Criteria

```
1. [Deploy]  kask init --profile server
             kask matrix deploy-sidecar --domain example.com
             → Caddy serves HTTPS, Conduit responds on /_matrix/

2. [Auth]    User visits /login, signs in with GitHub
             → OAuth callback succeeds, session created
             → User lands in terminal session, kask repl scoped to their WebID

3. [Export]  kask backup export --passphrase "user-chosen"
             → archive.db created, encrypted with passphrase
             → CnsSpan::BackupExport emitted

4. [Download] scp archive.db user@laptop:~
             → file opens only with correct passphrase
             → triple count matches server

5. [Migrate] kask backup upload --server https://new-server.example
             → MigrationReceipt.triple_count matches archive count
             → replicants renamed on collision

6. [Merge]   kask replicate merge --from ada-migrated-xxx --into ada
             → triples merged, source unchanged

7. [Multi]   Users A and B both signed in
             → A cannot see B's triples, pods, or wallet
             → B cannot see A's triples, pods, or wallet
```

---

## 12. Open Questions

| # | Question | Why Deferred |
|---|----------|-------------|
| Q1 | Browser terminal vs SSH-only? SSH is simpler (no WebTTY dependency), but browser terminal lowers the barrier for non-technical users. | Deploy and gather user feedback. |
| Q2 | Should the server auto-provision SSH authorized_keys from the web dashboard? Or is manual key addition acceptable? | Depends on target audience. |
| Q3 | Should auto-export archives be encrypted with the user's session key (server-side) or require a passphrase at download time? | Session-key encryption is more convenient but means the server briefly holds the encryption key. Passphrase-at-download is more secure but requires the user to be present. |
| Q4 | OAuth provider scope: GitHub only? GitHub + Google? Add more later? | Start with GitHub (developer audience). Add Google if demand exists. |

---

## 13. Implementation Sequence

| Phase | Tasks | Depends On |
|-------|-------|-----------|
| **Phase 1 — OAuth** | `OAuthProvider`, OAuth config, `/auth/login` + `/auth/callback` endpoints, session cookie, `HumanUser.provider` fields | — |
| **Phase 2 — Sessions** | Session provisioning (SSH ForceCommand or WebTTY), session-scoped `kask repl`, `SessionOpen`/`SessionClose` CNS spans | Phase 1 |
| **Phase 3 — Backup** | `BackupArchive` type, `kask backup export`, `GET /api/v1/backup/download`, auto-export scheduler, CNS spans | Phase 1 |
| **Phase 4 — Migration** | `kask backup upload`, replicant rename/merge/delete, `MigrationReceipt`, auto-rename on collision | Phase 3 |
| **Phase 5 — Integration** | End-to-end: deploy → OAuth sign-in → export → upload to second server → merge → verify | Phase 4 |
| **Phase 6 — Harden** | Interruption testing, multi-user isolation testing, backup auto-export tuning | Phase 5 |
