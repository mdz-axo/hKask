---
title: "Cloud Implementation Plans — hKask"
audience: [architects, developers]
last_updated: 2026-06-20
version: "0.30.0"
status: "Implementation Planning"
domain: "Deployment"
mds_categories: [lifecycle, composition]
depends_on: ["docs/research/cloud-deployment-research-report.md"]
gentle_lovelace_score: 92/100 (composite cosine distance: 0.158 — Excellent)
---

# hKask Cloud Implementation Plans

**Purpose:** Detailed implementation plans for two cloud providers (Fly.io, Hetzner Cloud), the `kask pod export-*` provider plug-in architecture, due diligence checklists, and critical-infrastructure partner considerations. Complements the [Cloud Deployment Research Report](./cloud-deployment-research-report.md).

**Status:** Pre-implementation planning. No code exists yet for any cloud export command.

---

## 1. Provider Plug-in Architecture

### 1.1 Design Principle

Per Dokkodo Precept 13 ("Do not be fond of material things") and Precept 20 ("Respect the gods and Buddhas, but do not rely on them"), the export commands must be provider-agnostic. The provider is a target, not a dependency.

```
kask pod export <provider> <pod-id> [--flags]
├── kask pod export fly <pod-id>     → fly.toml + deploy script
├── kask pod export k8s <pod-id>     → k8s manifests (StatefulSet, PVC, Service)
├── kask pod export runpod <pod-id>  → RunPod template
└── kask pod export docker <pod-id>  → standalone Dockerfile + compose
```

### 1.2 Crate Architecture

```
crates/hkask-cli/src/commands/pod/
├── mod.rs              # pod command entrypoint
├── export.rs           # export dispatch (match on provider enum)
├── export_fly.rs       # Fly.io export logic
├── export_k8s.rs       # K8s export logic
├── export_runpod.rs    # RunPod export logic
└── export_docker.rs    # standalone Docker export

crates/hkask-types/src/
└── cloud.rs            # CloudProvider enum, ExportConfig, ManifestFormat
```

### 1.3 Provider Trait (Rust)

```rust
/// Trait implemented by each cloud provider export module.
/// Following deep-module discipline: 4 public methods, ≤ 7 total surface.
#[async_trait]
pub trait CloudProvider {
    /// Provider identifier (e.g., "fly", "hetzner-k8s", "runpod")
    fn provider_id(&self) -> &'static str;

    /// Generate deployment manifests for a pod.
    /// Returns a map of filename → content suitable for writing to disk.
    fn generate_manifests(&self, pod: &AgentPod, config: &ExportConfig) -> Result<HashMap<String, String>>;

    /// Validate that the pod's configuration is compatible with this provider.
    fn validate_pod(&self, pod: &AgentPod) -> Result<Vec<ValidationWarning>>;

    /// Return provider-specific environment variables required at deploy time.
    fn required_env_vars(&self) -> Vec<EnvVarSpec>;
}

pub struct EnvVarSpec {
    pub name: String,
    pub description: String,
    pub secret: bool,  // should be stored as a secret, not plaintext
}
```

---

## 2. Implementation Plan: Fly.io

### 2.1 Provider Profile

| Attribute | Detail |
|-----------|--------|
| **Company** | Fly.io, Inc. |
| **Founded** | 2017 |
| **Infrastructure** | Own bare-metal servers in 35+ regions. Custom Rust hypervisor. Firecracker microVMs. |
| **API** | Machines REST API (OpenAPI 3.0). GraphQL for org/provisioning operations. |
| **Auth** | Macaroon tokens (since May 2025). Deploy tokens scoped to app or org. |
| **Key dependency** | Litestream (built and maintained by Fly.io's Ben Johnson). |

### 2.2 Architecture Diagram

Two Conduit deployment models are viable. **Model A (per-pod Conduit sidecar)** preserves OCAP isolation at the Matrix layer. **Model B (shared Conduit app)** is simpler but creates a shared dependency outside the pod boundary.

#### Model A: Per-Pod Conduit Sidecar (OCAP-Aligned)

```
+------------------------------------------------------------------+
|                      Fly.io Organization                          |
|                                                                   |
|  +------------------------------------------------------------+   |
|  |                 Fly App: hkask-pod-{id}                     |   |
|  |                                                             |   |
|  |  +------------------------------------------------------+  |   |
|  |  |             Fly Machine (Firecracker)                 |  |   |
|  |  |                                                       |  |   |
|  |  |  +----------+ +----------+ +----------------------+  |  |   |
|  |  |  |Litestream| | Conduit  | |     kask binary      |  |  |   |
|  |  |  | restore  | | sidecar  | |                      |  |  |   |
|  |  |  | (init)   | |          | |  kask serve          |  |  |   |
|  |  |  |          | | :8448    | |  --pod-id {id}       |  |  |   |
|  |  |  |          | | Matrix   | |  --data-dir /data    |  |  |   |
|  |  |  |          | | federation| |  --matrix-url        |  |  |   |
|  |  |  |          | | port     | |  http://localhost:8008|  | |   |
|  |  |  +----------+ +----+-----+ +----------+-----------+  |  |   |
|  |  |                    |                   |              |  |   |
|  |  |  +-----------------+-------------------+----------+   |  |   |
|  |  |  | Litestream      |  kask <-Matrix->  |          |   |  |   |
|  |  |  | replicate       |  Conduit          |          |   |  |   |
|  |  |  | -exec           |  OCAP-gated       |          |   |  |   |
|  |  |  | supervisord     |  A2A messages     |          |   |  |   |
|  |  |  +--------+--------+-------------------+----------+   |  |   |
|  |  |           |                            |              |  |   |
|  |  |  +--------+----------------------------+----------+   |  |   |
|  |  |  |              Fly Volume                         |   |  |   |
|  |  |  |  /data/kask.db        (SQLCipher, WAL)         |   |  |   |
|  |  |  |  /data/conduit.db     (Conduit homeserver DB)  |   |  |   |
|  |  |  |  1GB -> 10GB auto-expand                        |   |  |   |
|  |  |  +------------------------------------------------+   |  |   |
|  |  +------------------------------------------------------+  |   |
|  |                                                             |   |
|  |  auto_stop_machines: true    (scale-to-zero on idle)       |   |
|  |  auto_start_machines: true   (wake on HTTP or Matrix msg)  |   |
|  |  min_machines_running: 0                                   |   |
|  +------------------------------------------------------------+   |
|                         |                                          |
|         +---------------+---------------+--------------+          |
|         v               v               v              v          |
|  +------------+ +------------+ +------------+ +--------------+    |
|  |Tigris/     | |Fly Secrets | |Fly Metrics | |Matrix        |    |
|  |Backblaze B2| |(API keys,  | |(Prometheus)| |Federation    |    |
|  |(Litestream | | keystore,  | |            | |(Conduit      |    |
|  | replica)   | | matrix     | |            | | :8448)       |    |
|  |            | | signingKey)| |            | |              |    |
|  +------------+ +------------+ +------------+ +------+-------+    |
|                                                       |           |
|         Pod-to-pod A2A: Matrix federation over        |           |
|         Fly.io private WireGuard network              |           |
|         +---------------------------------------------+           |
|         v                                                         |
|  +----------------------------------------------------------+     |
|  |              Other hKask Pods (Fly Apps)                  |     |
|  |  +--------------+  +--------------+  +----------------+  |     |
|  |  | hkask-pod-2  |  | hkask-pod-3  |  | hkask-pod-N    |  |     |
|  |  | Conduit      |  | Conduit      |  | Conduit        |  |     |
|  |  | :8448        |  | :8448        |  | :8448          |  |     |
|  |  +--------------+  +--------------+  +----------------+  |     |
|  +----------------------------------------------------------+     |
+------------------------------------------------------------------+
```

#### Model B: Shared Conduit App

Pods share a single Conduit Fly App as their Matrix homeserver. `kask --matrix-url http://hkask-conduit.internal:8008`. Simpler to operate but Conduit becomes a shared dependency outside the OCAP perimeter.

#### Model Comparison

| Criterion | Model A (Per-Pod) | Model B (Shared) |
|-----------|-------------------|------------------|
| OCAP isolation | Pod owns its Matrix server | Shared server |
| Operational simplicity | N+1 Conduit instances | Single Conduit |
| Resource overhead | ~50MB RAM/pod | One for all pods |
| Federation resilience | Each pod federates independently | Single point of failure |
| Scale-to-zero | ❌ Conduit must stay warm for messages | Shared stays warm |
| P4.1 alignment | Messaging inside pod boundary | Messaging outside pod boundary |

**Recommendation:** Model A for production. Model B for initial deployment. Both use the same `kask --matrix-url` flag — switching is a configuration change.

### 2.3 Conduit Sidecar Configuration (Model A)

Conduit runs as a process managed by `supervisord` alongside kask and Litestream. It listens on `:8008` (client API, kask connects here) and `:8448` (federation, other pods connect here).

```yaml
# /etc/conduit/conduit.toml
global:
  server_name: "pod-{{ pod_id }}.hkask.local"
  address: "0.0.0.0"
  port: 8008
  federation:
    enabled: true
    address: "0.0.0.0"
    port: 8448
  database:
    backend: "sqlite"
    path: "/data/conduit.db"
  registration:
    enabled: false
  allow_federation:
    - "*.hkask.local"
```

Each pod gets a Matrix identity: `@pod-{pod_id}:pod-{pod_id}.hkask.local`. OCAP DelegationTokens are carried as custom Matrix event fields (`hkask.ocap_token`). Pods discover each other via Fly.io internal DNS (`<app-name>.internal`) on the WireGuard private network.

### 2.4 Dockerfile

```dockerfile
# Stage 1: Build kask
FROM rust:1.85-slim-bookworm AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release --bin kask

# Stage 2: Build Litestream
FROM golang:1.23-bookworm AS litestream-builder
RUN go install github.com/benbjohnson/litestream/cmd/litestream@v0.5.0

# Stage 3: Build Conduit (Matrix homeserver, Rust)
FROM rust:1.85-slim-bookworm AS conduit-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev libsqlite3-dev && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://gitlab.com/famedly/conduit.git /conduit
WORKDIR /conduit
RUN cargo build --release --bin conduit

# Stage 4: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kask /usr/local/bin/kask
COPY --from=litestream-builder /root/go/bin/litestream /usr/local/bin/litestream
COPY --from=conduit-builder /conduit/target/release/conduit /usr/local/bin/conduit

# Configuration templates
COPY deploy/fly/litestream.yml /etc/litestream.yml.template
COPY deploy/fly/conduit.toml /etc/conduit/conduit.toml.template
COPY deploy/fly/supervisord.conf /etc/supervisor/conf.d/hkask.conf

# Entrypoint script: render configs -> restore -> migrate -> supervisord
COPY deploy/fly/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME /data
EXPOSE 3000 8008 8448

ENV HKASK_DATA_DIR=/data
ENV LITESTREAM_CONFIG=/etc/litestream.yml

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
```

### 2.5 Entrypoint Script (with Conduit)

The entrypoint now uses `supervisord` to manage three long-running processes: kask, Litestream, and Conduit.

```bash
#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

# Render config templates from environment variables
envsubst < /etc/litestream.yml.template > /etc/litestream.yml
envsubst < /etc/conduit/conduit.toml.template > /etc/conduit/conduit.toml

# Restore kask database from S3 if no local copy exists
if [ ! -f "$DB_PATH" ]; then
    echo "No local database found. Attempting restore from Litestream replica..."
    litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH" || {
        echo "No replica found. Starting with fresh database."
    }
fi

# Run database migrations (idempotent)
kask migrate --data-dir "$DATA_DIR"

# Start supervisord which manages all three processes:
#   - conduit:  Matrix homeserver (Matrix federation preserved across restarts)
#   - litestream: WAL replication to S3
#   - kask: main application
# supervisord runs as PID 1; all child processes are monitored and restarted on failure
exec /usr/bin/supervisord -c /etc/supervisor/supervisord.conf
```

### 2.6 Supervisord Configuration

```ini
# /etc/supervisor/conf.d/hkask.conf
[supervisord]
nodaemon=true
logfile=/dev/stdout
logfile_maxbytes=0

[program:conduit]
command=/usr/local/bin/conduit
environment=CONDUIT_CONFIG="/etc/conduit/conduit.toml"
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0

[program:litestream]
command=/usr/local/bin/litestream replicate -config /etc/litestream.yml
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0

[program:kask]
command=/usr/local/bin/kask serve --data-dir /data
environment=POD_ID="%(ENV_POD_ID)s",HKASK_DATA_DIR="/data",HKASK_MATRIX_URL="http://localhost:8008"
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0
```

### 2.7 Litestream Configuration Template

```yaml
# /etc/litestream.yml.template — rendered by envsubst at container start
addr: ":9090"
sync-interval: 1s
snapshot-interval: 6h

dbs:
  - path: /data/kask.db
    replicas:
      - type: s3
        bucket: ${LITESTREAM_S3_BUCKET}
        path: pods/${POD_ID}/kask.db
        endpoint: ${LITESTREAM_S3_ENDPOINT}
        region: ${LITESTREAM_S3_REGION}
        access-key-id: ${LITESTREAM_ACCESS_KEY_ID}
        secret-access-key: ${LITESTREAM_SECRET_ACCESS_KEY}
        force-path-style: ${LITESTREAM_FORCE_PATH_STYLE:-false}
```

### 2.6 fly.toml (Generated by `kask pod export fly`)

```toml
app = "hkask-pod-{{ pod_id }}"
primary_region = "{{ primary_region }}"

[build]
  image = "{{ container_registry }}/hkask:kask-{{ version }}"

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 768

[mounts]
  source = "hkask_data"
  destination = "/data"
  initial_size = "1gb"
  auto_extend_size_increment = "1gb"
  auto_extend_size_limit = "10gb"

[[services]]
  protocol = "tcp"
  internal_port = 3000

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.ports]]
    port = 80
    handlers = ["http"]
    force_https = true

  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

# Matrix Federation service (Conduit :8448)
# Exposed publicly so other pods' Conduit instances can federate
# Note: this service does NOT auto-stop - Conduit must stay reachable
[[services]]
  protocol = "tcp"
  internal_port = 8448

  [[services.ports]]
    port = 8448
    handlers = ["tls"]

  auto_stop_machines = false

[experimental]
  auto_rollback = true

[deploy]
  release_command = "kask migrate --data-dir /data"

[env]
  HKASK_DATA_DIR = "/data"
  POD_ID = "{{ pod_id }}"
```

### 2.9 Fly Secrets (Generated, Never Committed)

```bash
# Generated by `kask pod export fly --secrets`
# These are set via `fly secrets set` or Machines API, never in fly.toml

fly secrets set \
  LITESTREAM_S3_BUCKET="hkask-pods-backup" \
  LITESTREAM_S3_ENDPOINT="https://fly.storage.tigris.dev" \
  LITESTREAM_S3_REGION="auto" \
  LITESTREAM_ACCESS_KEY_ID="tigris_xxx" \
  LITESTREAM_SECRET_ACCESS_KEY="xxx" \
  LITESTREAM_FORCE_PATH_STYLE="true" \
  POD_ID="pod_abc123" \
  HKASK_KEYSTORE_PASSPHRASE="xxx" \
  CONDUIT_MATRIX_SIGNING_KEY="ed25519_xxx"
```

> **`CONDUIT_MATRIX_SIGNING_KEY`** is the Ed25519 private key Conduit uses to sign Matrix federation events. It must be stable across pod restarts — if it changes, other pods will reject federation requests. Store it as a Fly Secret, not in the volume (volume restore from Litestream doesn't cover Conduit state).

### 2.10 Fly Machines API Integration (Rust)

The `kask pod activate` command will use the Machines API directly, not `flyctl`, to enable programmatic pod lifecycle management:

```rust
// crates/hkask-cli/src/commands/pod/export_fly.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};

const FLY_API_HOST: &str = "https://api.machines.dev";

pub struct FlyClient {
    client: Client,
    token: String,
    org_slug: String,
}

impl FlyClient {
    /// Create a new Fly.io API client.
    pub fn new(token: String, org_slug: String) -> Self {
        Self {
            client: Client::new(),
            token,
            org_slug,
        }
    }

    /// Create a Fly App for the pod.
    pub async fn create_app(&self, app_name: &str) -> Result<FlyApp> {
        let resp = self.client
            .post(format!("{}/apps", FLY_API_HOST))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({
                "app_name": app_name,
                "org_slug": self.org_slug,
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Create a Fly Volume for persistent SQLite storage.
    pub async fn create_volume(
        &self,
        app_name: &str,
        name: &str,
        region: &str,
        size_gb: u32,
    ) -> Result<FlyVolume> {
        let resp = self.client
            .post(format!("{}/apps/{}/volumes", FLY_API_HOST, app_name))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({
                "name": name,
                "region": region,
                "size_gb": size_gb,
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Create and start a Fly Machine for the pod.
    pub async fn create_machine(
        &self,
        app_name: &str,
        config: &MachineConfig,
    ) -> Result<FlyMachine> {
        let resp = self.client
            .post(format!("{}/apps/{}/machines", FLY_API_HOST, app_name))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(config)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Stop a Machine (suspend, not destroy — preserves volume).
    pub async fn stop_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        let resp = self.client
            .post(format!(
                "{}/apps/{}/machines/{}/stop",
                FLY_API_HOST, app_name, machine_id
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    /// Set secrets for an app.
    pub async fn set_secrets(&self, app_name: &str, secrets: &HashMap<String, String>) -> Result<()> {
        let resp = self.client
            .post(format!("{}/apps/{}/secrets", FLY_API_HOST, app_name))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({ "secrets": secrets }))
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }
}
```

### 2.11 Pod Lifecycle Mapping (Fly.io with Conduit)

| hKask Pod State | Fly.io Operation | Notes |
|-----------------|-----------------|-------|
| `Create` | `POST /apps` + `POST /apps/{app}/volumes` + `POST /apps/{app}/secrets` (signing key) | Volume created, Conduit signing key stored as secret |
| `Populate` | `POST /apps/{app}/secrets` | Remaining secrets (S3 creds, keystore passphrase) |
| `Register` | `POST /apps/{app}/machines` | Machine boots, entrypoint renders configs, supervisord starts conduit + litestream + kask |
| `Activate` | Machine running | Conduit federates with other pods; kask connects to localhost:8008 |
| `Deactivate` | `POST /apps/{app}/machines/{id}/stop` | Conduit gracefully shuts down federation; Litestream flushes WAL to S3 |
| `Destroy` | `DELETE /apps/{app}/machines/{id}` + `DELETE /apps/{app}/volumes/{id}` | Volume deleted; S3 Litestream replica remains for migration |

> **Scale-to-zero tradeoff:** Model A (per-pod Conduit) cannot scale to zero because Conduit must be reachable for inbound Matrix federation messages. The HTTP API service (port 3000) can auto-stop, but the Matrix federation service (port 8448) must remain running. This means idle pods still incur the Machine cost (~$1.94/mo). Model B (shared Conduit) avoids this — only the shared Conduit stays warm, and kask pods can scale to zero. Choose based on whether OCAP isolation or cost efficiency is the higher priority.

---

## 3. Implementation Plan: Hetzner Cloud + K3s

### 3.1 Provider Profile

| Attribute | Detail |
|-----------|--------|
| **Company** | Hetzner Online GmbH (German GmbH, privately held) |
| **Founded** | 1997 |
| **Infrastructure** | Own data centers in Falkenstein, Nuremberg, Helsinki. Partner DCs in Ashburn (VA), Hillsboro (OR), Singapore. |
| **API** | Hetzner Cloud API v1 at `api.hetzner.cloud/v1`. hcloud CLI (Go). Terraform provider. Python SDK. |
| **Certifications** | ISO/IEC 27001 (DE, FI DCs). BSI C5:2020 Type 2. KRITIS operator (German critical infrastructure). GDPR compliant. |
| **Key dependency** | Self-managed K3s (or Cloudfleet for managed K8s). Hetzner CSI driver for volumes. |

### 3.2 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Hetzner Cloud Project                     │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Private Network (10.0.0.0/16)         │   │
│  │                                                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │ K3s Server 1 │  │ K3s Server 2 │  │ K3s Server 3 │  │   │
│  │  │ (control     │  │ (control     │  │ (control     │  │   │
│  │  │  plane)     │  │  plane)     │  │  plane)     │  │   │
│  │  │ CX33: €6.49 │  │ CX33: €6.49 │  │ CX33: €6.49 │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  │                         │                             │   │
│  │  ┌──────────────────────────────────────────────────┐│   │
│  │  │              K3s Worker Nodes                     ││   │
│  │  │                                                  ││   │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐       ││   │
│  │  │  │Worker 1  │  │Worker 2  │  │Worker N  │       ││   │
│  │  │  │CX43      │  │CX43      │  │CX53      │       ││   │
│  │  │  │8 vCPU    │  │8 vCPU    │  │16 vCPU   │       ││   │
│  │  │  │16GB RAM  │  │16GB RAM  │  │32GB RAM  │       ││   │
│  │  │  │€12/mo    │  │€12/mo    │  │€29/mo    │       ││   │
│  │  │  │          │  │          │  │          │       ││   │
│  │  │  │ ┌──────┐ │  │ ┌──────┐ │  │ ┌──────┐ │       ││   │
│  │  │  │ │Pod 1 │ │  │ │Pod 3 │ │  │ │Pod N │ │       ││   │
│  │  │  │ │Pod 2 │ │  │ │Pod 4 │ │  │ │...   │ │       ││   │
│  │  │  │ └──────┘ │  │ └──────┘ │  │ └──────┘ │       ││   │
│  │  │  └──────────┘  └──────────┘  └──────────┘       ││   │
│  │  └──────────────────────────────────────────────────┘│   │
│  │                         │                             │   │
│  │  ┌──────────────────────┼──────────────────────────┐ │   │
│  │  │         Hetzner Cloud Resources                  │ │   │
│  │  │  ┌────────────┐  ┌──────────┐  ┌────────────┐  │ │   │
│  │  │  │ CSI Volumes│  │  Network │  │  Firewall  │  │ │   │
│  │  │  │ (per pod) │  │  Load    │  │  (DDoS)    │  │ │   │
│  │  │  │ €0.044/GB │  │  Balancer│  │  Free      │  │ │   │
│  │  │  │            │  │  €5.89/mo│  │            │  │ │   │
│  │  │  └────────────┘  └──────────┘  └────────────┘  │ │   │
│  │  └─────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         External Services (outside Hetzner)           │   │
│  │  ┌────────────────────┐  ┌────────────────────────┐  │   │
│  │  │ Backblaze B2       │  │ Cloudfleet (optional)  │  │   │
│  │  │ (Litestream        │  │ Managed K8s control     │  │   │
│  │  │  replica target)   │  │ plane, 99.95% SLA)     │  │   │
│  │  └────────────────────┘  └────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 K3s Cluster Bootstrap

Two paths: self-managed K3s (cheapest) or Cloudfleet (managed, SLA-backed).

#### Path A: Self-Managed K3s (via hetzner-k3s)

```bash
# One-time cluster creation (~2-3 minutes)
hetzner-k3s create \
  --name hkask-prod \
  --location nbg1 \
  --masters 3 \
  --master-type cx33 \
  --workers 3 \
  --worker-type cx43 \
  --network-zone eu-central \
  --autoscaling-enabled

# hetzner-k3s automatically:
# - Creates private network (10.0.0.0/16)
# - Installs K3s on all nodes
# - Deploys Hetzner CCM (load balancer integration)
# - Deploys Hetzner CSI (volume provisioning)
# - Deploys Cluster Autoscaler
# - Outputs kubeconfig
```

#### Path B: Cloudfleet Managed K8s

```bash
# Cloudfleet handles control plane. You provide Hetzner API token.
# No kubeconfig management — Cloudfleet provides API access.

# Cluster created via Cloudfleet UI or API
# Pro plan: €69/mo + €4.95/vCPU (first 24 vCPUs free)
# Enterprise: €5,000/mo minimum, 1-hour support SLA, private control plane
```

### 3.4 K8s Manifests (Generated by `kask pod export k8s`)

#### Namespace + NetworkPolicy (per-pod isolation)

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: hkask-pod-{{ pod_id }}
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: pod-isolation
  namespace: hkask-pod-{{ pod_id }}
spec:
  podSelector: {}
  policyTypes: [Ingress, Egress]
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: hkask-ingress
      ports:
        - port: 3000
          protocol: TCP
  egress:
    - to:
        - namespaceSelector: {}  # allow same-namespace traffic
    - to:  # allow external API access (inference providers)
        - ipBlock:
            cidr: 0.0.0.0/0
            except: [10.0.0.0/8]  # except internal traffic (handled above)
      ports:
        - port: 443
          protocol: TCP
        - port: 80
          protocol: TCP
```

#### StatefulSet (per-pod deployment)

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: kask
  namespace: hkask-pod-{{ pod_id }}
spec:
  serviceName: kask
  replicas: 1
  selector:
    matchLabels:
      app: kask
  template:
    metadata:
      labels:
        app: kask
        pod-id: "{{ pod_id }}"
    spec:
      # Init container: restore SQLite from Litestream if no local DB
      initContainers:
        - name: litestream-restore
          image: litestream/litestream:0.5.0
          args:
            - restore
            - -if-db-not-exists
            - -if-replica-exists
            - /data/kask.db
          envFrom:
            - secretRef:
                name: litestream-s3
          volumeMounts:
            - name: data
              mountPath: /data
            - name: litestream-config
              mountPath: /etc/litestream.yml
              subPath: litestream.yml

      containers:
        # Main application container
        - name: kask
          image: {{ container_registry }}/hkask:kask-{{ version }}
          args:
            - serve
            - --data-dir
            - /data
            - --pod-id
            - "{{ pod_id }}"
          ports:
            - containerPort: 3000
              protocol: TCP
          envFrom:
            - secretRef:
                name: kask-secrets
          volumeMounts:
            - name: data
              mountPath: /data
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
          readinessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 15
            periodSeconds: 20

        # Litestream sidecar: continuous WAL replication
        - name: litestream
          image: litestream/litestream:0.5.0
          args:
            - replicate
          envFrom:
            - secretRef:
                name: litestream-s3
          volumeMounts:
            - name: data
              mountPath: /data
            - name: litestream-config
              mountPath: /etc/litestream.yml
              subPath: litestream.yml
          resources:
            requests:
              cpu: 10m
              memory: 32Mi
            limits:
              cpu: 100m
              memory: 64Mi

      volumes:
        - name: litestream-config
          configMap:
            name: litestream-config

  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        storageClassName: hcloud-volumes
        accessModes: [ReadWriteOnce]
        resources:
          requests:
            storage: {{ volume_size_gb }}Gi
```

#### ConfigMap + Secrets

```yaml
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: litestream-config
  namespace: hkask-pod-{{ pod_id }}
data:
  litestream.yml: |
    addr: ":9090"
    sync-interval: 1s
    snapshot-interval: 6h
    dbs:
      - path: /data/kask.db
        replicas:
          - type: s3
            bucket: ${LITESTREAM_S3_BUCKET}
            path: pods/{{ pod_id }}/kask.db
            endpoint: ${LITESTREAM_S3_ENDPOINT}
            region: ${LITESTREAM_S3_REGION}
            access-key-id: ${LITESTREAM_ACCESS_KEY_ID}
            secret-access-key: ${LITESTREAM_SECRET_ACCESS_KEY}
---
apiVersion: v1
kind: Secret
metadata:
  name: litestream-s3
  namespace: hkask-pod-{{ pod_id }}
stringData:
  LITESTREAM_S3_BUCKET: "hkask-pods-backup"
  LITESTREAM_S3_ENDPOINT: "https://s3.us-west-000.backblazeb2.com"
  LITESTREAM_S3_REGION: "us-west-000"
  LITESTREAM_ACCESS_KEY_ID: "<generated>"
  LITESTREAM_SECRET_ACCESS_KEY: "<generated>"
---
apiVersion: v1
kind: Secret
metadata:
  name: kask-secrets
  namespace: hkask-pod-{{ pod_id }}
stringData:
  POD_ID: "{{ pod_id }}"
  HKASK_DATA_DIR: "/data"
  HKASK_KEYSTORE_PASSPHRASE: "<generated>"
```

#### HPA (Horizontal Pod Autoscaler — CNS-driven)

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: kask-hpa
  namespace: hkask-pod-{{ pod_id }}
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: kask
  minReplicas: 1
  maxReplicas: {{ max_replicas }}
  metrics:
    # Scale on HTTP request rate
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    # Scale on CNS variety deficit (custom metric — future)
    # - type: Pods
    #   pods:
    #     metric:
    #       name: hkask_cns_variety_deficit
    #     target:
    #       type: AverageValue
    #       averageValue: 100
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Percent
          value: 50
          periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
        - type: Percent
          value: 100
          periodSeconds: 30
```

### 3.5 Pod Lifecycle Mapping (Hetzner K3s)

| hKask Pod State | K8s Operation | kubectl / API Call |
|-----------------|---------------|-------------------|
| `Create` | `kubectl create namespace` + apply manifests | `POST /api/v1/namespaces` |
| `Populate` | `kubectl create secret` + configmap | `POST /api/v1/namespaces/{ns}/secrets` |
| `Register` | `kubectl apply -f statefulset.yaml` | `POST /apis/apps/v1/namespaces/{ns}/statefulsets` |
| `Activate` | Pod auto-starts via StatefulSet | (automatic) |
| `Deactivate` | `kubectl scale statefulset kask --replicas=0` | `PATCH .../statefulsets/kask/scale` |
| `Destroy` | `kubectl delete namespace` | `DELETE /api/v1/namespaces/{ns}` |

### 3.6 Hetzner Cloud API Integration (Rust)

```rust
// crates/hkask-cli/src/commands/pod/export_k8s.rs

const HCLOUD_API: &str = "https://api.hetzner.cloud/v1";

pub struct HetznerClient {
    client: Client,
    token: String,
}

impl HetznerClient {
    /// Create a new server instance.
    pub async fn create_server(
        &self,
        name: &str,
        server_type: &str,
        image: &str,
        location: &str,
        ssh_keys: &[String],
    ) -> Result<HetznerServer> {
        let resp = self.client
            .post(format!("{}/servers", HCLOUD_API))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({
                "name": name,
                "server_type": server_type,
                "image": image,
                "location": location,
                "ssh_keys": ssh_keys,
                "start_after_create": true,
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Create a volume for persistent storage.
    pub async fn create_volume(
        &self,
        name: &str,
        size_gb: u32,
        location: &str,
    ) -> Result<HetznerVolume> {
        let resp = self.client
            .post(format!("{}/volumes", HCLOUD_API))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({
                "name": name,
                "size": size_gb,
                "location": location,
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Attach a volume to a server.
    pub async fn attach_volume(
        &self,
        volume_id: u64,
        server_id: u64,
    ) -> Result<()> {
        let resp = self.client
            .post(format!("{}/volumes/{}/actions/attach", HCLOUD_API, volume_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({ "server": server_id }))
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    /// List all servers in the project.
    pub async fn list_servers(&self) -> Result<Vec<HetznerServer>> {
        let resp = self.client
            .get(format!("{}/servers", HCLOUD_API))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        Ok(serde_json::from_value(body["servers"].clone())?)
    }
}
```

---

## 4. Due Diligence Checklists

### 4.1 Fly.io Due Diligence

| # | Check | Status | Evidence |
|---|-------|--------|----------|
| 1 | **Data residency.** Where are pod databases physically stored? | ✅ Known | Fly volumes reside in the region selected. S3 replicas in Tigris/Backblaze B2 (configurable). |
| 2 | **Encryption at rest.** Are volumes encrypted? | ⚠️ Verify | Fly.io volumes are on NVMe — confirm if encryption-at-rest is default or opt-in. SQLCipher provides application-level encryption regardless. |
| 3 | **SOC2/HIPAA.** Do they have compliance attestations? | ✅ Confirmed | SOC2 Type 2 attested. HIPAA-ready (BAA available, $99/mo). |
| 4 | **GDPR compliance.** EU data protection? | ⚠️ Partial | Fly.io has EU regions but is a US company. Data Processing Agreement (DPA) needed. |
| 5 | **SLA coverage.** What's covered and what's excluded? | ✅ Known | 99.9% uptime SLA (Enterprise only, $2,500/mo min). Excludes: free tier, scheduled maintenance, customer-caused issues. |
| 6 | **Support responsiveness.** What are the actual response times? | ✅ Known | Public dashboard: 99.5% SLA compliance, 55min median first response. Enterprise: 15min urgent, 4hr normal. |
| 7 | **Vendor lock-in.** How hard is migration? | ✅ Low | Standard Docker containers. `litestream restore` to any SQLite. `fly.toml` is the only proprietary config. |
| 8 | **GPU deprecation.** Impact on roadmap? | ✅ Assessed | GPUs unavailable after Aug 2026. Core kask doesn't need GPU. Inference dispatches externally. Acceptable. |
| 9 | **Pricing stability.** History of price changes? | ⚠️ Monitor | Removed free tier (Oct 2024). Added volume snapshot billing (Jan 2026). Changed inter-region networking billing (Feb 2026). |
| 10 | **Financial viability.** Risk of shutdown/acquisition? | ⚠️ Monitor | Privately held. VC-funded. Revenue not public. Litestream team at Fly.io — if Fly.io fails, Litestream is open-source (MIT). |
| 11 | **Litestream + SQLCipher compatibility.** Tested? | ❌ Untested | Litestream replicates WAL at file level — should work with SQLCipher. Must test: restore, WAL integrity, encryption key rotation. |
| 12 | **API stability.** Machines API versioning? | ✅ Known | OpenAPI 3.0 spec at docs.machines.dev. No formal versioning but documented. Macaroon tokens stable since May 2025. |

### 4.2 Hetzner Cloud Due Diligence

| # | Check | Status | Evidence |
|---|-------|--------|----------|
| 1 | **Data residency.** Where are pod databases physically stored? | ✅ Known | Customer selects location (DE, FI, US, SG). DE/FI data centers ISO 27001 certified. Volumes stay in selected location. |
| 2 | **Encryption at rest.** Are volumes encrypted? | ⚠️ Verify | Hetzner volumes are not encrypted by default. Must use Hetzner CSI encryption feature or rely on SQLCipher. Confirm CSI encryption support. |
| 3 | **ISO/SOC/C5.** Compliance certifications? | ✅ Confirmed | ISO/IEC 27001 (DE, FI DCs). BSI C5:2020 Type 2. KRITIS operator. GDPR-compliant (German GmbH). |
| 4 | **SLA coverage.** What's covered and what's excluded? | ⚠️ Narrow | 99.9% per Cloud Server. Excludes: load balancers, firewalls, snapshots, backups, network. Credit is per-instance (very small — €0.87 for 185min outage on €0.37/hr server). |
| 5 | **Support model.** 24/7? Phone? | ✅ Known | 24/7 data center staff for hardware. Email support during business hours for technical issues. No phone support for Cloud (only dedicated servers with password). |
| 6 | **Vendor lock-in.** How hard is migration? | ✅ Low | Standard VPS. Standard K8s. `litestream restore` to any SQLite. K8s manifests are portable. |
| 7 | **GPU availability.** Options for inference? | ❌ None | No GPU instances. Inference must dispatch to external providers (RunPod, Together, etc.). |
| 8 | **Scale-to-zero.** Idle cost? | ❌ None | Billing continues while server object exists, even when powered off. Must delete server to stop charges. |
| 9 | **Pricing stability.** History of price changes? | ⚠️ Monitor | +30-37% increase (April 2026). +33% for some types (June 2026). Upward trend. |
| 10 | **Operational burden.** K8s expertise required? | ⚠️ High | Self-managed K3s requires K8s operational knowledge. Cloudfleet mitigates (€69/mo + €4.95/vCPU). |
| 11 | **Network reliability.** DDoS, peering? | ✅ Known | Free DDoS protection. 99.9% network availability per GTC. 20TB free egress. |
| 12 | **Litestream S3 target.** What object storage? | ⚠️ External | Hetzner Object Storage (S3-compatible) or Backblaze B2. Not managed by Hetzner within the Cloud SLA. |

---

## 5. Critical Infrastructure Partner Considerations

### 5.1 What "Critical Infrastructure" Means for hKask

For partners running hKask as critical infrastructure, the following are non-negotiable:

1. **Data sovereignty.** Per P1 (User Sovereignty): users own their data. The cloud provider must not have access to decrypted pod databases. SQLCipher ensures this at the application layer.
2. **OCAP boundary integrity.** Per P4.1: pod boundary IS the OCAP enforcement perimeter. The cloud provider must not provide a side-channel across pod boundaries. Hardware isolation (Firecracker) is stronger than container isolation (K8s namespaces).
3. **No ambient authority.** Per P4: no admin bypass. The cloud provider's support team must not have the ability to access pod data or impersonate pods.
4. **Portability.** Per P1: data portability is a first-class guarantee. Pods must be migratable between providers via Litestream restore.

### 5.2 Fly.io as Critical Infrastructure Partner

**Strengths:**
- SOC2 Type 2 + HIPAA-ready attestation provides third-party verification of security controls
- Custom Rust hypervisor reduces supply chain attack surface (no QEMU/KVM CVEs)
- Memory-safe stack (Rust) for the hypervisor layer
- Hardware virtualization prevents cross-tenant data leaks at the hypervisor level
- Macaroon tokens with fine-grained scoping (app-level, org-level)
- 24/7 enterprise support with 15-minute urgent response
- Public support metrics dashboard (transparency)
- Deploy tokens enable CI/CD without human credential exposure

**Weaknesses:**
- US company — GDPR data transfer implications (Standard Contractual Clauses needed)
- Smaller company than AWS/GCP — financial viability risk over 5-10 year horizon
- No BSI C5 certification (relevant for German/EU regulated industries)
- SLA only on Enterprise plan ($2,500/mo minimum)
- GPU deprecation signals willingness to cut product lines
- 2023 reliability issues (acknowledged by CEO) — recovery to mid-tier PaaS reliability in 2026

**Recommendation for critical infrastructure:** Suitable with Enterprise plan + SLA. Mitigate financial risk by maintaining Hetzner as secondary provider with `kask pod export-k8s` as migration path.

### 5.3 Hetzner Cloud as Critical Infrastructure Partner

**Strengths:**
- KRITIS operator (German critical infrastructure) — legally mandated security standards
- BSI C5:2020 Type 2 attestation — gold standard for German public sector
- ISO/IEC 27001 certified data centers (DE, FI)
- GDPR compliance by jurisdiction (German GmbH, EU data centers)
- 24/7 on-site data center staff for hardware incidents
- Privately held, profitable — no VC pressure, no acquisition risk from public markets
- 20+ year operating history (founded 1997)
- Free DDoS protection, redundant network infrastructure

**Weaknesses:**
- No managed Kubernetes — operational burden falls on hKask team (or Cloudfleet partner)
- SLA covers individual Cloud Servers only, not the platform — credit is per-instance and minimal
- No phone support for Cloud products (email only during business hours)
- No GPU instances for inference workloads
- No scale-to-zero — idle pods incur full cost
- Historical price increases (30-37% in 2026) — not promotional, structural, but trending up
- Volume encryption requires CSI configuration — not default
- Object Storage (for Litestream) is a separate product with its own SLA terms

**Recommendation for critical infrastructure:** Excellent for EU-regulated workloads (GDPR, BSI C5). Pair with Cloudfleet Enterprise for managed K8s + 99.95% SLA + 1-hour support. Mitigate GPU gap with RunPod for inference. Mitigate operational burden with Cloudfleet.

### 5.4 Multi-Provider Resilience Strategy

```
                    ┌──────────────────────────┐
                    │    kask pod export-*     │
                    │    (provider-agnostic)   │
                    └──────────┬───────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            ▼                  ▼                  ▼
    ┌───────────────┐  ┌───────────────┐  ┌───────────────┐
    │   Fly.io      │  │  Hetzner+K3s  │  │   RunPod      │
    │  (primary)    │  │  (secondary)  │  │   (GPU)       │
    │               │  │               │  │               │
    │ Orchestrator  │  │ Orchestrator  │  │ Inference     │
    │ + Litestream  │  │ + Litestream  │  │ + Training    │
    │ + Volume      │  │ + CSI Volume  │  │ + NW Volume   │
    └───────┬───────┘  └───────┬───────┘  └───────┬───────┘
            │                  │                  │
            └──────────────────┼──────────────────┘
                               │
                    ┌──────────▼───────────┐
                    │   Litestream S3      │
                    │   (Backblaze B2 /    │
                    │    Cloudflare R2)    │
                    │                      │
                    │   Provider-agnostic  │
                    │   Database replicas  │
                    │   Sub-second RPO     │
                    └──────────────────────┘
```

**Migration procedure (Fly.io → Hetzner):**
1. `kask pod export-k8s <pod-id>` — generates K8s manifests
2. `kubectl apply -f manifests/` — deploys to Hetzner K3s
3. Litestream init container restores database from same S3 bucket
4. DNS cutover to Hetzner load balancer
5. `fly machines destroy <pod-id>` — decommission Fly.io Machine
6. Database WAL continues replicating to same S3 bucket — no data migration needed

---

## 6. Implementation Sequence

### Phase 1: Foundation (P0)
- [ ] `Dockerfile` (multi-stage Rust + Litestream + Conduit)
- [ ] `entrypoint.sh` (render configs → restore → supervisord)
- [ ] `supervisord.conf` (conduit, litestream, kask)
- [ ] `litestream.yml.template` (S3 configuration)
- [ ] `conduit.toml.template` (Matrix homeserver configuration)
- [ ] CI/CD pipeline to build and push container image
- [ ] SQLCipher + Litestream compatibility test

### Phase 2: Fly.io (P1)
- [ ] `CloudProvider` trait in `hkask-types`
- [ ] `FlyClient` in `hkask-cli` (Machines API)
- [ ] `kask pod export fly <pod-id>` command
- [ ] `fly.toml` Jinja2 template (HTTP :3000 + Matrix :8448 services)
- [ ] `kask pod activate` → `fly machines start`
- [ ] `kask pod deactivate` → `fly machines stop`
- [ ] Conduit federation test: pod-1 ↔ pod-2 Matrix messaging
- [ ] Integration test: create → activate → deactivate → destroy on Fly.io

### Phase 3: Hetzner K3s (P2)
- [ ] `HetznerClient` in `hkask-cli` (Hetzner Cloud API)
- [ ] `kask pod export k8s <pod-id>` command
- [ ] K8s manifest templates (StatefulSet, PVC, NetworkPolicy, HPA, ConfigMap, Secrets)
- [ ] K3s bootstrap script / Cloudfleet integration
- [ ] `kask pod activate` → `kubectl apply`
- [ ] `kask pod deactivate` → `kubectl scale --replicas=0`
- [ ] Integration test: full lifecycle on Hetzner K3s

### Phase 4: RunPod (P3)
- [ ] `kask pod export runpod <pod-id>` command
- [ ] RunPod CPU pod template
- [ ] RunPod serverless GPU endpoint template
- [ ] Integration test: orchestrate inference via RunPod serverless

### Phase 5: Multi-Provider (P4)
- [ ] Cross-provider migration test (Fly.io → Hetzner via Litestream)
- [ ] CNS span for cloud provider health
- [ ] Provider health dashboard (which pods on which provider)
- [ ] Automated failover (if Fly.io region down → spawn on Hetzner)

---

## 7. References

- [Cloud Deployment Research Report](./cloud-deployment-research-report.md)
- [Fly.io Machines API](https://fly.io/docs/machines/api/)
- [Fly.io Enterprise](https://fly.io/enterprise/)
- [Fly.io SLA](https://fly.io/legal/sla-uptime/)
- [Fly.io Support](https://fly.io/support/)
- [Hetzner Cloud API](https://docs.hetzner.cloud/)
- [Hetzner SLA](https://docs.hetzner.com/general/company-and-policy/slas-cloud/)
- [Hetzner Security (TOMs)](https://docs.hetzner.com/general/security-and-identify/technical-and-organizational-measures/)
- [Cloudfleet Managed Kubernetes](https://cloudfleet.ai/lp/managed-hetzner-kubernetes/)
- [Litestream Documentation](https://litestream.io/)
- [Litestream VFS (Fly.io blog)](https://fly.io/blog/litestream-vfs/)
- [hetzner-k3s](https://vitobotta.github.io/hetzner-k3s/)
