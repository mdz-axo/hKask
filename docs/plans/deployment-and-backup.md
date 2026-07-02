---
title: "hKask Deployment & Multi-User Plan"
audience: [architects, developers]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active — Phases 1-5 implemented, Phase 6 deferred"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
anchored_on: [PRINCIPLES.md §0, P1, P2, P3, P9, P12]
reviewed_via: [pragmatic-laziness, essentialist, grill-me, coding-guidelines]
---

# hKask Deployment & Multi-User Plan

**Purpose:** Define the cloud server deployment model, multi-user OAuth sign-in, terminal session provisioning, the backup-as-portable-sovereignty-archive model, and the server migration procedure.

**Decision:** There is no client — no binary, no install, no SSH setup. Users visit a website, sign in with GitHub or Google, and get a terminal. The "client" is a browser tab running xterm.js connected to the server via WebSocket. The server spawns `kask repl` on a PTY and pipes I/O. That is the entire product surface.

**Status:** Phases 1–5 implemented. Phase 6 (hardening) deferred. See §15 for per-phase status.

---

## 1. Architecture — Single Node, Multi-User

### 1.1 Deployment Model

One binary (`kask`), one server, many users. Each user gets a terminal session scoped to their WebID.

Two deployment paths are supported: **bare-metal** (binary + Docker sidecars, see admin install guide) and **K8s pod** (single container with supervisord managing kask + Litestream + Conduit, see `deploy/` directory).

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
- **Inference:** quota-tracked per user (CNS gas budget, wallet balance).
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
kask init
# Prompts for:
#   - Server master passphrase (Argon2id → HKDF key derivation)
#   - Data directory (default: /var/lib/hkask/)
#   - Domain name (for Caddy TLS + OAuth redirect URIs)
#   - OAuth provider credentials (GitHub client ID + secret)
```

**What this creates:**
- `~/.config/hkask/` — server config, keystore version file, OAuth provider config
- `/var/lib/hkask/` — SQLCipher database (all user data), git backup repository
- OS keychain entry: `master-passphrase` (master passphrase)
- OAuth credentials stored in OS keychain: `oauth-github-credentials`, `oauth-google-credentials`

**After init, deploy sidecars:**
```bash
kask matrix deploy-sidecar --domain my-server.hkask.example
cd ~/.config/hkask/sidecar && docker compose up -d
```

### 3.2 User Onboarding

No install. No SSH keys. User visits `https://my-server.hkask.example`, clicks "Sign in with GitHub," and gets a terminal. First sign-in provisions their WebID, default replicant, and wallet.

### 3.3 Systemd Unit File

> **Incorporated from:** `docs/guides/DEPLOYMENT.md`

```ini
[Unit]
Description=hKask Server
After=network.target docker.service

[Service]
Type=simple
User=hkask
Group=hkask
ExecStart=/usr/local/bin/kask daemon --host 0.0.0.0 --port 8080
Environment=DI_API_KEY=${DI_API_KEY}
Environment=HKASK_DB_PATH=/var/lib/hkask/hkask.db
Environment=RUST_LOG=hkask=info
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable: `sudo systemctl daemon-reload && sudo systemctl enable --now hkask`.

### 3.4 Daemon & Socket Architecture

> **Incorporated from:** `docs/guides/OPERATIONS_RUNBOOK.md`

The `kask` daemon listens on `~/.config/hkask/daemon.sock` (Unix socket) and handles: agent authentication/session management, MCP server role assignment, OCAP capability verification, dual memory encoding (episodic + semantic). All CLI/API/MCP servers connect via this socket. `kask daemon start|stop|status` manages the lifecycle. Stale socket removal (`rm ~/.config/hkask/daemon.sock`) before restart resolves port-binding failures.

### 3.5 Dockerfile Reference

> **Incorporated from:** `docs/guides/DEPLOYMENT.md`

```dockerfile
FROM rust:1.91-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin kask

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kask /usr/local/bin/
RUN useradd -m hkask
USER hkask
ENV HKASK_DB_PATH=/home/hkask/hkask.db
EXPOSE 8080
CMD ["kask", "daemon", "--host", "0.0.0.0", "--port", "8080"]
```

### 3.6 Kubernetes StatefulSet Model

> **Incorporated from:** `docs/guides/kubernetes-primer.md`

hKask uses **StatefulSets** (not Deployments) because each pod needs stable identity, its own persistent volume, and ordered startup. The pod contains 3 containers: `kask` binary (:3000), `litestream` sidecar (WAL replication to S3), `conduit` Matrix homeserver (:8008). All three share a PVC at `/data/`.

**Key k8s resources per pod:**

| Resource | Purpose |
|----------|---------|
| **StatefulSet** | Stable pod identity (`kask-0`), ordered startup |
| **volumeClaimTemplate** | Auto-creates PVC per pod (10Gi, `hcloud-volumes` on Hetzner) |
| **Init containers** | `litestream-restore` (restore DB from S3 if absent), `kask-migrate` (idempotent schema migrations) |
| **NetworkPolicy** | Ingress only from ingress controller on :3000; egress to internet (:443/:80) |
| **HPA** | CPU-based scaling 1–3 replicas (target 70% utilization, 5-min stabilization) |
| **ConfigMap** | `litestream.yml` + `conduit.toml` |
| **Secret** | Object storage credentials, keystore passphrase |

**PVC survival:** Deleting the pod preserves the PVC. Deleting the StatefulSet preserves PVCs unless explicitly deleted. "Deactivate" (= scale to zero) never loses data.

**Pod lifecycle commands:**
```bash
kask pod create <name>        # creates pod DB locally
kask pod export-k8s <id>      # generates 6 YAML manifests
kubectl apply -f k8s-manifests/
kask pod activate <id>        # kubectl apply (best-effort)
kask pod deactivate <id>      # scale statefulset to 0
kubectl delete namespace agent-pod-<id>  # destroy everything
```

**Useful kubectl commands:**
```bash
kubectl get namespaces -l app=hkask
kubectl get pods -n agent-pod-alice -w
kubectl logs -n agent-pod-alice statefulset/kask -c kask
kubectl exec -n agent-pod-alice statefulset/kask -c litestream -- litestream generations /data/kask.db
kubectl describe pod -n agent-pod-alice kask-0
kubectl get hpa -n agent-pod-alice
kubectl get events -n agent-pod-alice --sort-by='.lastTimestamp'
kubectl port-forward -n agent-pod-alice statefulset/kask 3000:3000
```

### 3.7 Cloud Gateway — Remote IDE Access (mTLS)

> **Incorporated from:** `docs/guides/admin-setup-guide.md`

The `hkask-mcp-cloud-gateway` provides secure remote access to the daemon for IDE clients outside the cluster. Uses mutual TLS (mTLS) for transport identity + Ed25519-signed DelegationTokens for per-request authorization.

```
IDE Client ──[mTLS 1.3]──▶ Cloud Gateway
  ├── Client cert CN = replicant name
  └── DelegationToken per request
                             │
                   ┌─────────┼─────────┐
                   │ Gate 1  │ Gate 2  │ Gate 3
                   │ CN→WebID│ Tool    │ Ed25519
                   │ match?  │ match?  │ verify?
                   └─────────┼─────────┘
                             ▼
                      DaemonHandler
```

**Security properties:** No API keys (identity is cryptographic). No ambient authority (every tool call requires scoped, expiring token). Token theft insufficient without matching client certificate.

**Provisioning (summary):**
```bash
# Generate CA + server + client certs (ED25519)
# Deploy as K8s service with TLS secrets
kubectl create secret tls gateway-tls-cert --cert=server.crt --key=server.key
kask pod export-k8s gateway && kubectl apply -f k8s-manifests/

# Issue scoped tokens
kask token issue --replicant alice \
  --capabilities curator:health,curator:cns \
  --ttl 168h
```

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
| **Operational Backup** | Git via `GixCasAdapter` (one repo per pod, directory tracking) | pod.db is SQLCipher-encrypted | Server disaster recovery — pod directory versioning, agent revert by date, automated 24h snapshots | `kask backup`, `kask backup restore <pod> --date` |

The operational backup is implemented in `hkask-storage` (absorbed into `hkask-storage`, v0.31.0):

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

## 5. Encryption Model

### 5.1 Two Layers, Two Keys

| Layer | Key | Purpose |
|-------|-----|---------|
| Transport | Caddy TLS (Let's Encrypt) | HTTPS for all browser/API traffic. SSH for terminal sessions. |
| Storage | User-provided passphrase at export time | Encrypts the backup archive (SQLCipher). Server never stores this. |

### 5.2 Server-Side Encryption

- **TripleStore:** SQLCipher-encrypted, key derived from server master passphrase (Argon2id → AES-256).
- **Git backup:** Already implemented — AES-256-GCM encrypted blobs before CAS storage.
- **Archive SQLCipher:** Key derived via Argon2id from user-provided passphrase → AES-256. Reuses `Database::open_impl` encryption path from `hkask-storage` (v0.28.0). Does NOT use server master passphrase — the user's key is independent.
- **PII:** Encrypted in `UserStore` with per-user PII key.

### 5.3 Key Rotation

Server key rotation follows the existing `hkask-keystore::master_key` pattern: increment `key_version`, old-version keys remain derivable. Zero new crypto code.

---

## 6. CLI Command Surface

```
kask init
    Initialize server, configure domain, OAuth providers, master passphrase.

kask matrix deploy-sidecar --domain <domain> [--with-web-client]
    Generate Caddy + Conduit docker-compose and config files.

kask matrix status-sidecar
    Health check Caddy, Conduit, and Hydrogen containers.

kask export create [--passphrase <passphrase>]
    Generate encrypted sovereignty archive for the authenticated user.

kask export upload --archive <path> --passphrase <passphrase>
    Restore a sovereignty archive (simple idempotent insert).

kask replicant rename --from <name> --to <name>
kask replicant delete <name>
    Manage replicants.
```

**Note:** `kask backup` commands (snapshot, restore, list, prune, verify, config) remain for operational backup — see §4.6. The `download` operation is API-only (`GET /api/v1/export/download`) since the CLI runs on the server and the file is local. Scheduled auto-export is deferred to Phase 6 (§15).

---

## 7. Operational Health Checks

> **Incorporated from:** `docs/guides/DEPLOYMENT.md`, `docs/guides/OPERATIONS_RUNBOOK.md`

### 7.1 API Health Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/cns/health` | GET | CNS health status |
| `/api/sovereignty/status` | GET | User sovereignty status |
| `/api/templates` | GET | Template registry status |

**Expected CNS health response:**
```json
{"overall_deficit": 0, "critical_count": 0, "warning_count": 0, "healthy": true}
```

### 7.2 CLI Health Commands

```bash
kask cns health              # CNS health
kask sovereignty status      # Sovereignty status
kask daemon status           # Daemon running? (daemon.sock present, PID file exists)
```

**Build-level health checks:**
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
bash docs/ci/check-links.sh
kask sovereignty verify
```

### 7.3 Metrics Thresholds

| Metric | Alert Threshold | Action |
|--------|-----------------|--------|
| CNS variety deficit | >100 | Investigate tool usage patterns |
| Algedonic alerts | >5/hour | Escalate to on-call |
| API latency p99 | >500ms | Scale horizontally (HPA) |
| Database size | >10GB | Archive old data |

**Monitoring via CNS (no visual dashboards):**
```bash
journalctl -u hkask -p err --no-pager          # View recent errors
journalctl -u hkask | grep "ALGEDONIC ALERT"   # CNS alerts
journalctl -u hkask | grep "variety"            # Variety counters
```

All observability is programmatic — query CNS spans via `hkask-cns` crate APIs. Algedonic alerts escalate to Curator/human when variety deficit >100.

---

## 8. Type Summary

### 8.1 New Types

| Type | Crate | Fields / Variants |
|------|-------|-------------------|
| `OAuthProvider` | `hkask-api` | `GitHub`, `Google` |
| `OAuthConfig` | `hkask-api` | `client_id: String`, `client_secret: SecretRef`, `redirect_uri: String` |
| `OAuthUserProfile` | `hkask-api` | `provider: OAuthProvider`, `provider_user_id: String`, `email: String`, `display_name: String` |
| `BackupArchive` | `hkask-storage` | Wraps `Database` (SQLCipher) — methods: `create(user_passphrase, triples)`, `open(user_passphrase)`, `metadata()`, `restore_into()` |
| `MigrationReceipt` | `hkask-storage` | `triple_count: u64` |

### 8.2 CNS Span Additions

**Status: Implemented.** The following spans are added to `CnsSpan` and wired into route handlers:

```rust
CnsSpan::SessionOpen,      // { user_id, provider } — emitted on OAuth callback
CnsSpan::SessionClose,     // { user_id, duration } — emitted on logout
CnsSpan::BackupExport,     // { triple_count, bytes, duration } — emitted on export create
CnsSpan::BackupAutoExport, // { webid, triple_count, bytes, duration } — deferred (Phase 6)
CnsSpan::BackupUpload,     // { triple_count, bytes, duration } — emitted on export upload
```

### 8.3 API Endpoints (New)

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/v1/auth/login?provider=github` | Initiate OAuth flow |
| `GET` | `/api/v1/auth/callback?provider=github&code=...` | OAuth callback |
| `GET` | `/api/v1/auth/session` | Return current session info |
| `POST` | `/api/v1/auth/logout` | Destroy session |
| `POST` | `/api/v1/export/create` | Generate and return encrypted sovereignty archive |
| `GET` | `/api/v1/export/download` | Download latest sovereignty archive (browser) |
| `POST` | `/api/v1/export/upload` | Restore sovereignty archive (idempotent insert) |
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
| Git backup (`BackupService`) | Server-side operational backup (complementary) | `hkask-services-core`, `hkask-storage` |
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
- **No backup archive export pruning code.** The archive is a single snapshot — no versioned history to prune. Operational backup pruning (`BackupService::prune` with `RetentionPolicy`, via `BackupLoop`) is a separate system for git-based artifact versioning and is already implemented. The memory pipeline (consolidation, salience, condensation) handles live triple pruning independently of both.
- **No artifact replication (LORA, research files).** Out of scope. Backup covers triples only.
- **No SSH key setup required.** Browser terminal is the default. SSH is an optional power-user feature.
- **No terminal app to install.** Alacritty, WezTerm, etc. are user preference — hKask doesn't ship one.

---

## 11. QA Pipeline

> **Incorporated from:** `docs/guides/OPERATIONS_RUNBOOK.md`

### 11.1 Fuzz Testing

```bash
# Property-based fuzzing (every CI push)
cargo test -p hkask-types-fuzz -p hkask-cns-fuzz -p hkask-inference-fuzz \
           -p hkask-wallet-fuzz -p hkask-storage-fuzz -p hkask-templates-fuzz \
           -p hkask-memory-fuzz -p hkask-services-core-fuzz -p hkask-improv-fuzz

# Coverage-guided fuzzing (nightly)
cargo +nightly bolero test -p hkask-types-fuzz fuzz_cns_span_parse_never_panics -T 60s -e libfuzzer

# Mutation testing (measures test suite quality)
cargo mutants -p hkask-types --timeout 120
```

### 11.2 Triage & Repair

```bash
# Triage bolero failures with LLM classifier
export DI_API_KEY="your-key"
cargo test -p hkask-types-fuzz 2>&1 | kask qa triage

# Suggest fuzz targets from surviving mutants
cargo mutants -p hkask-types --timeout 120 2>&1 | grep "Uncaught" | kask qa suggest-fuzz
```

| Command | Output | Action |
|---------|--------|--------|
| `kask qa triage` | "No bolero failures detected" | System healthy |
| `kask qa triage` | "HIGH confidence: ..." | Check for auto-repair PR |
| `kask qa triage` | "LOW confidence: ..." | Open investigation issue |
| `kask qa suggest-fuzz` | "→ [suggestion]" | Consider adding suggested fuzz target |
| `cargo mutants` | "Uncaught mutants in ..." | Test gap — add test or fuzz target |

---

## 12. Success Criteria

```
1. [Deploy]  kask init
             kask matrix deploy-sidecar --domain example.com
             -> Caddy serves HTTPS, Conduit responds on /_matrix/

2. [Login]   User visits https://example.com, clicks "Sign in with GitHub"
             -> OAuth callback succeeds, session cookie set
             -> redirected to /terminal, xterm.js loads
             -> WebSocket connects, kask repl prompt appears

3. [Export]  kask export create --passphrase "user-chosen"
             -> archive.db created, encrypted with passphrase
             -> CnsSpan::BackupExport emitted

4. [Restore] kask export upload --archive archive.db --passphrase "user-chosen"
             -> MigrationReceipt.triple_count matches archive count

5. [Rename] kask replicant rename --from old-name --to new-name
             -> replicant renamed

6. [Delete]  kask replicant delete old-name
             -> replicant deleted

7. [Multi]   Users A and B both signed in
             -> A cannot see B's triples, pods, or wallet
             -> B cannot see A's triples, pods, or wallet

7. [Zero]    No binary to download, no SSH key to generate, no terminal to install
             -> User only needs a browser
```

---

## 13. Open Questions

| # | Question | Resolution |
|---|----------|------------|
| Q1 | ~~Should auto-export archives be encrypted with the user's session key (server-side) or require a passphrase at download time?~~ | **Resolved:** Passphrase-at-download only. Session-key encryption would mean the server holds the key, contradicting §4.3 ("server never stores the user's backup password") and §5.1 ("Storage: User-provided passphrase at export time"). Auto-export archives are encrypted at rest with a key derived from the user's passphrase, provided at download time. The server stores only the encrypted blob. |
| Q2 | OAuth provider scope: GitHub only? GitHub + Google? | **Resolved:** GitHub first (developer audience). Google sign-in button is on the landing page but the callback handler only supports GitHub. The Google button will be removed until the callback handler is implemented. Revisit if demand exists. |
| Q3 | Should the backup include artifacts (LORA, research files, skill bundles) organized by registry in a zip? | Extends the backup format. Needs artifact store maturity first. |

---

## 14. Troubleshooting

> **Incorporated from:** `docs/guides/DEPLOYMENT.md`, `docs/guides/OPERATIONS_RUNBOOK.md`, `docs/guides/admin-install.md`

### 14.1 Common Issues

| Issue | Cause | Resolution |
|-------|-------|------------|
| `Provider X is not available` | API key not set | Set `DI_API_KEY` or `OR_API_KEY` |
| `Inference error: error sending request` | Provider unreachable | Verify provider URL and network connectivity |
| `Database locked` | Concurrent access | Ensure single writer; use WAL mode |
| `Template not found` | Registry empty | Register templates: `kask template register` |
| `Capability denied` | Missing/invalid token | Grant capability: `kask bot grant` |
| `Chat response slow` | High inference latency | Check provider load; reduce `max_tokens` |
| `WebSocket disconnected` | Session expired | Re-authenticate via OAuth sign-in |
| Daemon won't start | Stale socket | `rm ~/.config/hkask/daemon.sock` then restart |

### 14.2 Deployment-Specific

**Connection refused :443:** Bare-metal → check Caddy (`docker ps \| grep caddy`), DNS (`dig`), firewall (ports 80/443). K8s → check ingress (`kubectl -n hkask get ingress`), cert-manager (`kubectl get certificaterequests -A`).

**OAuth callback failed:** Verify callback URL matches GitHub OAuth App exactly: `https://hkask.example.com/api/v1/auth/callback?provider=github`. Check `HKASK_DOMAIN` (bare-metal) or `configmap.yaml` domain value (K8s).

**Database errors:** Bare-metal → check `/var/lib/hkask/` exists and is writable. K8s → check PVC bound (`kubectl -n hkask get pvc`), check pod logs for Litestream restore errors.

**Pod won't start (k8s):**
```bash
kubectl -n hkask describe pod -l app=hkask
kubectl -n hkask logs -l app=hkask --tail=50
```
Common causes: image pull failure (check ghcr.io access), PVC not bound, secret/configmap missing.

**Sidecar health check fails (bare-metal):** `cd ~/.config/hkask/sidecar && docker compose logs`. Conduit may take 15–30s to initialize DB on first start.

### 14.3 Debug Mode

```bash
export RUST_LOG=debug
kask cns health --verbose

# Test inference provider connectivity
curl -s https://api.deepinfra.com/v1/openai/chat/completions \
  -H "Authorization: Bearer $DI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "meta-llama/Llama-3.3-70B-Instruct", "messages": [{"role": "user", "content": "test"}]}'
```

---

## 15. Implementation Sequence

| Phase | Tasks | Depends On | Status |
|-------|-------|-----------|--------|
| **Phase 1 — OAuth** | `OAuthProvider`, OAuth config, `/auth/login` + `/auth/callback`, session cookie, `HumanUser.provider` fields | — | ✅ Implemented (GitHub complete; Google deferred per Q2) |
| **Phase 2 — Terminal** | `/api/v1/terminal/ws` WebSocket endpoint, PTY spawn + I/O pipe, static `/terminal` page with xterm.js | Phase 1 | ✅ Implemented |
| **Phase 3 — Export** | `BackupArchive` type, `kask export create`, CNS spans | Phase 1 | ✅ Implemented (types, HTTP routes, CLI commands, CNS spans done) |
| **Phase 4 — Upload & Replicants** | `kask export upload`, replicant rename/delete, `MigrationReceipt` | Phase 3 | ✅ Implemented (types, HTTP routes, CLI commands done) |
| **Phase 5 — Integration** | End-to-end: deploy → OAuth sign-in → terminal → export → upload → rename → verify. Includes 9 HTTP integration tests (in-memory server, health endpoint, auth-gating, CNS health), K8s manifest hardening (NetworkPolicies, PDB, init containers, readiness probes), and Dockerfile optimization. | Phase 4 | ✅ Implemented (2026-07-01) |
| **Phase 6 — Harden** | Interruption testing, multi-user isolation stress tests, backup auto-export tuning, health endpoint with Matrix connectivity check. K8s readiness probe uses `/health` endpoint (DB + Conduit checks). | Phase 5 | 🔴 Deferred |

---

## 16. Related Research and Past Plans

> **Incorporated from:** `plans/hetzner-blocking-issues.md`, `plans/hetzner-k3s-implementation-plan.md`, `plans/rjoule-cost-tracking-implementation.md`, `research/cloud-deployment-research-report.md`, `research/cloud-implementation-plans.md`

### 16.1 Hetzner k3s Deployment

> **Incorporated from:** `docs/guides/kubernetes-primer.md`

Hetzner Cloud (CX22/CX32) + k3s cluster topology (3 master + 3 worker nodes) was evaluated as the production deployment target. Cilium CNI, Longhorn storage, cert-manager TLS. Blocking issues (boot volume encryption, firewalls, S3-compatible backup, PostgreSQL HA) confirmed available. Full implementation plan archived in `docs/archive/guides-2026-06-22/`.

**Provisioning flow via `hetzner-k3s`:**
```bash
hetzner-k3s create \
  --name prod-cluster \
  --location nbg1 \
  --masters 3 --master-type cx33 \
  --workers 3 --worker-type cx43
```

This single command provisions 6 servers and installs: private network (10.0.0.0/16), K3s on each node, Hetzner Cloud Controller Manager (enables Load Balancer creation), Hetzner CSI driver (NVMe block volumes, `hcloud-volumes` storage class), and Cluster Autoscaler. Total time: 2–3 minutes. Alternative: [Cloudfleet](https://cloudfleet.ai/) managed control plane (free tier up to 24 vCPUs).

**Key Hetzner specifics:**
- **CSI:** NVMe SSD volumes pinned to server location — can't move between Falkenstein and Nuremberg without snapshot/restore.
- **CCM:** Provisions cloud Load Balancers (EUR 5.89/month) when K8s Services of type LoadBalancer are created.
- **Object Storage:** S3-compatible, path-style addressing (`https://nbg1.your-objectstorage.com/bucket/object`), EUR 5/TB/month, 1TB free egress.
- **Network:** 20TB free egress per server/month — significant cost advantage for inference API calls.

**Decision guides:** Managed K8s (GKE/EKS/AKS) reduces operational burden at higher cost. Self-managed k3s on any VPS (DigitalOcean, Linode, Vultr) is viable if `hcloud-volumes` CSI driver is replaced with local-path or Longhorn. Minimum requirements: RWX or RWO persistent volumes, LoadBalancer Services, 2+ vCPUs, 4 GB RAM. Not viable: AWS Lambda, Cloud Run (no persistent volumes, no StatefulSet support).

### 16.2 Cloud Provider Comparison

Multi-provider evaluation: Hetzner, Fly.io, Railway, Render, DigitalOcean, AWS, GCP, Azure. Hetzner selected for cost-to-capability ratio. Key constraint: single binary deployment with SQLCipher as primary store.

### 16.3 rJoule Cost Tracking

Per-provider pricing tracking, energy consumption estimation, and cumulative cost accounting design. Deferred until multi-provider inference routing is production-ready.

---
