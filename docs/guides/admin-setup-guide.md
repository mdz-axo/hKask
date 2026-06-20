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

**Purpose:** Step-by-step guide for deploying hKask to cloud infrastructure. Covers account creation, API key collection, `.env` configuration, container build, and pod deployment.

**Target audience:** System administrators with basic command-line and cloud experience. No Rust knowledge required.

**Estimated time:** 2-4 hours for CORE tier (inference only). 1-2 days for FULL tier (all providers + cloud deployment). Most time is spent creating provider accounts and waiting for API key approval.

---

## 1. Overview

hKask connects to many external services. Each requires an account and an API key. The setup process is:

```
Create provider accounts → Collect API keys → Fill .env → Build container → Deploy
```

You do not need all providers to start. Three tiers are available:

| Tier | Providers | What You Get |
|------|-----------|-------------|
| **CORE** | 1 inference provider (DeepInfra, fal.ai, Together, or OpenRouter) | Agent chat, basic inference |
| **STANDARD** | + 2 search providers + 1 financial data + Litestream backups | Web search, company research, adapter training, database persistence |
| **FULL** | + Cloud provider + Matrix + custom domain | Cloud deployment, cross-pod federation |

---

## 2. Prerequisites

- [ ] A computer with `git`, `docker`, and a terminal
- [ ] A GitHub account (for container registry)
- [ ] An email address (for provider sign-ups)
- [ ] A credit/debit card (some providers require one, even for free tiers)
- [ ] 2-4 hours of uninterrupted time for CORE tier

---

## 3. Step-by-Step: CORE Tier (Inference)

### 3.1 Choose Your Inference Provider

Pick ONE to start. You can add more later.

| Provider | Free Tier | Sign-Up Time | Best For |
|----------|-----------|-------------|----------|
| **DeepInfra** | $1.30/mo credits | 2 minutes | Lowest cost, broad model support |
| **Together AI** | $5 free credits | 2 minutes | Training + inference, good free tier |
| **fal.ai** | Pay-per-use, generous free | 2 minutes | Image/video generation, fast inference |
| **OpenRouter** | Pay-per-use | 2 minutes | Access to 200+ models from one key |

### 3.2 Create Account and Get API Key

**DeepInfra (recommended for first setup):**

1. Go to https://deepinfra.com/
2. Click "Sign Up" → sign in with GitHub or Google
3. Go to https://deepinfra.com/dash/api_keys
4. Click "New API Key" → copy the key
5. Paste into `.env`: `DI_API_KEY=your-key-here`

**Together AI:**

1. Go to https://api.together.ai/
2. Sign up → verify email
3. Go to https://api.together.ai/settings/api-keys
4. Copy the key → `.env`: `TOGETHER_API_KEY=your-key-here`

**fal.ai:**

1. Go to https://fal.ai/
2. Sign up → go to https://fal.ai/dashboard/keys
3. Copy key → `.env`: `FA_API_KEY=your-key-here`

**OpenRouter:**

1. Go to https://openrouter.ai/
2. Sign up → go to https://openrouter.ai/keys
3. Create key → `.env`: `OPENROUTER_API_KEY=your-key-here`

### 3.3 Configure .env

```bash
# Copy the template
cp .env.example .env

# Edit with your values
# At minimum, set ONE of these:
DI_API_KEY=your-deepinfra-key
# TOGETHER_API_KEY=your-together-key
# FA_API_KEY=your-fal-key
# OPENROUTER_API_KEY=your-openrouter-key

# Set your default provider and model
HKASK_DEFAULT_PROVIDER=DI
HKASK_DEFAULT_MODEL=DI/meta-llama/Llama-3.3-70B-Instruct
```

### 3.4 Verify

```bash
# Load .env and run a quick health check
source .env
cargo run --bin kask -- doctor

# Expected output:
# ✅ DI_API_KEY: configured
# ✅ Inference router: DeepInfra available
# ⚠️ HKASK_BRAVE_API_KEY: not set (web search disabled)
# ⚠️ HKASK_FMP_API_KEY: not set (company data disabled)
```

---

## 4. Step-by-Step: STANDARD Tier (Search + Financial Data + Backups)

### 4.1 Web Search Providers

Pick at least ONE. More providers = better search coverage via RRF (Reciprocal Rank Fusion).

| Provider | Free Tier | Setup Time | Notes |
|----------|-----------|-----------|-------|
| **Brave Search** | 2,000 queries/mo free | 5 min | Requires card on file. Best free tier. |
| **Firecrawl** | 500 credits/mo free | 2 min | Also enables web scraping + browsing. |
| **Tavily** | 1,000 queries/mo free | 2 min | AI-optimized, good for research agents. |
| **Exa** | Pay-per-use | 2 min | Semantic/neural search. |
| **SerpAPI** | 100 searches/mo free | 2 min | Google results. |
| **Browserbase** | Pay-per-use | 2 min | Headless browser for interactive browsing. |

**Setup (Brave Search example):**

1. Go to https://brave.com/search/api/
2. Click "Get Started" → create account
3. Go to https://api.search.brave.com/app/dashboard
4. Subscribe to a plan (Free includes 2,000/mo)
5. Copy API key → `.env`: `HKASK_BRAVE_API_KEY=your-key`

**Setup (Firecrawl example):**

1. Go to https://firecrawl.dev/
2. Sign up → go to Dashboard → API Keys
3. Copy key → `.env`: `HKASK_FIRECRAWL_API_KEY=your-key`

### 4.2 Financial Data Providers

| Provider | Free Tier | Setup Time | Coverage |
|----------|-----------|-----------|----------|
| **FMP** | 250 requests/day free | 5 min | US companies (best for US) |
| **EODHD** | 20 requests/day free | 5 min | Global markets (best for non-US) |

**Setup:**

1. FMP: https://financialmodelingprep.com/ → sign up → Dashboard → API key
2. EODHD: https://eodhd.com/ → register → API key
3. `.env`: `HKASK_FMP_API_KEY=your-key` and `HKASK_EODHD_API_KEY=your-key`

### 4.3 Adapter / Training Providers

| Provider | Purpose | Key Name |
|----------|---------|----------|
| **RunPod** | GPU inference + training | `RUNPOD_API_KEY` |
| **Baseten** | Model deployment | `BASETEN_API_KEY` |
| **HuggingFace** | Model/adapter access | `HF_TOKEN` |

**Setup:**

1. RunPod: https://runpod.io/ → Settings → API Keys → Read & Write key
2. Baseten: https://baseten.co/ → Settings → API Keys
3. HuggingFace: https://huggingface.co/settings/tokens → Create token (read)

---


### 4.4 Litestream Object Storage (Database Backups)

hKask uses Litestream for continuous SQLite backup to object storage. This is the canonical persistence strategy — used everywhere, not tied to any cloud provider. Choose one backend:

#### Backblaze B2 — Cheapest, Most Transparent

Why: $0.006/GB/mo (cheapest). 10GB free. Publishes drive failure statistics openly. Bandwidth Alliance with Cloudflare for free egress.

1. Go to https://www.backblaze.com/cloud-storage
2. Create account, then Create a Bucket (name: `hkask-pods-backup`)
3. App Keys, then Add a New Application Key
4. Copy keyID and applicationKey
5. `.env`:
   ```
   LITESTREAM_BUCKET=hkask-pods-backup
   LITESTREAM_ENDPOINT=https://s3.us-west-000.backblazeb2.com
   LITESTREAM_REGION=us-west-000
   LITESTREAM_ACCESS_KEY_ID=your-key-id
   LITESTREAM_SECRET_ACCESS_KEY=your-application-key
   LITESTREAM_FORCE_PATH_STYLE=true
   ```

#### Tigris — Zero Egress, Globally Distributed

Why: Built by Fly.io. Zero egress fees. Single endpoint serves all regions. Versioning and object lock built in. Best if your compute is on Fly.io.

1. Go to https://www.tigrisdata.com/ and sign up, or access via Fly.io dashboard
2. Create a bucket (name: `hkask-pods-backup`)
3. Create an Access Key from the Tigris Console
   - Access Key ID starts with `tid_`
   - Secret Access Key starts with `tsec_`
4. `.env`:
   ```
   LITESTREAM_BUCKET=hkask-pods-backup
   LITESTREAM_ENDPOINT=https://fly.storage.tigris.dev
   LITESTREAM_REGION=auto
   LITESTREAM_ACCESS_KEY_ID=tid_your_access_key
   LITESTREAM_SECRET_ACCESS_KEY=tsec_your_secret_key
   LITESTREAM_FORCE_PATH_STYLE=false
   ```

> **Endpoint note:** Tigris uses `https://fly.storage.tigris.dev` when accessed from Fly.io (zero egress within Fly.io network). For standalone Tigris outside Fly.io, use `https://t3.storage.dev`. Tigris uses virtual hosted-style addressing (`LITESTREAM_FORCE_PATH_STYLE=false`), unlike Backblaze B2 which uses path-style.

#### Hetzner Object Storage — EU Data Residency

Why: €5/TB/mo. Data stays in EU. Same provider as your K3s cluster (lower latency, simpler billing). 1TB free egress.

1. Go to https://console.hetzner.cloud/ , then Object Storage
2. Create a bucket (name: `hkask-pods-backup`, region: same as your K3s cluster)
3. Generate Access Key + Secret Key
4. `.env`:
   ```
   LITESTREAM_BUCKET=hkask-pods-backup
   LITESTREAM_ENDPOINT=https://nbg1.your-objectstorage.com
   LITESTREAM_REGION=nbg1
   LITESTREAM_ACCESS_KEY_ID=your-access-key
   LITESTREAM_SECRET_ACCESS_KEY=your-secret-key
   LITESTREAM_FORCE_PATH_STYLE=true
   ```

#### Decision Guide

| If you... | Choose |
|-----------|--------|
| Want the cheapest option and don't anticipate heavy egress | **Backblaze B2** |
| Run compute on Fly.io and want zero egress + global distribution | **Tigris** |
| Run K3s on Hetzner and need EU data residency | **Hetzner Object Storage** |
| Do frequent pod restores and worry about egress costs | **Tigris** or **Cloudflare R2** |
| Need air-gapped or self-hosted | **MinIO** |


## 5. Step-by-Step: FULL Tier (Cloud Deployment)

### 5.1 Container Registry

Push the built container image to a registry your cloud provider can pull from.

**GitHub Container Registry (recommended, free for public repos):**

```bash
# 1. Create a Personal Access Token (classic) with write:packages scope
#    https://github.com/settings/tokens

# 2. Login to GHCR
echo "$GITHUB_TOKEN" | docker login ghcr.io -u YOUR_USERNAME --password-stdin

# 3. Set in .env
CONTAINER_REGISTRY=ghcr.io/your-org/hkask
```

### 5.2 Cloud Provider: Fly.io

Fly.io has two token types relevant to hKask. Use the right one for each purpose:

| Token Type | Command | Scope | Use For |
|-----------|---------|-------|---------|
| **Org deploy token** | `fly tokens create org -o <org-slug>` | All apps in an org | Curator setup, pod lifecycle automation |
| **App deploy token** | `fly tokens create deploy -a <app-name>` | Single app only | CI/CD (GitHub Actions), per-app least-privilege |
| **Read-only token** | `fly tokens create readonly` | Read org-wide | Monitoring, health checks, CNS observability |

**Initial setup (Curator):**

```bash
# 1. Install flyctl
curl -L https://fly.io/install.sh | sh

# 2. Login (opens browser, creates personal auth session)
fly auth login

# 3. Create your organization (one-time)
fly orgs create hkask

# 4. Create an org-scoped deploy token for automation
#    This token can create/manage apps, volumes, secrets across the org.
#    Default 20-year expiry. Set shorter for production.
fly tokens create org -o hkask -n hkask-curator -x 87600h
#    Copy the FULL output including "FlyV1 " prefix.

# 5. Set in .env:
FLY_API_TOKEN=FlyV1 fm2_...
FLY_ORG_SLUG=hkask
```

**CI/CD token (per-pod, least privilege):**

```bash
# After pod creation, generate a deploy token scoped to that app only
fly tokens create deploy -a hkask-pod-my-first-pod -n cicd-token -x 87600h
```

> **Security principle:** The Curator uses an org token during setup and pod lifecycle (`kask pod create`, `kask pod activate`). CI/CD pipelines use app-scoped deploy tokens. Never use `fly auth token` (full personal token) in automation — it has unrestricted access to your account.

### 5.3 Cloud Provider: Hetzner Cloud

Hetzner is the cost leader (3-5x cheaper than Fly.io at scale) and the recommended choice for EU-regulated workloads (BSI C5, ISO 27001, GDPR-compliant by jurisdiction). It requires more operational setup than Fly.io but delivers substantial savings at scale.

**Step 1: Create Hetzner account and API token**

1. Go to https://console.hetzner.cloud/
2. Create project, then Security, then API Tokens, then Generate (Read and Write)
3. `.env`: `HCLOUD_TOKEN=your-token`

**Step 2: Create K3s cluster**

Choose one path:

*Path A: Self-managed K3s (cheapest)*

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

*Path B: Cloudfleet managed K8s (simpler, from free tier)*

1. Go to https://cloudfleet.ai/ and sign up (free Basic tier, up to 24 vCPUs)
2. Connect your Hetzner account (provide API token)
3. Create cluster, select Hetzner region, deploy
4. Download kubeconfig from Cloudfleet dashboard

**Step 3: Install cert-manager for TLS**

Unlike Fly.io, Hetzner does not auto-provision TLS. Use cert-manager:

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

### 5.4 Provider Decision: Fly.io vs Hetzner

| Factor | Fly.io | Hetzner + K3s |
|--------|--------|---------------|
| Setup time | 10 minutes | 30-60 minutes |
| Cost per pod | $2-10/mo | 4-6 EUR/mo |
| Scale-to-zero | Yes (under 300ms) | No (always-on) |
| TLS | Auto-provisioned | Manual (cert-manager) |
| Global regions | 35+ | 6 |
| Compliance | SOC2, HIPAA | ISO 27001, BSI C5, GDPR |
| GPU | Deprecated (Aug 2026) | None |
| Ops burden | Low (PaaS) | Medium-High (K8s) |
| Best for | Global, bursty, fast setup | EU-regulated, cost-sensitive, always-on |

Recommendation: Start with Fly.io for fast time-to-deploy. Add Hetzner as secondary for cost optimization and EU compliance. Both share the same Litestream object storage bucket so migrating pods requires no data migration.

### 5.5 Matrix / Conduit (Curator-Managed)

The Matrix Conduit server is **set up once by the Curator** during system installation. Pod admins do not configure it directly. What you need to know:

- **`HKASK_MATRIX_URL`** — set by the Curator. Points to the shared Conduit homeserver (Model B) or `http://localhost:8008` for per-pod Conduit (Model A). This is the only Matrix-related value in your `.env`.
- **Agent credentials** (`HKASK_MATRIX_AGENT_USERNAME` / `HKASK_MATRIX_AGENT_PASSWORD`) — auto-generated by `kask pod create` and stored as Fly Secrets. Never manually configured.
- **Conduit signing key** (`CONDUIT_MATRIX_SIGNING_KEY`) — owned by the Curator, stored in the Curator's keystore. This is a system-level secret, not a per-pod setting.

**Curator setup (for reference):**

```bash
# Curator initializes the system (one-time):
kask curator init --domain hkask.your-domain.com
# This generates the Conduit signing key, stores it in the Curator's keystore,
# and deploys the shared Conduit Fly App (Model B).
# The --domain flag sets the Matrix server_name (e.g., hkask.your-domain.com).

# Curator creates a pod (per-pod):
kask pod create my-first-pod
# This auto-generates Matrix agent credentials and stores them as pod secrets.

# Pod admin deploys:
kask pod export fly my-first-pod
# Credentials are baked into the generated fly-secrets.sh — nothing to configure.
```

### 5.6 DNS Configuration

DNS setup depends on your cloud provider. Do this BEFORE deploying (DNS propagation takes time).

#### Fly.io DNS

```bash
# 1. Set your domain
HKASK_BASE_URL=https://hkask.your-domain.com

# 2. Add DNS records at your registrar:
#    Type: A     Name: hkask     Value: <Fly.io IPv4>     (from: fly ips list)
#    Type: AAAA  Name: hkask     Value: <Fly.io IPv6>     (from: fly ips list)

# 3. Certify Fly.io to handle TLS:
fly certs create hkask.your-domain.com

# 4. Verify DNS:
dig hkask.your-domain.com +short
```

#### Hetzner DNS

```bash
# 1. Set your domain
HKASK_BASE_URL=https://hkask.your-domain.com

# 2. Deploy the K8s ingress first (get the Load Balancer IP):
kask pod export k8s my-first-pod
kubectl apply -f k8s-manifests/
kubectl get svc -n hkask-pod-my-first-pod
# Note the EXTERNAL-IP of the LoadBalancer service

# 3. Add DNS records at your registrar:
#    Type: A     Name: hkask     Value: <LoadBalancer EXTERNAL-IP>

# 4. Verify DNS:
dig hkask.your-domain.com +short

# 5. Verify TLS (cert-manager auto-provisions after DNS resolves):
curl https://hkask.your-domain.com/health
```

> **DNS propagation note:** DNS changes can take up to 48 hours to propagate globally, though most resolvers update within 15-30 minutes. Start DNS setup early.

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

# Tag doesn't need docker pull — Fly.io and K8s pull from registry at deploy time.
# Just reference the image in your deployment files and deploy.
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

### 6.4 Deploy to Fly.io

```bash
# Generate deployment files for a pod
kask pod export fly my-first-pod \
  --region iad \
  --volume-size-gb 3

# This creates:
#   fly.toml           — Fly.io app configuration
#   fly-secrets.sh     — secrets to set (run once)

# Set secrets
source fly-secrets.sh

# Deploy (Fly.io pulls the image from your registry)
fly deploy --config fly.toml

# Verify
fly status
fly logs
```

### 6.5 Deploy to Hetzner K3s

```bash
# Generate K8s manifests for a pod
kask pod export k8s my-first-pod \
  --volume-size-gb 10 \
  --max-replicas 3

# This creates:
#   k8s-manifests/namespace.yaml
#   k8s-manifests/networkpolicy.yaml
#   k8s-manifests/statefulset.yaml
#   k8s-manifests/configmap.yaml
#   k8s-manifests/secrets.yaml
#   k8s-manifests/hpa.yaml

# Deploy
kubectl apply -f k8s-manifests/

# Verify
kubectl get pods -n hkask-pod-my-first-pod
kubectl logs -n hkask-pod-my-first-pod statefulset/kask
```

### 6.6 Verify Deployment

```bash
# Check pod health (both providers)
curl https://hkask.your-domain.com/health
# Expected: {"status":"ok","pod_id":"my-first-pod"}

# Check Litestream replication (Fly.io)
fly ssh console -C "litestream generations /data/kask.db"

# Check Litestream replication (Hetzner)
kubectl exec -n hkask-pod-my-first-pod statefulset/kask -c litestream -- \
  litestream generations /data/kask.db
# Expected: list of generations in object storage

# Check Conduit federation (both providers)
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
  ✅ FLY_API_TOKEN        — Fly.io (valid, org: my-org)
  ⚠️ HCLOUD_TOKEN         — not set

  Storage
  ✅ LITESTREAM_BUCKET — Backblaze B2 (valid, 3 generations found)

  Matrix
  ✅ HKASK_MATRIX_URL     — http://localhost:8008 (Conduit reachable)
  ✅ Matrix agent credentials — auto-generated by kask pod create

  Summary: 9/14 providers configured, 1 invalid, 4 missing
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
| `fly deploy: authentication failed` | Token expired | `fly auth login`, update `FLY_API_TOKEN` |
| `conduit: federation rejected` | Wrong signing key or DNS | Check `CONDUIT_MATRIX_SIGNING_KEY`, verify `<app>.fly.dev:8448` is reachable |
| Container build fails (out of memory) | Rust + Conduit both compile | Increase Docker memory limit to 4GB, or build in CI |
| Pod starts but no inference | Model not available on chosen provider | Check provider's model list, try `DI/Qwen/Qwen2.5-7B-Instruct` |

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

# --- Deploy (Fly.io) ---
kask pod export fly my-pod                    # Generate fly.toml + secrets
source fly-secrets.sh                         # Set secrets
fly deploy --config fly.toml                  # Fly.io pulls image from registry
fly logs                                      # Watch logs

# --- Deploy (Hetzner K3s) ---
kask pod export k8s my-pod                    # Generate K8s manifests
kubectl apply -f k8s-manifests/               # Deploy to cluster
kubectl logs -n hkask-pod-my-pod statefulset/kask

# --- Verify (both providers) ---
curl https://hkask.your-domain.com/health     # Health check
# Litestream: fly ssh console / kubectl exec — litestream generations
# Conduit:    curl :8448/_matrix/federation/v1/version
```

---

## 11. References

- [`.env.example`](../../.env.example) — Complete environment variable template. Provider API keys configured via `FW_API_KEY`, `DI_API_KEY`, etc.
- [`docs/research/cloud-implementation-plans.md`](../research/cloud-implementation-plans.md) — Detailed cloud architecture
- [`docs/research/cloud-deployment-research-report.md`](../research/cloud-deployment-research-report.md) — Provider evaluation
- [Fly.io Machines API](https://fly.io/docs/machines/api/)
- [Litestream Documentation](https://litestream.io/)
- [Conduit Matrix Server](https://conduit.rs/)
