---
title: "Hetzner K3s Implementation Plan — hKask"
audience: [developers]
last_updated: 2026-06-20
version: "0.30.0"
status: "Implementation Plan"
domain: "Deployment"
mds_categories: [lifecycle]
depends_on: ["docs/guides/kubernetes-primer.md", "docs/research/cloud-implementation-plans.md"]
---

# Hetzner K3s Implementation Plan

**Purpose:** Concrete, ordered plan for deploying hKask on Hetzner Cloud using K3s. Each phase is independently verifiable. Each task names the files that must change and the success criteria.

**Current state:** We have `export_k8s` (generates 6 YAML manifests), `HetznerClient` (Cloud API + Object Storage validation), `cloud_activate_k8s`/`cloud_deactivate_k8s` (kubectl integration via subprocess), and a broken Dockerfile that references deleted `deploy/fly/` paths. Nothing has been tested against a real K3s cluster.

---

## Phase 0: Clean Up Fly.io Artifacts

**Goal:** Remove all fly.io deployment artifacts so we have a clean foundation.

### Task 0.1: Delete `deploy/fly/`

```
rm -rf deploy/fly/
```

The directory contains `entrypoint.sh` (uses `litestream replicate -exec`, no Conduit/supervisord) and `litestream.yml.template` (references Tigris, only backs up kask.db). Both are fly.io-specific and will be replaced with K8s versions.

### Task 0.2: Create `deploy/k8s/` directory

Empty directory to receive K8s-specific deployment configs in Phase 1.

**Verify:** `ls deploy/k8s/` shows empty directory, `deploy/fly/` no longer exists.

---

## Phase 1: Build Pipeline (Dockerfile)

**Goal:** A Dockerfile that builds the `kask` binary, Litestream, and Conduit, then produces a container image with supervisord managing all three processes. A GitHub Actions workflow pushes tagged images to GHCR.

### Task 1.1: Rewrite `deploy/Dockerfile`

**Current problems:**
- References `deploy/fly/` paths (deleted)
- Missing Conduit build stage
- No supervisord for multi-process management
- Exposes port 3000 only (Conduit needs 8008)

**New Dockerfile structure:**

```
Stage 1: Build kask (Rust)
Stage 2: Build Litestream (Go)
Stage 3: Build Conduit (Rust)
Stage 4: Runtime (Debian slim + supervisord + all binaries + config templates)
```

Key changes from current:
- Add Conduit build stage (clone gitlab.com/famedly/conduit, cargo build)
- Add supervisord to runtime stage
- Copy from `deploy/k8s/` instead of `deploy/fly/`
- Expose ports 3000 (kask API) and 8008 (Conduit Matrix)
- Four config templates: litestream.yml, conduit.toml, supervisord.conf, entrypoint.sh

**File to change:** `deploy/Dockerfile`

### Task 1.2: Create `deploy/k8s/entrypoint.sh`

Replaces the fly.io version. Changes:
- Renders conduit.toml template (in addition to litestream.yml)
- Still uses litestream restore for initial DB recovery
- Still runs kask migrate
- Starts supervisord instead of `litestream replicate -exec` (because we need three processes: kask, litestream, conduit)

```bash
#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

echo "=== hKask pod starting ==="
echo "Pod ID: ${POD_ID:-unknown}"
echo "Data directory: $DATA_DIR"

mkdir -p "$DATA_DIR"

# Render configs from environment variables
envsubst < /etc/litestream.yml.template > /etc/litestream.yml
envsubst < /etc/conduit/conduit.toml.template > /etc/conduit/conduit.toml

# Restore kask database from Litestream if no local copy
if [ ! -f "$DB_PATH" ]; then
    echo "No local database. Attempting restore from Litestream replica..."
    if litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH"; then
        echo "Database restored from object storage."
    else
        echo "No replica found. Starting with fresh database."
    fi
fi

echo "Running database migrations..."
kask migrate --data-dir "$DATA_DIR" || echo "Warning: migrate command failed"

echo "Starting supervisord..."
exec /usr/bin/supervisord -c /etc/supervisor/supervisord.conf
```

**File to create:** `deploy/k8s/entrypoint.sh`

### Task 1.3: Create `deploy/k8s/litestream.yml.template`

Updated from the fly.io version:
- Remove Tigris references in comments
- Add conduit.db replica alongside kask.db
- Uses `${LITESTREAM_FORCE_PATH_STYLE}` without default (set in .env)

```yaml
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

**File to create:** `deploy/k8s/litestream.yml.template`

### Task 1.4: Create `deploy/k8s/conduit.toml.template`

Conduit Matrix homeserver configuration, rendered by envsubst at container start.

```toml
[global]
server_name = "${CONDUIT_SERVER_NAME:-localhost}"
address = "0.0.0.0"
port = 8008

[global.database]
backend = "sqlite"
path = "/data/conduit.db"

[global.registration]
enabled = false
```

**File to create:** `deploy/k8s/conduit.toml.template`

### Task 1.5: Create `deploy/k8s/supervisord.conf`

Manages three processes: conduit (Matrix), litestream (WAL replication), kask (main app).

```ini
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

**File to create:** `deploy/k8s/supervisord.conf`

### Task 1.6: Create `.github/workflows/build.yml`

GitHub Actions workflow that builds and pushes on tag push.

```yaml
name: Build and Push
on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          context: .
          file: ./deploy/Dockerfile
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.ref_name }},ghcr.io/${{ github.repository }}:kask-${{ github.ref_name }}
```

**File to create:** `.github/workflows/build.yml`

### Task 1.7: Verify Build

```bash
# Local build test (requires ~4GB Docker memory)
docker build -f deploy/Dockerfile -t kask:local .

# Verify the image contains all three binaries
docker run --rm kask:local which kask litestream conduit

# Verify supervisord config parses
docker run --rm kask:local cat /etc/supervisor/supervisord.conf

# Push a tag to trigger CI (after all files committed)
git tag v0.30.0
git push origin v0.30.0

# Verify image is on GHCR
docker pull ghcr.io/mdz-axo/hkask:kask-v0.30.0
```

**Success criteria:** `docker build` succeeds. `docker run --rm kask:local which kask litestream conduit` shows all three paths. GHCR image pulls successfully after tag push.

---

## Phase 2: K3s Cluster Operations

**Goal:** A running K3s cluster on Hetzner with cert-manager, ingress controller, and shared Conduit. The `curator_init` command actually does the work instead of printing a stub message.

### Task 2.1: Implement `curator_init` (Real)

Replace the current stub in `crates/hkask-cli/src/commands/curator.rs` with a real implementation that:

1. Validates `HCLOUD_TOKEN`, `CONTAINER_REGISTRY`, `LITESTREAM_*`, `HKASK_BASE_URL` env vars
2. Calls `HetznerClient::validate_token()` to confirm API access
3. Generates a Conduit signing key (Ed25519, base64)
4. Creates a K8s namespace `hkask-conduit` for the shared Conduit
5. Generates and applies K8s manifests for the shared Conduit (Deployment + Service + Secret with signing key)
6. Stores the signing key in the Curator's keystore
7. Creates the Curator pod namespace and applies its manifests
8. Prints the Matrix URL and Curator URL for the admin

**File to change:** `crates/hkask-cli/src/commands/curator.rs`

**Success criteria:** Running `kask curator init --domain hkask.example.com` with valid env vars creates the Conduit namespace, deploys Conduit, creates the Curator pod, and prints accessible URLs.

### Task 2.2: Create Shared Conduit K8s Manifests

The shared Conduit is a Deployment (not StatefulSet — stateless, all state is in the SQLite database on a PVC) in the `hkask-conduit` namespace. Needs:

- `namespace.yaml` — creates `hkask-conduit` namespace
- `deployment.yaml` — Conduit container, single replica
- `service.yaml` — ClusterIP service on port 8008
- `pvc.yaml` — Persistent volume for conduit.db
- `secret.yaml` — Conduit signing key, server name

These could be generated by `curator_init` or stored as templates alongside the other deployments.

### Task 2.3: K3s Bootstrap Script

A shell script that automates the cluster creation steps from the Kubernetes primer (§6):

```bash
#!/bin/bash
# scripts/bootstrap-k3s.sh
set -e

# Validate
: "${HCLOUD_TOKEN:?Set HCLOUD_TOKEN in environment}"

# Create cluster
hetzner-k3s create \
  --name hkask-prod \
  --location nbg1 \
  --masters 3 --master-type cx33 \
  --workers 3 --worker-type cx43 \
  --network-zone eu-central \
  --autoscaling-enabled

export KUBECONFIG=$(pwd)/kubeconfig

# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/latest/download/cert-manager.yaml

# Install NGINX Ingress
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.10.0/deploy/static/provider/cloud/deploy.yaml

# Wait for ingress to get an external IP
echo "Waiting for ingress external IP..."
kubectl wait --namespace ingress-nginx \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/component=controller \
  --timeout=300s

kubectl get svc -n ingress-nginx
echo "Cluster ready. KUBECONFIG: $(pwd)/kubeconfig"
```

**File to create:** `scripts/bootstrap-k3s.sh`

### Task 2.4: Object Storage Bucket Provisioning

Add a `create_bucket` method to `HetznerClient` or provide a script. Hetzner Object Storage uses the S3 API, so bucket creation is a `PUT` to the endpoint. This could also be done manually via the Hetzner Console for v1.

**File to change:** `crates/hkask-services-cloud/src/hetzner.rs` (optional, v1 manual setup acceptable)

---

## Phase 3: Pod Lifecycle End-to-End

**Goal:** A full pod lifecycle works: create (local DB) → export (K8s manifests) → deploy (kubectl apply) → verify (health check) → scale (activate/deactivate) → destroy (delete namespace). Data survives deactivation and is restorable.

### Task 3.1: Verify `export_k8s` Manifests Against Real Cluster

Generate manifests for a test pod and apply them:

```bash
# Create test pod
kask pod create test-pod

# Export K8s manifests
kask pod export-k8s test-pod --volume-size-gb 10 --max-replicas 1

# Apply
kubectl apply -f k8s-manifests/

# Watch startup
kubectl get pods -n hkask-pod-test-pod -w
```

Expected sequence:
1. Namespace created
2. PVC provisioned (Hetzner CSI creates block volume)
3. Init container `litestream-restore` runs (no-op if no replica exists)
4. Init container `kask-migrate` runs
5. Main containers start: conduit → litestream → kask
6. Pod reaches Ready state

**Verify:** `kubectl get pods -n hkask-pod-test-pod` shows 1/1 Ready. `kubectl port-forward -n hkask-pod-test-pod statefulset/kask 3000:3000` and `curl http://localhost:3000/health` returns `{"status":"ok"}`.

### Task 3.2: Fix Image Reference in `export_k8s`

The current `export_k8s` references `{container_registry}:kask-{version}` but the Dockerfile's GitHub Actions workflow will push as `ghcr.io/org/hkask:kask-v0.30.0`. Verify the tag format matches or adjust `export_k8s`.

**File to check:** `crates/hkask-cli/src/commands/pod.rs::export_k8s` (lines 186, 193, 224)

### Task 3.3: Test Pod Deactivation (Scale to Zero)

```bash
kask pod deactivate test-pod
```

Expected: `kubectl scale statefulset kask --replicas=0 -n hkask-pod-test-pod`. Pod terminates. PVC survives.

**Verify:** `kubectl get pods -n hkask-pod-test-pod` shows no pods. `kubectl get pvc -n hkask-pod-test-pod` shows PVC still exists.

### Task 3.4: Test Pod Reactivation

```bash
kask pod activate test-pod
```

Expected: `kubectl scale statefulset kask --replicas=1 -n hkask-pod-test-pod`. Pod restarts. Init containers restore database from Litestream if needed, otherwise use existing PVC data.

**Verify:** Pod reaches Ready. `curl` health endpoint responds. Conduit reconnects to shared Matrix.

### Task 3.5: Test Litestream Backup and Restore

```bash
# Verify Litestream is replicating
kubectl exec -n hkask-pod-test-pod statefulset/kask -c litestream -- \
  litestream generations /data/kask.db

# Expected: at least one generation in object storage

# Simulate disaster: delete namespace (PVC too)
kubectl delete namespace hkask-pod-test-pod

# Re-deploy (fresh PVC)
kask pod export-k8s test-pod
kubectl apply -f k8s-manifests/

# Init container should restore from Litestream
kubectl logs -n hkask-pod-test-pod statefulset/kask -c litestream-restore
# Expected: "Database restored from object storage"
```

### Task 3.6: Test Pod Destruction

```bash
kubectl delete namespace hkask-pod-test-pod
```

Expected: Namespace, pods, PVC, NetworkPolicy, Secrets, ConfigMaps all deleted. Litestream replica in object storage remains (for potential migration to another cluster).

**Verify:** `kubectl get namespace hkask-pod-test-pod` returns "not found". Object storage still contains `pods/test-pod/kask.db`.

---

## Phase 4: Production Readiness

**Goal:** The system can be operated in production with monitoring, health checks, and backup verification.

### Task 4.1: Add CNS Spans for K8s Operations

CNS spans should be emitted for:
- `cns.cloud.k8s.apply` — when `kubectl apply` succeeds/fails
- `cns.cloud.k8s.scale` — when StatefulSet is scaled
- `cns.cloud.litestream.restore` — when init container restores from object storage
- `cns.cloud.litestream.replicate` — periodic health check of Litestream sidecar

**File to change:** `crates/hkask-cli/src/commands/pod.rs` (cloud_activate_k8s, cloud_deactivate_k8s)

### Task 4.2: Create ClusterIssuer for cert-manager

Automate the Let's Encrypt ClusterIssuer creation as part of `curator_init` or the bootstrap script:

```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: ${ADMIN_EMAIL}
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
```

### Task 4.3: Ingress and DNS

Create an Ingress resource that routes `hkask-pod-{id}.{domain}` to the pod's kask service. This requires:
- A wildcard DNS record (`*.hkask.example.com` → Hetzner Load Balancer IP)
- An Ingress resource per pod (or a single wildcard ingress)
- cert-manager annotation for automatic TLS

### Task 4.4: Backup Verification

A periodic job (CronJob) that runs `litestream verify` against each pod's replica to confirm backups are valid and restorable. Emits CNS span on failure.

---

## Dependency Order

```
Phase 0 (cleanup)
  └─> Phase 1 (Dockerfile, CI)
        └─> Phase 2 (cluster ops, curator_init)
              └─> Phase 3 (pod lifecycle e2e)
                    └─> Phase 4 (production readiness)
```

Phase 1 is the hard blocker — without a buildable Docker image, nothing else can be tested. Phase 2 requires a running K3s cluster but can be developed in parallel with Phase 1. Phase 3 requires Phase 1 + 2. Phase 4 is polish.

---

## Files Summary

| File | Phase | Action |
|------|-------|--------|
| `deploy/fly/` | 0 | Delete entire directory |
| `deploy/Dockerfile` | 1 | Rewrite (add Conduit stage, supervisord, K8s configs) |
| `deploy/k8s/entrypoint.sh` | 1 | Create |
| `deploy/k8s/litestream.yml.template` | 1 | Create (kask.db + conduit.db) |
| `deploy/k8s/conduit.toml.template` | 1 | Create |
| `deploy/k8s/supervisord.conf` | 1 | Create |
| `.github/workflows/build.yml` | 1 | Create |
| `crates/hkask-cli/src/commands/curator.rs` | 2 | Rewrite curator_init |
| `scripts/bootstrap-k3s.sh` | 2 | Create |
| `crates/hkask-services-cloud/src/hetzner.rs` | 2 | Optional: add create_bucket |
| `crates/hkask-cli/src/commands/pod.rs` | 3 | Verify image tag format, add CNS spans |
