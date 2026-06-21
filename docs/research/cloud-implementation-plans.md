---
title: "Cloud Implementation Plans — hKask"
audience: [architects, developers]
last_updated: 2026-06-20
version: "0.30.0"
status: "Implementation Planning"
domain: "Deployment"
mds_categories: [lifecycle, composition]
depends_on: ["docs/research/cloud-deployment-research-report.md"]
---

# hKask Cloud Implementation Plans

**Purpose:** Detailed implementation plan for Hetzner Cloud + K3s deployment, the `kask pod export-*` provider plug-in architecture, due diligence checklists, and critical-infrastructure partner considerations. Complements the [Cloud Deployment Research Report](./cloud-deployment-research-report.md).

**Status:** Pre-implementation planning. K8s manifest generation exists. K3s cluster bootstrap and integration testing blocked on Hetzner cluster access.

**Note:** Fly.io was previously evaluated and found architecturally incompatible (container isolation prevents Curator file access to other pods). All fly.io code removed as of v0.30.0.

---

## 1. Provider Plug-in Architecture

### 1.1 Design Principle

Per Dokkodo Precept 13 ("Do not be fond of material things") and Precept 20 ("Respect the gods and Buddhas, but do not rely on them"), the export commands must be provider-agnostic. The provider is a target, not a dependency.

```
kask pod export <provider> <pod-id> [--flags]
├── kask pod export-k8s <pod-id>     → k8s manifests (StatefulSet, PVC, Service)
├── kask pod export-runpod <pod-id>  → RunPod template
└── kask pod export-docker <pod-id>  → standalone Dockerfile + compose
```

### 1.2 Crate Architecture

```
crates/hkask-cli/src/commands/pod/
├── mod.rs              # pod command entrypoint
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
    /// Provider identifier (e.g., "hetzner-k8s", "runpod")
    fn provider_id(&self) -> &'static str;

    /// Generate deployment manifests for a pod.
    fn generate_manifests(&self, pod: &AgentPod, config: &ExportConfig) -> Result<HashMap<String, String>>;

    /// Validate that the pod's configuration is compatible with this provider.
    fn validate_pod(&self, pod: &AgentPod) -> Result<Vec<ValidationWarning>>;

    /// Return provider-specific environment variables required at deploy time.
    fn required_env_vars(&self) -> Vec<EnvVarSpec>;
}

pub struct EnvVarSpec {
    pub name: String,
    pub description: String,
    pub secret: bool,
}
```

---

## 2. Implementation Plan: Hetzner Cloud + K3s

### 2.1 Provider Profile

| Attribute | Detail |
|-----------|--------|
| **Company** | Hetzner Online GmbH |
| **Founded** | 1997 |
| **Infrastructure** | Own data centers in 6 regions (DE, FI, US, SG). Self-managed K3s on VPS instances. |
| **API** | REST API for Cloud management. Standard kubectl for K8s. S3-compatible API for Object Storage. |
| **Auth** | API tokens (read/write scoped). kubeconfig for K8s. S3 access/secret keys for Object Storage. |
| **Key dependency** | Litestream (MIT-licensed; provider-agnostic — works with any S3-protocol object storage: Backblaze B2, Hetzner OS, Cloudflare R2). |

### 2.2 Architecture Diagram

```
+-----------------------------------------------------------------+
|                     Hetzner Cloud Project                         |
|                                                                   |
|  +-----------------------------------------------------------+   |
|  |                   K3s Cluster (VPS)                         |   |
|  |                                                             |   |
|  |  +-----------------------------------------------------+   |   |
|  |  |  Namespace: hkask-pod-curator                        |   |   |
|  |  |  +-----------------------------------------------+  |   |   |
|  |  |  |  Pod: curator (StatefulSet)                    |  |   |   |
|  |  |  |  kask serve --pod-id curator                   |  |   |   |
|  |  |  |  SemanticIndex owner                           |  |   |   |
|  |  |  |  /data/curator.db                              |  |   |   |
|  |  |  +-----------------------------------------------+  |   |   |
|  |  +-----------------------------------------------------+   |   |
|  |                                                             |   |
|  |  +-----------------------------------------------------+   |   |
|  |  |  Namespace: hkask-pod-{id} (per replicant)           |   |   |
|  |  |                                                       |   |   |
|  |  |  +-----------------------------------------------+  |   |   |
|  |  |  |              Pod (StatefulSet)                  |  |   |   |
|  |  |  |                                                |  |   |   |
|  |  |  |  +-----------+ +-----------+ +-------------+  |  |   |   |
|  |  |  |  |Litestream | | Conduit   | | kask binary |  |  |   |   |
|  |  |  |  | sidecar   | | sidecar   | |             |  |  |   |   |
|  |  |  |  | streams   | | Matrix   | | kask serve  |  |  |   |   |
|  |  |  |  | WAL to OS | | :8008     | | --pod-id    |  |  |   |   |
|  |  |  |  +-----+-----+ +-----------+ +-------------+  |  |   |   |
|  |  |  |        |                                        |  |   |   |
|  |  |  |  +-----v------------------------------------+  |  |   |   |
|  |  |  |  |         Hetzner CSI Volume                |  |  |   |   |
|  |  |  |  |  /data/kask.db      SQLCipher-encrypted  |  |  |   |   |
|  |  |  |  |  /data/conduit.db   Conduit SQLite       |  |  |   |   |
|  |  |  |  |  EUR 0.044/GB/mo                            |  |  |   |   |
|  |  |  |  +-------------------------------------------+  |  |   |   |
|  |  |  +-----------------------------------------------+  |   |   |
|  |  |                                                       |   |   |
|  |  |  NetworkPolicy: per-namespace isolation               |   |   |
|  |  |  HPA: CPU + CNS variety metrics (min=1, max=N)       |   |   |
|  |  +-----------------------------------------------------+   |   |
|  |                                                             |   |
|  |  +-----------------------------------------------------+   |   |
|  |  |  Hetzner Load Balancer → Ingress → cert-manager TLS |   |   |
|  |  |  Free DDoS protection. 20TB free egress.            |   |   |
|  |  +-----------------------------------------------------+   |   |
|  +-----------------------------------------------------------+   |
|                          |                                        |
|                   HTTPS  |  (Litestream WAL streaming)            |
|                          v                                        |
|  +-----------------------------------------------------------+   |
|  |              Object Storage (S3-compatible)                  |   |
|  |  pods/{pod_id}/kask.db     ← encrypted SQLCipher pages      |   |
|  |  pods/{pod_id}/conduit.db  ← Conduit database backup        |   |
|  |  (Backblaze B2 / Hetzner OS / Cloudflare R2)                |   |
|  +-----------------------------------------------------------+   |
+-----------------------------------------------------------------+
```

### 2.3 Pod Communication: Matrix (Conduit)

Pods communicate via Matrix (Conduit). Each pod gets a Matrix identity: `@pod-{pod_id}:pod-{pod_id}.hkask.local`. OCAP DelegationTokens are carried as custom Matrix event fields (`hkask.ocap_token`). Pods discover each other via the shared Conduit homeserver deployed as a K8s workload.

The Curator pod can directly access other pods' SQLCipher databases because all pods run in the same K3s cluster. This preserves the multi-pod data flow specified in `MULTI_POD_ARCHITECTURE.md` without requiring an API-based sync layer.

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

COPY deploy/k8s/litestream.yml /etc/litestream.yml.template
COPY deploy/k8s/conduit.toml /etc/conduit/conduit.toml.template
COPY deploy/k8s/supervisord.conf /etc/supervisor/conf.d/hkask.conf

COPY deploy/k8s/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME /data
EXPOSE 3000 8008

ENV HKASK_DATA_DIR=/data
ENV LITESTREAM_CONFIG=/etc/litestream.yml

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
```

### 2.5 Entrypoint Script

```bash
#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

envsubst < /etc/litestream.yml.template > /etc/litestream.yml
envsubst < /etc/conduit/conduit.toml.template > /etc/conduit/conduit.toml

if [ ! -f "$DB_PATH" ]; then
    echo "No local database found. Attempting restore from Litestream replica..."
    litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH" || {
        echo "No replica found. Starting with fresh database."
    }
fi

kask migrate --data-dir "$DATA_DIR"

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
environment=POD_ID="%(ENV_POD_ID)s",HKASK_DATA_DIR="/data",HKASK_BASE_URL="%(ENV_HKASK_BASE_URL)s",HKASK_MATRIX_URL="http://localhost:8008"
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
        bucket: ${LITESTREAM_BUCKET}
        path: pods/${POD_ID}/kask.db
        endpoint: ${LITESTREAM_ENDPOINT}
        region: ${LITESTREAM_REGION}
        access-key-id: ${LITESTREAM_ACCESS_KEY_ID}
        secret-access-key: ${LITESTREAM_SECRET_ACCESS_KEY}
        force-path-style: ${LITESTREAM_FORCE_PATH_STYLE}
  - path: /data/conduit.db
    replicas:
      - type: s3
        bucket: ${LITESTREAM_BUCKET}
        path: pods/${POD_ID}/conduit.db
        endpoint: ${LITESTREAM_ENDPOINT}
        region: ${LITESTREAM_REGION}
        access-key-id: ${LITESTREAM_ACCESS_KEY_ID}
        secret-access-key: ${LITESTREAM_SECRET_ACCESS_KEY}
        force-path-style: ${LITESTREAM_FORCE_PATH_STYLE}
```

> **`LITESTREAM_FORCE_PATH_STYLE`**: `true` for Backblaze B2 and Hetzner OS (path-style), `false` for Cloudflare R2 (virtual hosted-style). Set to match your chosen backend.

### 2.8 K8s StatefulSet (Generated by `kask pod export-k8s`)

The `export_k8s` function generates complete K8s manifests including namespace, networkpolicy (per-pod isolation), statefulset (with init containers for litestream-restore and kask-migrate, plus sidecar containers for litestream and conduit), configmaps (litestream + conduit config), secrets, and HPA.

### 2.9 Pod Lifecycle Mapping (K3s)

| hKask Pod State | K8s Operation | Notes |
|-----------------|---------------|-------|
| `Create` | `kubectl apply -f manifests/` | Namespace + NetworkPolicy + StatefulSet created |
| `Populate` | Init containers run | litestream-restore → kask-migrate |
| `Register` | StatefulSet pod starts | supervisord starts conduit + litestream + kask |
| `Activate` | Pod running | Conduit connects to shared Matrix; kask serves API |
| `Deactivate` | `kubectl scale --replicas=0` | Graceful shutdown; Litestream flushes WAL |
| `Destroy` | `kubectl delete namespace` | PVC deleted; Litestream replica remains for migration |

---

## 3. Due Diligence Checklists

### 3.1 Hetzner Cloud Due Diligence

| # | Check | Status | Evidence |
|---|-------|--------|----------|
| 1 | **Data residency.** Where are pod databases physically stored? | ✅ Known | Customer selects location (DE, FI, US, SG). DE/FI data centers ISO 27001 certified. |
| 2 | **Encryption at rest.** Are volumes encrypted? | ⚠️ Verify | Hetzner volumes are not encrypted by default. Must use CSI encryption or rely on SQLCipher. |
| 3 | **ISO/SOC/C5.** Compliance certifications? | ✅ Confirmed | ISO/IEC 27001 (DE, FI DCs). BSI C5:2020 Type 2. KRITIS operator. GDPR-compliant (German GmbH). |
| 4 | **SLA coverage.** What's covered and what's excluded? | ⚠️ Narrow | 99.9% per Cloud Server. Excludes: load balancers, firewalls, snapshots, backups, network. |
| 5 | **Support model.** 24/7? Phone? | ✅ Known | 24/7 data center staff for hardware. Email support during business hours. No phone for Cloud. |
| 6 | **Vendor lock-in.** How hard is migration? | ✅ Low | Standard VPS. Standard K8s. `litestream restore` to any SQLite. K8s manifests are portable. |
| 7 | **GPU availability.** Options for inference? | ❌ None | No GPU instances. Inference must dispatch to external providers (RunPod, Together, etc.). |
| 8 | **Scale-to-zero.** Idle cost? | ❌ None | Billing continues while server object exists. Must delete server to stop charges. |
| 9 | **Pricing stability.** History of price changes? | ⚠️ Monitor | +30-37% increase (April 2026). Upward trend. |
| 10 | **Operational burden.** K8s expertise required? | ⚠️ High | Self-managed K3s requires K8s knowledge. Cloudfleet mitigates. |
| 11 | **Network reliability.** DDoS, peering? | ✅ Known | Free DDoS protection. 99.9% network availability. 20TB free egress. |
| 12 | **Litestream replica target.** What object storage? | ⚠️ External | Hetzner OS (S3-compatible) or Backblaze B2. Not managed by Hetzner within the Cloud SLA. |

---

## 4. Critical Infrastructure Partner Considerations

### 4.1 What "Critical Infrastructure" Means for hKask

For partners running hKask as critical infrastructure, the following are non-negotiable:

1. **Data sovereignty.** Per P1 (User Sovereignty): users own their data. The cloud provider must not have access to decrypted pod databases. SQLCipher ensures this at the application layer.
2. **OCAP boundary integrity.** Per P4.1: pod boundary IS the OCAP enforcement perimeter. K8s namespaces + NetworkPolicy provide the enforcement perimeter.
3. **No ambient authority.** Per P4: no admin bypass. The cloud provider's support team must not have the ability to access pod data or impersonate pods.
4. **Portability.** Per P1: data portability is a first-class guarantee. Pods must be migratable via Litestream restore.

### 4.2 Hetzner Cloud as Critical Infrastructure Partner

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

### 4.3 Multi-Provider Resilience Strategy

```
                    ┌──────────────────────────┐
                    │    kask pod export-*     │
                    │    (provider-agnostic)   │
                    └──────────┬───────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            ▼                  ▼                  ▼
    ┌───────────────┐  ┌───────────────┐  ┌───────────────┐
    │  Hetzner+K3s  │  │    RunPod     │  │   (future)    │
    │  (primary)    │  │   (GPU)       │  │               │
    │               │  │               │  │               │
    │ Orchestrator  │  │ Inference     │  │ Additional    │
    │ + Litestream  │  │ + Training    │  │ providers     │
    │ + CSI Volume  │  │ + NW Volume   │  │               │
    └───────┬───────┘  └───────┬───────┘  └───────┬───────┘
            │                  │                  │
            └──────────────────┼──────────────────┘
                               │
                    ┌──────────▼───────────┐
                    │   Litestream replica  │
                    │   (Backblaze B2 /    │
                    │    Cloudflare R2 /   │
                    │    Hetzner OS)       │
                    │                      │
                    │   Provider-agnostic  │
                    │   Database replicas  │
                    │   Sub-second RPO     │
                    └──────────────────────┘
```

---

## 5. Implementation Sequence

### Phase 1: Foundation — P0

- [x] `Dockerfile` (Rust + Litestream + Conduit, multi-stage)
- [x] `entrypoint.sh` (restore → migrate → supervisord)
- [x] `litestream.yml.template` (kask.db + conduit.db)
- [x] CI/CD pipeline (GitHub Actions, builds on tag push)
- [ ] SQLCipher + Litestream compatibility test

### Phase 2: Hetzner + Object Storage — P1

- [x] Hetzner account provisioned — API key in `.env`, workspace has storage bucket + cloud server
- [x] `HetznerClient` in `hkask-services-cloud` (Cloud API + Object Storage validation)
- [x] `kask pod export-k8s` command — generates 6 YAML manifests
- [x] K8s manifest templates (namespace, networkpolicy, statefulset, hpa, configmap, secrets)
- [ ] K3s cluster bootstrap (hetzner-k3s or Cloudfleet integration)
- [ ] cert-manager + Let's Encrypt setup automation
- [ ] Object Storage bucket provisioning automation
- [x] `kask pod activate` → `kubectl apply` → `cloud_activate_k8s`
- [x] `kask pod deactivate` → `kubectl scale --replicas=0` → `cloud_deactivate_k8s`
- [ ] Integration test: full lifecycle on Hetzner K3s

### Phase 3: RunPod — GPU Workloads — P2

- [ ] `kask pod export-runpod` command
- [ ] RunPod CPU pod template
- [ ] RunPod serverless GPU endpoint template
- [ ] Integration test: orchestrate inference via RunPod serverless

### Phase 4: Multi-Provider Resilience — P3

- [ ] Cross-provider migration test (Hetzner K3s → new K3s cluster via Litestream)
- [ ] CNS span for cloud provider + object storage health
- [ ] Provider health monitoring (which pods on which infrastructure)

---

## 6. References

- [Cloud Deployment Research Report](./cloud-deployment-research-report.md)
- [Hetzner Cloud API](https://docs.hetzner.cloud/)
- [Hetzner SLA](https://docs.hetzner.com/general/company-and-policy/slas-cloud/)
- [Hetzner Security (TOMs)](https://docs.hetzner.com/general/security-and-identify/technical-and-organizational-measures/)
- [Cloudfleet Managed Kubernetes](https://cloudfleet.ai/lp/managed-hetzner-kubernetes/)
- [Litestream Documentation](https://litestream.io/)
- [Backblaze B2](https://www.backblaze.com/cloud-storage)
- [Hetzner Object Storage](https://www.hetzner.com/storage/object-storage)
- [hetzner-k3s](https://vitobotta.github.io/hetzner-k3s/)
