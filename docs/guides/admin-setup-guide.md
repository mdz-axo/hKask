---
title: "Admin Setup Guide — hKask Cloud Deployment"
audience: [administrators, devops]
last_updated: 2026-06-20
version: "0.30.0"
status: "Implementation Guide"
domain: "Deployment"
mds_categories: [lifecycle]
depends_on: ["docs/research/cloud-implementation-plans.md", ".env.example"]
---

# hKask Admin Setup Guide

**Purpose:** Step-by-step guide for deploying hKask to cloud infrastructure. Covers account creation, API key collection, `.env` configuration, container build, and pod deployment to Hetzner K3s.

**Target audience:** System administrators with basic command-line and cloud experience. No Rust knowledge required.

**Estimated time:** 2-4 hours for CORE tier (inference only). 1-2 days for FULL tier (all providers + cloud deployment). Most time is spent creating provider accounts and waiting for API key approval.

---

## 1. Overview

hKask connects to many external services. Each requires an account and an API key. The setup process is:

```
Create provider accounts → Collect API keys → Fill .env → Build container → Deploy to K3s
```

You do not need all providers to start. Three tiers are available:

| Tier | Providers | What You Get |
|------|-----------|-------------|
| **CORE** | 1 inference provider (DeepInfra, fal.ai, Together, or OpenRouter) | Agent chat, basic inference |
| **STANDARD** | + 2 search providers + 1 financial data + Litestream backups | Web search, company research, adapter training, database persistence |
| **FULL** | + Cloud provider (Hetzner) + Matrix + custom domain | Cloud deployment, cross-pod federation |

---

## 2. Prerequisites

- [ ] A computer with `git`, `docker`, and a terminal
- [ ] A GitHub account (for container registry)
- [ ] An email address (for provider sign-ups)
- [ ] A credit/debit card (some providers require one, even for free tiers)
- [ ] 2-4 hours of uninterrupted time for CORE tier

---

## 3. Step-by-Step: CORE Tier (Inference)

### 3.1 Create Your Inference Provider Account

Pick **one** of these four providers. All support the same models via hKask's inference router.

**DeepInfra** (recommended, easiest start):
1. Go to https://deepinfra.com/
2. Sign up (GitHub or email), verify email
3. Go to Dashboard → API Keys
4. Create a new key, copy it
5. `.env`: `DI_API_KEY=paste-key-here`

**fal.ai** (pay-per-use, generous free tier):
1. Go to https://fal.ai/
2. Sign up, go to Settings → API Keys
3. Create key, copy it
4. `.env`: `FA_API_KEY=paste-key-here`

**Together AI** ($5 free credits):
1. Go to https://together.ai/
2. Sign up, go to Settings → API Keys
3. Create key, copy it
4. `.env`: `TOGETHER_API_KEY=paste-key-here`

**OpenRouter** (unified API for 200+ models):
1. Go to https://openrouter.ai/
2. Sign up, go to Keys
3. Create key, copy it
4. `.env`: `OPENROUTER_API_KEY=paste-key-here`

### 3.2 Configure .env

```bash
cp .env.example .env
```

Edit `.env` and set at minimum:
```bash
DI_API_KEY=your-key-here
HKASK_DEFAULT_PROVIDER=DI
HKASK_DEFAULT_MODEL=DI/meta-llama/Llama-3.3-70B-Instruct
```

### 3.3 Run Locally to Verify

```bash
cargo run -- repl
# In the REPL: ask the agent a question
# > What is the capital of France?
```

### 3.4 Verify

If the agent responds, your CORE tier is working. You can use hKask locally with inference.

---

## 4. Step-by-Step: STANDARD Tier (Search + Financial Data + Backups)

### 4.1 Web Search Providers

Pick at least one:

- **Brave Search** (free: 2,000 queries/month): https://brave.com/search/api/ → `.env`: `HKASK_BRAVE_API_KEY=`
- **Firecrawl** (web scraping + search): https://firecrawl.dev/ → `.env`: `HKASK_FIRECRAWL_API_KEY=`
- **Tavily** (AI-optimized search): https://tavily.com/ → `.env`: `HKASK_TAVILY_API_KEY=`
- **SerpAPI** (Google Search API): https://serpapi.com/ → `.env`: `HKASK_SERPAPI_API_KEY=`
- **Exa** (semantic search): https://exa.ai/ → `.env`: `HKASK_EXA_API_KEY=`

### 4.2 Financial Data Providers

- **Financial Modeling Prep**: https://financialmodelingprep.com/ → `.env`: `HKASK_FMP_API_KEY=`
- **EOD Historical Data**: https://eodhd.com/ → `.env`: `HKASK_EODHD_API_KEY=`

### 4.3 Adapter / Training Providers

- **RunPod** (GPU cloud for inference + training): https://runpod.io/ → `.env`: `RUNPOD_API_KEY=`
- **HuggingFace** (model/adapter repository): https://huggingface.co/settings/tokens → `.env`: `HF_TOKEN=`

### 4.4 Litestream Object Storage (Database Backups)

Litestream is the canonical hKask persistence strategy. It continuously streams SQLite WAL to S3-compatible object storage, providing sub-second RPO and disaster recovery. Backs up both `kask.db` (pod state) and `conduit.db` (Matrix messages).

Choose one object storage backend:

**Backblaze B2** (cheapest, 10GB free):
1. Go to https://www.backblaze.com/b2/cloud-storage.html
2. Create account, create a bucket
3. Create an Application Key with read/write access to that bucket
4. Note the endpoint (e.g., `s3.us-west-000.backblazeb2.com`) and key details
5. `.env`:
   ```bash
   LITESTREAM_BUCKET=your-bucket-name
   LITESTREAM_ENDPOINT=https://s3.us-west-000.backblazeb2.com
   LITESTREAM_REGION=us-west-000
   LITESTREAM_ACCESS_KEY_ID=your-application-key-id
   LITESTREAM_SECRET_ACCESS_KEY=your-application-key
   LITESTREAM_FORCE_PATH_STYLE=true
   ```

**Hetzner Object Storage** (EUR 5/TB/mo, EU data residency):
1. Go to https://console.hetzner.cloud/
2. Create a bucket in your project
3. Generate S3 access keys
4. `.env`:
   ```bash
   LITESTREAM_BUCKET=your-bucket-name
   LITESTREAM_ENDPOINT=https://nbg1.your-objectstorage.com
   LITESTREAM_REGION=nbg1
   LITESTREAM_ACCESS_KEY_ID=your-access-key
   LITESTREAM_SECRET_ACCESS_KEY=your-secret-key
   LITESTREAM_FORCE_PATH_STYLE=true
   ```

**Cloudflare R2** (10GB free, zero egress, global edge caching):
1. Go to https://dash.cloudflare.com/
2. Create R2 bucket, generate API tokens
3. `.env`:
   ```bash
   LITESTREAM_BUCKET=your-bucket-name
   LITESTREAM_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com
   LITESTREAM_REGION=auto
   LITESTREAM_ACCESS_KEY_ID=your-access-key-id
   LITESTREAM_SECRET_ACCESS_KEY=your-secret-access-key
   LITESTREAM_FORCE_PATH_STYLE=false
   ```

#### Decision Guide

| If you... | Choose |
|-----------|--------|
| Want the cheapest option | **Backblaze B2** |
| Use Hetzner for compute and want EU data residency | **Hetzner OS** |
| Do frequent pod restores and worry about egress costs | **Cloudflare R2** |

---

## 5. Step-by-Step: FULL Tier (Cloud Deployment)

### 5.1 Container Registry

Push the built container image to a registry your K3s cluster can pull from.

**GitHub Container Registry (recommended, free for public repos):**

```bash
# 1. Create a Personal Access Token (classic) with write:packages scope
#    https://github.com/settings/tokens

# 2. Login to GHCR
echo "$GITHUB_TOKEN" | docker login ghcr.io -u YOUR_USERNAME --password-stdin

# 3. Set in .env
CONTAINER_REGISTRY=ghcr.io/your-org/hkask
```

### 5.2 Cloud Provider: Hetzner Cloud + K3s

Hetzner is the recommended cloud provider for hKask deployments. It is the cost leader (3-5x cheaper than other providers at scale) and the recommended choice for EU-regulated workloads (BSI C5, ISO 27001, GDPR-compliant by jurisdiction). K3s provides per-pod namespace isolation with NetworkPolicy enforcement.

#### Step 1: Create Hetzner account and API token

1. Go to https://console.hetzner.cloud/
2. Create a project
3. Navigate to Security → API Tokens → Generate API Token (Read & Write)
4. `.env`: `HCLOUD_TOKEN=your-token`

#### Step 2: Provision Object Storage (if using Hetzner OS)

1. In Hetzner Console, navigate to Object Storage
2. Create a bucket (e.g., `hkask-pods-backup`)
3. Generate S3 access keys
4. Set in `.env` per §4.4 above

#### Step 3: Create K3s Cluster

Choose one path:

**Path A: Self-managed K3s (cheapest, full control)**

```bash
# Install hetzner-k3s
curl -sL https://github.com/vitobotta/hetzner-k3s/releases/latest/download/hetzner-k3s-linux-amd64 -o hetzner-k3s
chmod +x hetzner-k3s

# Create cluster (2-3 minutes)
hetzner-k3s create \
  --name hkask-prod \
  --location nbg1 \
  --masters 3 --master-type cx33 \
  --workers 3 --worker-type cx43 \
  --network-zone eu-central \
  --autoscaling-enabled

# Outputs kubeconfig to ./kubeconfig
export KUBECONFIG=$(pwd)/kubeconfig
kubectl get nodes
```

**Path B: Cloudfleet managed K8s (simpler, from free tier)**

1. Go to https://cloudfleet.ai/ and sign up (free Basic tier, up to 24 vCPUs)
2. Connect your Hetzner account (provide API token)
3. Create cluster, select Hetzner region, deploy
4. Download kubeconfig from Cloudfleet dashboard

#### Step 4: Install cert-manager for TLS

```bash
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/latest/download/cert-manager.yaml

cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: your-email@example.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF
```

#### Step 5: Install NGINX Ingress Controller

```bash
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.10.0/deploy/static/provider/cloud/deploy.yaml
```

### 5.3 Matrix / Conduit (Curator-Managed)

The Matrix Conduit server is **set up once by the Curator** during system installation. Pod admins do not configure it directly. What you need to know:

- **`HKASK_MATRIX_URL`** — set by the Curator. Points to the shared Conduit homeserver. This is the only Matrix-related value in your `.env`.
- **Agent credentials** — auto-generated by `kask pod create` and stored as K8s Secrets. Never manually configured.
- **Conduit signing key** — owned by the Curator, stored in the Curator's keystore. This is a system-level secret, not a per-pod setting.

**Curator setup (for reference):**

```bash
# Curator initializes the system (one-time):
kask curator init --domain hkask.your-domain.com
# This generates the Conduit signing key, stores it in the Curator's keystore,
# and deploys the shared Conduit as a K8s StatefulSet.

# Export K8s manifests for Conduit + Curator:
kask pod export-k8s curator

# Apply:
kubectl apply -f k8s-manifests/

# Curator creates a pod (per-user):
kask pod create my-first-pod
# This auto-generates Matrix agent credentials and stores them as pod secrets.

# Deploy the pod:
kask pod export-k8s my-first-pod
kubectl apply -f k8s-manifests/
```

### 5.4 DNS Configuration

**For v1, DNS configuration is manual but straightforward.** Your pods are accessible through the K8s ingress. Set up A/AAAA records at your registrar pointing to your K3s cluster's load balancer IP.

Custom domains (e.g., `hkask.your-domain.com`) are a v2 feature alongside crypto integration. For v1, use the cluster's IP or a single domain with path-based routing.

---

## 6. Build and Deploy

hKask uses a container image (`kask` binary + Litestream + Conduit). You have three options for getting this image:

| Path | Build Where | Requires | Best For |
|------|------------|----------|----------|
| **A: Pre-built (fastest)** | None — pull from GHCR | Nothing | Quick start, no customizations |
| **B: Cloud build (recommended)** | GitHub Actions | GitHub repo | Custom builds, CI/CD pipeline |
| **C: Local build** | Your machine | Docker + 4GB RAM | Development, testing changes |

### 6.1 Path A: Pre-Built Image (No Build Required)

```bash
# Skip the build entirely. Use the official image from GitHub Container Registry.
# Set in .env:
CONTAINER_REGISTRY=ghcr.io/mdz-axo/hkask

# K8s pulls from registry at deploy time.
# Just reference the image in your StatefulSet and deploy.
```

### 6.2 Path B: Cloud Build via GitHub Actions

```yaml
# .github/workflows/build.yml
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
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.ref_name }}
```

Push a tag to trigger: `git tag v0.30.0 && git push origin v0.30.0`

### 6.3 Path C: Local Build

```bash
# Requires: Docker with 4GB+ memory allocated
# WARNING: Rust + Conduit compilation can take 10-20 minutes on first build

docker build -t kask:0.30.0 .
docker tag kask:0.30.0 ghcr.io/your-org/hkask:kask-0.30.0
docker push ghcr.io/your-org/hkask:kask-0.30.0
```

### 6.4 Deploy to Hetzner K3s

```bash
# Generate K8s manifests for a pod
kask pod export-k8s my-first-pod \
  --volume-size-gb 10 \
  --max-replicas 3

# This creates:
#   k8s-manifests/namespace.yaml       — isolated pod namespace
#   k8s-manifests/networkpolicy.yaml   — ingress/egress rules
#   k8s-manifests/statefulset.yaml     — main pod + litestream sidecar + conduit
#   k8s-manifests/configmap.yaml       — litestream + conduit config
#   k8s-manifests/secrets.yaml         — API keys and credentials
#   k8s-manifests/hpa.yaml             — horizontal pod autoscaler

# Deploy
kubectl apply -f k8s-manifests/

# Verify
kubectl get pods -n hkask-pod-my-first-pod
kubectl logs -n hkask-pod-my-first-pod statefulset/kask
```

### 6.5 Verify Deployment

```bash
# Check pod health
curl https://hkask.your-domain.com/health
# Expected: {"status":"ok","pod_id":"my-first-pod"}

# Check Litestream replication
kubectl exec -n hkask-pod-my-first-pod statefulset/kask -c litestream -- \
  litestream generations /data/kask.db
# Expected: list of generations in object storage

# Check Conduit federation
curl https://hkask.your-domain.com:8448/_matrix/federation/v1/version
# Expected: {"server":{"name":"hkask.your-domain.com","version":"..."}}
```

---

## 7. Making Setup Easier

The most time-consuming part of hKask setup is creating all the provider accounts and collecting API keys. Here's how we're addressing this:

### 7.1 `kask setup wizard` (Planned — Phase 2)

An interactive terminal wizard that walks you through each provider:

```
$ kask setup wizard

  Welcome to hKask setup!
  Let's configure your providers.

  ── Inference ──────────────────────────────────
  Which inference provider? [DeepInfra / Together / fal.ai / OpenRouter / skip]
  > DeepInfra

  Open https://deepinfra.com/dash/api_keys in your browser?
  [Y] Yes  [N] I'll do it myself
  > Y

  Paste your API key:
  > [hidden input]

  ✅ DeepInfra configured.

  ── Web Search ─────────────────────────────────
  ... (repeats for each provider)
```

The wizard:
- Opens provider signup pages in the browser automatically
- Validates keys on entry (makes a test API call)
- Generates the `.env` file incrementally
- Tracks which providers are configured and which are missing
- Saves state so you can resume later

### 7.2 `kask doctor` (Planned — Phase 2)

Validates your entire configuration:

```
$ kask doctor

  Inference Providers
  ✅ DI_API_KEY          — DeepInfra (valid, 80 models available)
  ⚠️ TOGETHER_API_KEY    — not set (Together AI unavailable)
  ❌ FA_API_KEY           — set but INVALID (401 Unauthorized)

  Search Providers
  ✅ HKASK_BRAVE_API_KEY  — Brave Search (valid, 1987/2000 queries remaining)
  ✅ HKASK_FIRECRAWL_API_KEY — Firecrawl (valid)
  ⚠️ HKASK_TAVILY_API_KEY — not set

  Financial Data
  ✅ HKASK_FMP_API_KEY    — FMP (valid)
  ⚠️ HKASK_EODHD_API_KEY  — not set

  Cloud Providers
  ✅ HCLOUD_TOKEN         — Hetzner Cloud (valid)
  ⚠️ CONTAINER_REGISTRY   — not set

  Storage
  ✅ LITESTREAM_BUCKET — Backblaze B2 (valid, 3 generations found)

  Matrix
  ✅ HKASK_MATRIX_URL     — http://localhost:8008 (Conduit reachable)
  ✅ Matrix agent credentials — auto-generated by kask pod create

  Summary: 8/12 providers configured, 1 invalid, 3 missing
  Tier: STANDARD (CORE ✅, STANDARD ✅, FULL ⚠️)
```

### 7.3 CNS Span: `cns.setup.provider_health`

Each provider's health status is emitted as a CNS span. The Curator can detect when:
- An API key is invalid (401)
- A provider is rate-limited (429)
- A free tier is about to expire
- A new provider was added

### 7.4 `kask keystore` (Existing — Enhanced)

Currently, `kask keystore` stores keys in the OS keychain. Enhancement plan:

```
# Load all provider keys from .env into encrypted OS keychain
kask keystore load --from .env

# Securely shred the plaintext .env
kask keystore load --from .env --shred

# Export (for migration to new machine)
kask keystore export --to .env.migrate
```

---

## 8. Crypto System Onboarding (Release 2)

*This section is a placeholder for the crypto/wallet onboarding planned for the second release.*

The crypto system will add:

- **Wallet provider keys**: Solana, Hedera, Hinkal (already in `cfg(feature)` gating)
- **Delegation token generation**: `kask delegation create --for pod-{id} --capabilities inference,search`
- **OCAP capability enrollment**: defining which pods can use which providers
- **Transaction signing**: wallet-backed CNS gas accounting

These will be added to the CORE tier as optional providers (no wallet = no gas accounting, purely advisory mode).

---

## 9. Troubleshooting

### Common Issues

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `kask doctor` shows all providers missing | `.env` not loaded | `source .env` before running commands |
| `DI_API_KEY: set but INVALID` | Wrong key or expired | Regenerate at deepinfra.com/dash/api_keys |
| `litestream: No replica found` | Bucket not created or wrong credentials | Check `LITESTREAM_*` vars, create bucket |
| `kubectl apply: connection refused` | kubeconfig not set or K3s not running | `export KUBECONFIG=$(pwd)/kubeconfig`, verify cluster |
| `conduit: federation rejected` | Wrong signing key or DNS | Check Conduit signing key, verify ingress DNS |
| Container build fails (out of memory) | Rust + Conduit both compile | Increase Docker memory limit to 4GB, or build in CI |
| Pod starts but no inference | Model not available on chosen provider | Check provider's model list, try `DI/Qwen/Qwen2.5-7B-Instruct` |
| StatefulSet stuck in Pending | PVC not bound | Check storage class: `kubectl get sc` |
| cert-manager: certificate not ready | DNS not propagated | Wait 5-15 min for DNS + Let's Encrypt validation |

---

## 10. Quick Reference Card

```bash
# --- First Time Setup ---
cp .env.example .env                          # Create config
# Edit .env with at least one inference key
kask doctor                                   # Validate configuration

# --- Build (pick one path) ---
# Path A: Pre-built image (skip to deploy)
# Path B: Cloud build — git tag v0.30.0 && git push origin v0.30.0
# Path C: Local build — docker build -t kask:0.30.0 . && docker push

# --- Deploy (Hetzner K3s) ---
kask pod export-k8s my-pod                    # Generate K8s manifests
kubectl apply -f k8s-manifests/               # Deploy to cluster
kubectl get pods -n hkask-pod-my-pod          # Check pod status
kubectl logs -n hkask-pod-my-pod statefulset/kask  # Watch logs

# --- Verify ---
curl https://hkask.your-domain.com/health     # Health check

# Litestream check:
kubectl exec -n hkask-pod-my-pod statefulset/kask -c litestream -- \
  litestream generations /data/kask.db
```

---

## 11. References

- [`.env.example`](../../.env.example) — Complete environment variable template
- [`docs/research/cloud-deployment-research-report.md`](../plans/deployment-and-backup.md#14-related-research-and-past-plans) — Provider evaluation
- [Litestream Documentation](https://litestream.io/)
- [Conduit Matrix Server](https://conduit.rs/)
- [Hetzner Cloud Documentation](https://docs.hetzner.com/cloud/)
