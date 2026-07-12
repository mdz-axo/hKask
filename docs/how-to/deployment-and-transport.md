---
title: "Deployment and Transport"
audience: [operators, developers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle, trust]
---

# Deployment and Transport

Deploy hKask on Kubernetes with the Conduit Matrix homeserver sidecar, configure Matrix transport for agent-to-agent communication, and manage backup and restore operations. These three operational concerns are tightly coupled: the K8s deployment includes the Matrix sidecar, and Litestream provides continuous backup within the same deployment.

---

## Kubernetes Deployment

### Architecture Overview

The deployment consists of two Pods in separate namespaces:

```
Your Domain (hkask.yourdomain.com)
        │
        ▼
┌──────────────────────────────────────────┐
│  Ingress (nginx)                          │
│  /         → kask (port 3000)            │
│  /_matrix  → conduit (port 8008)         │
└──────────┬───────────────────────────────┘
           │
    ┌──────┴──────┐
    ▼             ▼
┌─────────┐  ┌──────────┐
│  kask   │  │ conduit  │
│  Pod    │  │  Pod     │
│ [kask]  │  │[conduit] │
│[litestr]│  │          │
│ /data   │  │ /data    │
│  PVC    │  │  PVC     │
└────┬────┘  └──────────┘
     │
     ▼
┌────────────────────────────┐
│  S3 Object Storage          │
│  Litestream streams WAL     │
│  Restores on pod restart    │
└────────────────────────────┘

Namespace: hkask       Namespace: hkask-conduit
```

### The Conduit Sidecar

Conduit is a lightweight Matrix homeserver deployed as a separate Pod in the `hkask-conduit` namespace. It provides:

- **Agent-to-agent (A2A) communication**: Replicants register as Matrix users and communicate through rooms.
- **7R7 listener integration**: The `SevenR7Listener` polls Matrix rooms and emits CNS observation spans.
- **Thread-based attention**: Agents monitor threads via watchlists; the Curator decides escalation.

Conduit runs as its own Deployment because it has a different lifecycle from kask — a Conduit crash does not restart the main application.

### Litestream Sidecar

Litestream runs as a sidecar container in the kask Pod, sharing the `/data` PersistentVolume. It continuously streams SQLite WAL (write-ahead log) changes to S3-compatible object storage. On pod restart, an **init container** runs `litestream restore` before kask starts, ensuring the database is fully restored.

Configure these in `deploy/k8s/configmap.yaml`:
- `litestream-bucket`: S3 bucket name
- `litestream-endpoint`: S3 endpoint URL
- `litestream-region`: S3 region
- `litestream-force-path-style`: `"true"` or `"false"`

And in `deploy/k8s/secret.yaml`:
- `litestream-access-key-id`: S3 access key
- `litestream-secret-access-key`: S3 secret key

The master passphrase for SQLCipher encryption goes in `deploy/k8s/secret.yaml` as `master-passphrase`.

### Namespace Isolation

Two namespaces provide security boundaries:
- **`hkask`**: kask Deployment, ConfigMap, Secret, PVC, Service, Ingress
- **`hkask-conduit`**: Conduit Deployment, Service, NetworkPolicy

NetworkPolicies restrict cross-namespace traffic. If Conduit is compromised, it cannot access kask's Secrets.

### Pod Startup Sequence

1. Init container runs `litestream restore` to pull the latest database from S3
2. Litestream sidecar starts streaming WAL changes to S3
3. kask container starts, opens the restored database, and begins serving

### Key Operational Commands

```bash
# View pods
kubectl -n hkask get pods
kubectl -n hkask-conduit get pods

# View logs
kubectl -n hkask logs deploy/kask
kubectl -n hkask logs deploy/kask -c litestream

# Restart
kubectl -n hkask rollout restart deploy/kask

# Check resource usage
kubectl -n hkask top pods

# Shell into the container
kubectl -n hkask exec -it deploy/kask -- /bin/sh

# Verify backups
kubectl -n hkask exec deploy/kask -c litestream -- litestream snapshots /data/kask.db
```

### Deployment Files

The full deployment in `deploy/k8s/` includes 18 YAML files: `namespace.yaml`, `secret.yaml`, `configmap.yaml`, `pvc.yaml`, `deployment.yaml`, `service.yaml`, `ingress.yaml`, `entrypoint.sh`, `conduit/*`, `conduit-external-service.yaml`, `networkpolicy.yaml` (both namespaces), and `pdb.yaml`.

For the full step-by-step walkthrough (including Hetzner setup, K3s installation, DNS, and TLS), see `docs/plans/k8s-admin-guide.md`.

### Pod Export to K8s

You can export an agent pod as K8s manifests directly from the CLI:

```bash
kask pod export-k8s <pod_id> [--volume-size-gb 10] [--max-replicas 3] [--output ./k8s-manifests]
```

This generates K8s manifests tailored for Hetzner K3s deployment. You can also export a pod as a container build context:

```bash
kask pod export-container <pod_id> [--output ./pod-build]
```

---

## Matrix Transport Setup

Configure hKask to communicate via the Matrix protocol for agent-to-agent (A2A) messaging.

### Prerequisites

A running Matrix homeserver (e.g., Conduit, Synapse), or a Matrix.org account. The `matrix` feature must be enabled (it is on by default in `hkask-communication`).

### Step 1: Enable the Matrix Feature

```bash
cargo build --features matrix
```

If you disabled it previously, re-enable it in your `.cargo/config.toml` or pass `--features matrix` to build commands.

### Step 2: Configure Matrix Credentials

Set these environment variables:

```bash
export HKASK_MATRIX_HOMESERVER="https://matrix.example.com"
export HKASK_MATRIX_USERNAME="@my-agent:example.com"
export HKASK_MATRIX_PASSWORD="your-password"
```

Alternatively, add them to `~/.hkask/settings.yaml`:

```yaml
matrix:
  homeserver: "https://matrix.example.com"
  username: "@my-agent:example.com"
  password: "your-password"
```

### Step 3: Deploy the Matrix Sidecar

Deploy a Conduit sidecar locally (for non-K8s deployments):

```bash
kask matrix deploy-sidecar --domain "example.com" [--with-web-client] [--output ./conduit-manifests]
```

### Step 4: Register Your Agent

Agents are registered on the Matrix homeserver via the CLI. The `register-agent` subcommand creates a Matrix account for the agent:

```bash
kask matrix register-agent "my-agent" --homeserver "https://matrix.example.com"
```

To create a human user account instead, use:

```bash
kask matrix register-user "my-username" --homeserver "https://matrix.example.com"
```

### Step 5: Verify Registration

```bash
kask matrix status-sidecar
```

Expected output shows:
- Docker container health (Conduit, Caddy)
- API reachability
- Database status

### Step 6: Test A2A Message Delivery

A2A messaging is performed through the REPL, not the CLI. Start a chat session and use the `/msg` slash command:

```bash
kask chat
```

Inside the REPL:

```
/msg <room_id> "Hello from hKask"
```

See [Agents and Pods](agents-and-pods.md) for the full `/matrix` and `/msg` slash command reference.

### Step 7: Monitor Message Flow

Matrix transport events emit CNS spans. Subscribe to the communication namespace:

```bash
kask cns subscribe --agent curator --spans cns.communication
```

### Troubleshooting

| Issue | Likely Cause | Fix |
|-------|-------------|-----|
| `Connection refused` | Homeserver unreachable | Verify URL and network access |
| `Authentication failed` | Invalid credentials | Check username/password |
| `Room not found` | Agent not registered | Run `kask matrix register-agent` |
| `Feature not enabled` | `matrix` feature disabled | Rebuild with `--features matrix` |

### Notes

- Matrix transport requires the `hkask-communication` crate with the `matrix` feature enabled.
- Full MatrixTransport integration tests require a running Conduit homeserver (Docker sidecar). Unit tests for AgentRegistry (record, resolve, deregister, monitor, watchers) run without a homeserver.
- The 7R7 listener processes incoming messages on a dedicated Matrix room listener thread.

---

## Backup and Restore

hKask uses SQLCipher-encrypted SQLite databases with Litestream for continuous backup to S3-compatible object storage. Backups are managed at two levels: Litestream continuous WAL streaming (Kubernetes deployment) and CLI on-demand snapshots.

Backups are stored at `~/.config/hkask/backups/` in encrypted SQLCipher format (`.db` files).

### CLI Backup Commands

The `kask backup` subcommands:

```bash
# Create a point-in-time snapshot of all tracked storage types
kask backup snapshot [--scope <scope>]

# List all stored snapshots with timestamps, artifact counts, and trigger types
kask backup list [--limit <n>]

# Restore the database from a snapshot (destructive — replaces current state)
kask backup restore [--pod <pod>] [--date <date>] [--commit <commit>]

# Verify that stored snapshots are not corrupted
kask backup verify

# Show backup configuration and status
kask backup status
```

### Backup Configuration

The `BackupDataBridge` in `crates/hkask-tui/src/bridges/backup.rs` exposes configuration fields:

- **Auto-Snapshot**: Enable/disable automatic snapshots on a schedule
- **Verify After Snapshot**: Run integrity verification after each snapshot
- **Encryption**: Enable/disable encryption of backup files (enabled by default with SQLCipher)
- **Tracked Types**: Number of storage artifact types included in backups
- **Retention**: Daily snapshot count and weekly snapshot count

### Kubernetes Litestream Backup

In the K8s deployment, Litestream runs as a sidecar in the kask Pod. Verify backups:

```bash
# Check Litestream snapshots
kubectl -n hkask exec deploy/kask -c litestream -- litestream snapshots /data/kask.db

# Check Litestream replication status
kubectl -n hkask logs deploy/kask -c litestream | grep "replicating"
```

Litestream configuration is in `deploy/k8s/configmap.yaml` (bucket, endpoint, region) and `deploy/k8s/secret.yaml` (access key, secret key).

### Backing Up the Keystore

The hKask keystore (`crates/hkask-keystore/`) stores cryptographic material. Back it up separately:

```bash
# The keystore is at ~/.config/hkask/keystore/
cp -r ~/.config/hkask/keystore/ ~/backups/hkask-keystore-$(date +%Y%m%d)/
```

The keystore path is configurable via environment:

```bash
export HKASK_KEYSTORE_PATH="/secure/path/keystore"
```

### Disaster Recovery

To fully restore from backup:

1. **Restore the database** from the most recent snapshot or Litestream S3 backup
2. **Restore the keystore** from your separate keystore backup
3. **Verify integrity**: Run `kask backup verify`
4. **Start kask**: The init container (K8s) or manual `litestream restore` (bare metal) ensures the database is complete before kask starts

The Litestream init container (`litestream restore`) in the K8s deployment runs before kask starts, guaranteeing the database is on disk when the application opens it.

---

## Related

- [Install and Configure hKask](install-and-configure.md) — Build and initial setup
- [Agents and Pods](agents-and-pods.md) — Pod export to K8s manifests
- [Sovereignty and Observability](sovereignty-and-observability.md) — CNS monitoring for deployment health