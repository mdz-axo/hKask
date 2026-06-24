---
title: "hKask Deployment & Multi-User Plan"
audience: [architects, developers]
last_updated: 2026-06-22
version: "0.30.0"
status: "Draft — Aligned with FUNCTIONAL_SPECIFICATION.md Domain 26"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
anchored_on: [PRINCIPLES.md §0, P1, P2, P3, P9, P12]
reviewed_via: [pragmatic-laziness, essentialist, grill-me, coding-guidelines]
---

# hKask Deployment & Multi-User Plan

**Purpose:** Define the cloud server deployment model, multi-user OAuth sign-in, terminal session provisioning, the backup-as-portable-sovereignty-archive model, and the server migration procedure.

**Decision:** There is no client — no binary, no install, no SSH setup. Users visit a website, sign in with GitHub or Google, and get a terminal. The "client" is a browser tab running xterm.js connected to the server via WebSocket. The server spawns `kask repl` on a PTY and pipes I/O. That is the entire product surface.

**Status:** Partially implemented. Phases 1–4 code exists (routes, types, CLI scaffolding). Awaiting Phase 5 integration (CNS instrumentation, CLI commands, end-to-end verification). See §13 for per-phase status.

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
│                         │  Per-pod SQLCipher files          │  │
│                         │  ({data_dir}/agents/{sanitized_name}/pod.db)   │
│                         └──────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
         │
         │ HTTPS (TLS via Caddy)
         │
   ┌─────┴──────┬──────────────┐
   │            │              │
┌──▼──┐   ┌────▼─────┐  ┌─────▼─────┐
│SSH  │   │Browser   │  │Matrix     │
│     │   │(xterm.js)│  │client     │
│(opt)│   │          │  │(Element)  │
└─────┘   └──────────┘  └───────────┘
```

**Primary access:** Browser. OAuth sign-in → WebSocket terminal.
**Optional access:** SSH for power users who add their key via `kask ssh-key add`.

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

### 2.1 Sign-In → Terminal

```
User visits https://my-server.hkask.example
  │
  ▼
Login page: "Sign in with GitHub" | "Sign in with Google"
  (one HTML page, two buttons, zero JavaScript framework)
  │
  ├── GitHub OAuth → callback → session cookie set
  ├── Google OAuth  → callback → session cookie set
  │
  ▼
Redirect to /terminal
  │
  ▼
Terminal page: xterm.js in the browser
  Connected via WebSocket to /api/v1/terminal/ws
  Server spawns `kask repl --webid <user>` on a PTY
  Pipes stdin/stdout over the WebSocket
  │
  ▼
User is in hKask. Done.
```

### 2.2 Terminal Implementation

The terminal page is two pieces:

| Piece | What | Technology |
|--------|------|------------|
| Frontend | Terminal emulator in the browser | xterm.js (MIT, the same library VS Code and Codespaces use) |
| Backend | PTY spawner + WebSocket bridge | `tokio-pty-process` spawns `kask repl`, `axum` WebSocket pipes I/O |

**Total new code:** ~200 lines of Rust (WebSocket handler + PTY spawn). One static HTML file (~50 lines). No React, no SPA, no framework. xterm.js is loaded from a CDN or bundled as a static asset.

**How it works:**
```
Browser                          Server
  │                               │
  │── WS /api/v1/terminal/ws ────>│  (session cookie attached)
  │                               │  verify session → extract webid
  │                               │  spawn: kask repl --webid <webid>
  │                               │  PTY master ←→ child process
  │<── PTY stdout ────────────────│  (streamed over WebSocket)
  │── keystrokes ────────────────>│  (written to PTY stdin)
```

**Optional SSH access:** Power users can run `kask ssh-key add` in the repl to register an SSH public key. The server adds it to `authorized_keys` with `ForceCommand kask repl --webid <user>`. Then `ssh user@my-server.hkask.example` works. But this is secondary — the browser terminal is the default.

### 2.3 Existing Infrastructure for Auth

| Component | What It Does | Crate |
|-----------|-------------|-------|
| `UserStore` | Human user registration, Argon2id passphrase hashing, encrypted PII, session management | `hkask-storage` |
| `UserSession` | Session ID, WebID, expiry, last_active | `hkask-types` |
| `ReplicantIdentity` | Replicant name, WebID, wallet ID, persona, login tracking | `hkask-types` |
| `AuthService` | Capability token verification (Ed25519), revocation tracking | `hkask-api` |
| `AuthContext` | Attached to request extensions after auth middleware passes | `hkask-api` |

### 2.4 What's New

| Addition | Description |
|----------|-------------|
| OAuth provider config | Client ID + secret for GitHub and Google, stored in OS keychain |
| `/api/v1/auth/login` | Initiates OAuth flow |
| `/api/v1/auth/callback` | OAuth callback, creates/loads user, starts session |
| `OAuthProvider` enum | `GitHub`, `Google` |
| `HumanUser.provider` fields | Links hKask identity to OAuth identity |
| Session cookie | Set after OAuth callback |
| `/api/v1/terminal/ws` | WebSocket endpoint, verifies session, spawns PTY |

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

No install. No SSH keys. User visits `https://my-server.hkask.example`, clicks "Sign in with GitHub," and gets a terminal. First sign-in provisions their WebID, default replicant, and wallet.

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
User runs: kask export create
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
User downloads via: scp, browser download (`GET /api/v1/export/download`), or direct file access
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

### 4.6 Relationship to Operational Backup

hKask maintains **two independent backup systems** with distinct purposes:

| System | Storage | Encryption | Purpose | CLI Namespace |
|--------|---------|-----------|---------|---------------|
| **Sovereignty Export** (this plan) | SQLCipher SQLite file | User passphrase at export time | P1 data portability — download and migrate to another server | `kask export` |
| **Operational Backup** (existing) | Git CAS via `GixCasAdapter` (pure Rust gix) | Server-side AES-256-GCM | Server disaster recovery — artifact versioning, retention, integrity verification, agent revert/spawn | `kask backup`, `kask agent revert`, `kask agent spawn-agent` |

The operational backup is implemented in `hkask-services-backup`:

- **`GitCASPort`** — 8-method hexagonal port (CRUD + snapshot + inspection) for content-addressed git storage across 8 repos (`Registry`, `Memory`, `CnsAudit`, `Sovereignty`, `GoalsSpecs`, `Sessions`, `Vault`, `Pods`)
- **`BackupService`** — 7-artifact operations (snapshot, restore, list, prune, verify, config, update_config) with mutual-exclusion gate (`AtomicBool` CAS), config injection, and encryption (AES-256-GCM + Argon2)
- **`PodBackupOps`** — 2 pod operations (revert, spawn_agent) sharing the same CAS port and gate, with atomic pod.db writes (temp file + rename) and safety snapshots before revert
- **`BackupLoop`** — cybernetic loop (sense → compare → compute → act) running daily snapshots with 1-hour failure dampener

Key properties:
- Pruning actually deletes blobs via `delete_blob` before orphan commits
- `SnapshotMetadata.trigger` and `.artifact_count` are `Option` — honest about what the git log carries
- `ArtifactType` uses `strum` for single-source label↔variant mapping
- `TemplateCrateLoader` (formerly `GitCasAdapter`) is distinct from `GixCasAdapter` — loads template crates from disk, not CAS operations

The operational backup runs automatically via the CNS cybernetic loop system and is *not* user-exportable. The sovereignty export is the user-facing P1 artifact — a downloadable, passphrase-encrypted archive that the user controls end-to-end.

### 4.7 System Boundary — What Each System Backs Up

The two systems serve different purposes and do NOT overlap:

| What | Backed Up By |
|------|-------------|
| Pod identity and state (pod.db) | Operational Backup (PodState → RepoId::Pods) |
| User triples (semantic memory) | Sovereignty Export (SQLCipher archive) |
| Templates, styles, skills | Operational Backup (Registry repo) |
| Specifications and goals | Operational Backup (GoalsSpecs repo) |
| CNS audit trail | Operational Backup (CnsAudit repo) |
| Wallet state | Operational Backup (Vault repo) |
| Backup configuration (backup.json) | Operational Backup (self-backup via ConfigProducer) |

**Pod revert does NOT revert user triples.** The pod.db contains agent identity, persona, and internal state. User data (triples) lives in the semantic store and is backed up via sovereignty export. Reverting a pod restores the agent's configuration and state; reverting user data requires restoring a sovereignty export archive.

### 4.8 Disaster Recovery Procedure

To reconstruct a running system from CAS repos after total server loss:

```bash
# 1. Restore the backup configuration
kask backup restore --commit <latest-settings-commit> --scope settings
# This restores backup.json → determines what was tracked

# 2. Restore templates and agent definitions
kask backup restore --commit <latest-registry-commit> --scope template

# 3. Restore pod states
kask backup list --type pod_state
# Choose the latest commit for each pod
kask agent revert <pod-name> --commit <commit> --reason "disaster recovery"
# Then restart each pod

# 4. Restore specifications
kask backup restore --commit <latest-spec-commit> --scope spec

# 5. Verify integrity
kask backup verify
```

The ordered sequence matters: templates first (so pods can deploy), then pods (so agents are active), then specs (operational metadata). CNS audit and wallet state can be restored last or on-demand.

---

## 5. Server Migration

### 5.1 Flow

```
User has: backup archive downloaded from old server

kask export upload --server https://new-server.hkask.example
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

- **Interrupted export:** Re-run `kask export create`. Fresh snapshot.
- **Interrupted upload:** Re-run `kask export upload`. 73% of triples already merged (no-ops), 27% fill in. Converged.

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
- **Archive SQLCipher:** Key derived via Argon2id from user-provided passphrase → AES-256. Reuses `Database::open_impl` encryption path from `hkask-storage` (v0.28.0). Does NOT use server master passphrase — the user's key is independent.
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

kask export create [--passphrase <passphrase>]
    Generate encrypted sovereignty archive for the authenticated user.

kask export upload --server <url> [--archive <path>]
    Upload sovereignty archive to a new server for migration.

kask replicate rename <from> <to>
kask replicate merge --from <source> --into <target>
kask replicate delete <name>
    Manage replicants after migration.
```

**Note:** `kask backup` commands (snapshot, restore, list, prune, verify, config) remain for operational backup — see §4.6. The `download` operation is API-only (`GET /api/v1/export/download`) since the CLI runs on the server and the file is local. Scheduled auto-export is deferred to Phase 6 (§13).

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

**Status: Not yet implemented.** The following spans are defined in the plan but not yet added to the `CnsSpan` enum in `crates/hkask-types/src/cns.rs`:

```rust
CnsSpan::SessionOpen,      // { user_id, provider }
CnsSpan::SessionClose,     // { user_id, duration }
CnsSpan::BackupExport,     // { triple_count, bytes, duration }
CnsSpan::BackupAutoExport, // { webid, triple_count, bytes, duration }
CnsSpan::BackupUpload,     // { triple_count, bytes, duration }
CnsSpan::ReplicantMerge,   // { source, target, triple_count, duration }
```

Route handlers for auth, export, and replicant operations currently log via `tracing` but do not emit CNS spans. Wiring these spans is the top-priority remaining implementation task.

### 8.3 API Endpoints (New)

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/v1/auth/login?provider=github` | Initiate OAuth flow |
| `GET` | `/api/v1/auth/callback?provider=github&code=...` | OAuth callback |
| `GET` | `/api/v1/auth/session` | Return current session info |
| `POST` | `/api/v1/auth/logout` | Destroy session |
| `POST` | `/api/v1/export/create` | Generate and return encrypted sovereignty archive |
| `GET` | `/api/v1/export/download` | Download latest sovereignty archive (browser) |
| `POST` | `/api/v1/export/upload` | Upload sovereignty archive for migration |
| `POST` | `/api/v1/replicants/merge` | Merge two replicants |
| `POST` | `/api/v1/replicants/rename` | Rename a replicant |
| `DELETE` | `/api/v1/replicants/{name}` | Delete a replicant |

**Note:** Existing `/api/v1/backup/*` routes (operational GitCAS backup) remain unchanged in the `backup_router`. The export routes use `/api/v1/export/*` to avoid namespace collision.

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

- **No client binary.** The "client" is a browser tab rendering xterm.js.
- **No feature gating or Cargo features.** Single binary, all crates compiled.
- **No SyncPort trait.** No client-server sync protocol. Backup is a file export.
- **No client registration protocol.** Users authenticate via OAuth. Sessions are server-managed.
- **No CRDT pull/upload streaming protocol.** Backup is a file. CRDT idempotence provides fault tolerance for migration uploads.
- **No client-side encryption key management.** User provides passphrase at export time. Server never stores it.
- **No server-to-server protocol.** Migration is user-mediated via downloadable archive.
- **No conflict resolution UI.** Replicant merge is user-initiated, idempotent upsert.
- **No backup archive export pruning code.** The archive is a single snapshot — no versioned history to prune. Operational backup pruning (`BackupService::prune` with `RetentionPolicy`, via `BackupLoop`) is a separate system for git-based artifact versioning and is already implemented. The memory pipeline (consolidation, salience, condensation) handles live triple pruning independently of both.
- **No artifact replication (LORA, research files).** Out of scope. Backup covers triples only.
- **No SSH key setup required.** Browser terminal is the default. SSH is an optional power-user feature.
- **No terminal app to install.** Alacritty, WezTerm, etc. are user preference — hKask doesn't ship one.

---

## 11. Success Criteria

```
1. [Deploy]  kask init --profile server
             kask matrix deploy-sidecar --domain example.com
             -> Caddy serves HTTPS, Conduit responds on /_matrix/

2. [Login]   User visits https://example.com, clicks "Sign in with GitHub"
             -> OAuth callback succeeds, session cookie set
             -> redirected to /terminal, xterm.js loads
             -> WebSocket connects, kask repl prompt appears

3. [Export]  kask export create --passphrase "user-chosen"
             -> archive.db created, encrypted with passphrase
             -> CnsSpan::BackupExport emitted

4. [Migrate] kask export upload --server https://new-server.example
             -> MigrationReceipt.triple_count matches archive count
             -> replicants renamed on collision

5. [Merge]   kask replicate merge --from ada-migrated-xxx --into ada
             -> triples merged, source unchanged

6. [Multi]   Users A and B both signed in
             -> A cannot see B's triples, pods, or wallet
             -> B cannot see A's triples, pods, or wallet

7. [Zero]    No binary to download, no SSH key to generate, no terminal to install
             -> User only needs a browser
```

---

## 12. Open Questions

| # | Question | Resolution |
|---|----------|------------|
| Q1 | ~~Should auto-export archives be encrypted with the user's session key (server-side) or require a passphrase at download time?~~ | **Resolved:** Passphrase-at-download only. Session-key encryption would mean the server holds the key, contradicting §4.3 ("server never stores the user's backup password") and §6.1 ("Storage: User-provided passphrase at export time"). Auto-export archives are encrypted at rest with a key derived from the user's passphrase, provided at download time. The server stores only the encrypted blob. |
| Q2 | OAuth provider scope: GitHub only? GitHub + Google? | **Resolved:** GitHub first (developer audience). Google sign-in button is on the landing page but the callback handler only supports GitHub. The Google button will be removed until the callback handler is implemented. Revisit if demand exists. |
| Q3 | Should the backup include artifacts (LORA, research files, skill bundles) organized by registry in a zip? | Extends the backup format. Needs artifact store maturity first. |

---

## 13. Implementation Sequence

| Phase | Tasks | Depends On | Status |
|-------|-------|-----------|--------|
| **Phase 1 — OAuth** | `OAuthProvider`, OAuth config, `/auth/login` + `/auth/callback`, session cookie, `HumanUser.provider` fields | — | ✅ Implemented (GitHub complete; Google deferred per Q2) |
| **Phase 2 — Terminal** | `/api/v1/terminal/ws` WebSocket endpoint, PTY spawn + I/O pipe, static `/terminal` page with xterm.js | Phase 1 | ✅ Implemented |
| **Phase 3 — Export** | `BackupArchive` type, `kask export create`, CNS spans | Phase 1 | ⚠️ Partial — types + HTTP routes done; CLI command + CNS spans pending |
| **Phase 4 — Migration** | `kask export upload`, replicant rename/merge/delete, `MigrationReceipt`, auto-rename on collision | Phase 3 | ⚠️ Partial — types + HTTP routes done; CLI commands + CNS spans pending |
| **Phase 5 — Integration** | End-to-end: deploy → OAuth sign-in → terminal → export → upload to second server → merge → verify | Phase 4 | 🔴 Not started |
| **Phase 6 — Harden** | Interruption testing, multi-user isolation, backup auto-export tuning | Phase 5 | 🔴 Not started (deferred) |

---

## 14. Related Research and Past Plans

> **Incorporated from:** `plans/hetzner-blocking-issues.md`, `plans/hetzner-k3s-implementation-plan.md`, `plans/rjoule-cost-tracking-implementation.md`, `research/cloud-deployment-research-report.md`, `research/cloud-implementation-plans.md`

### 14.1 Hetzner k3s Deployment

Hetzner Cloud (CX22/CX32) + k3s cluster topology (3 master + 3 worker nodes) was evaluated as the production deployment target. Cilium CNI, Longhorn storage, cert-manager TLS. Blocking issues (boot volume encryption, firewalls, S3-compatible backup, PostgreSQL HA) confirmed available. Full implementation plan archived in `docs/archive/guides-2026-06-22/`.

### 14.2 Cloud Provider Comparison

Multi-provider evaluation: Hetzner, Fly.io, Railway, Render, DigitalOcean, AWS, GCP, Azure. Hetzner selected for cost-to-capability ratio. Key constraint: single binary deployment with SQLCipher as primary store.

### 14.3 rJoule Cost Tracking

Per-provider pricing tracking, energy consumption estimation, and cumulative cost accounting design. Deferred until multi-provider inference routing is production-ready.

---
