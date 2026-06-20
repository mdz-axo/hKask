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
| **STANDARD** | + 2 search providers + 1 financial data | Web search, company research, adapter training |
| **FULL** | + Cloud provider + S3 storage + Matrix | Cloud deployment, database backups, cross-pod federation |

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

## 4. Step-by-Step: STANDARD Tier (Search + Financial Data)

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

### 5.2 Litestream S3 Storage (Database Backups)

hKask uses Litestream for continuous SQLite backup to S3-compatible storage.

**Backblaze B2 (recommended, 10GB free):**

1. Go to https://www.backblaze.com/cloud-storage
2. Create account → Create a Bucket (name: `hkask-pods-backup`)
3. Go to App Keys → Add a New Application Key
4. Copy keyID and applicationKey
5. `.env`:
   ```
   LITESTREAM_S3_BUCKET=hkask-pods-backup
   LITESTREAM_S3_ENDPOINT=https://s3.us-west-000.backblazeb2.com
   LITESTREAM_S3_REGION=us-west-000
   LITESTREAM_ACCESS_KEY_ID=your-key-id
   LITESTREAM_SECRET_ACCESS_KEY=your-application-key
   LITESTREAM_FORCE_PATH_STYLE=true
   ```

**Cloudflare R2 (alternative, 10GB free, no egress fees):**

1. Go to https://dash.cloudflare.com/ → R2 → Create bucket
2. Manage R2 API Tokens → Create token (Edit permissions)
3. `.env`: `LITESTREAM_S3_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com`

### 5.3 Cloud Provider: Fly.io

1. Install flyctl: `curl -L https://fly.io/install.sh | sh`
2. `fly auth login` (opens browser)
3. Get token: `fly auth token`
4. `.env`: `FLY_API_TOKEN=your-token`
5. Get org slug: `fly orgs list`
6. `.env`: `FLY_ORG_SLUG=your-org-slug`

### 5.4 Cloud Provider: Hetzner Cloud (Optional Secondary)

1. Go to https://console.hetzner.cloud/
2. Create project → Security → API Tokens → Generate
3. `.env`: `HCLOUD_TOKEN=your-token`

### 5.5 Matrix / Conduit (Curator-Managed)

The Matrix Conduit server is **set up once by the Curator** during system installation. Pod admins do not configure it directly. What you need to know:

- **`HKASK_MATRIX_URL`** — set by the Curator. Points to the shared Conduit homeserver (Model B) or `http://localhost:8008` for per-pod Conduit (Model A). This is the only Matrix-related value in your `.env`.
- **Agent credentials** (`HKASK_MATRIX_AGENT_USERNAME` / `HKASK_MATRIX_AGENT_PASSWORD`) — auto-generated by `kask pod create` and stored as Fly Secrets. Never manually configured.
- **Conduit signing key** (`CONDUIT_MATRIX_SIGNING_KEY`) — owned by the Curator, stored in the Curator's keystore. This is a system-level secret, not a per-pod setting.

**Curator setup (for reference):**

```bash
# Curator initializes the system (one-time):
kask curator init --matrix-domain hkask.local
# This generates the Conduit signing key and stores it in the Curator's keystore.
# It also deploys the shared Conduit Fly App (Model B).

# Curator creates a pod (per-pod):
kask pod create my-first-pod
# This auto-generates Matrix agent credentials and stores them as pod secrets.

# Pod admin deploys:
kask pod export fly my-first-pod
# Credentials are baked into the generated fly-secrets.sh — nothing to configure.
```

---

## 6. Build and Deploy

### 6.1 Build the Container

```bash
# Build the Docker image (includes kask + Litestream + Conduit)
docker build -t kask:0.30.0 .

# Tag for registry
docker tag kask:0.30.0 ghcr.io/your-org/hkask:kask-0.30.0

# Push
docker push ghcr.io/your-org/hkask:kask-0.30.0
```

### 6.2 Deploy to Fly.io

```bash
# Generate deployment files for a pod
kask pod export fly my-first-pod \
  --region iad \
  --volume-size-gb 3

# This creates:
#   fly.toml           — Fly.io app configuration
#   fly-secrets.sh     — secrets to set (run once)
#   deploy.sh          — deployment script

# Set secrets
source fly-secrets.sh

# Deploy
fly deploy --config fly.toml

# Verify
fly status
fly logs
```

### 6.3 Verify Deployment

```bash
# Check pod health
curl https://hkask-pod-my-first-pod.fly.dev/health
# Expected: {"status":"ok","pod_id":"my-first-pod"}

# Check Litestream replication
fly ssh console -C "litestream generations /data/kask.db"
# Expected: list of generations in S3

# Check Conduit federation (if Matrix configured)
curl https://hkask-pod-my-first-pod.fly.dev:8448/_matrix/federation/v1/version
# Expected: {"server":{"name":"pod-my-first-pod.hkask.local","version":"..."}}
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
  ✅ LITESTREAM_S3_BUCKET — Backblaze B2 (valid, 3 generations found)

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
| `litestream: No replica found` | S3 bucket not created or wrong credentials | Check `LITESTREAM_S3_*` vars, create bucket |
| `fly deploy: authentication failed` | Token expired | `fly auth login`, update `FLY_API_TOKEN` |
| `conduit: federation rejected` | Wrong signing key or DNS | Check `CONDUIT_MATRIX_SIGNING_KEY`, verify `<app>.fly.dev:8448` is reachable |
| Container build fails (out of memory) | Rust + Conduit both compile | Increase Docker memory limit to 4GB, or build in CI |
| Pod starts but no inference | Model not available on chosen provider | Check provider's model list, try `DI/Qwen/Qwen2.5-7B-Instruct` |

---

## 10. Quick Reference Card

```bash
# ─── First Time Setup ───────────────────────────────────────
cp .env.example .env                          # Create config
# Edit .env with at least one inference key
kask setup wizard                             # Interactive setup (Phase 2)
kask doctor                                   # Validate configuration

# ─── Build ───────────────────────────────────────────────────
docker build -t kask:0.30.0 .                 # Build container
docker tag kask:0.30.0 ghcr.io/org/hkask:kask-0.30.0
docker push ghcr.io/org/hkask:kask-0.30.0

# ─── Deploy (Fly.io) ────────────────────────────────────────
kask pod export fly my-pod                    # Generate deployment files
source fly-secrets.sh                         # Set secrets
fly deploy --config fly.toml                  # Deploy to Fly.io
fly logs                                      # Watch logs

# ─── Verify ──────────────────────────────────────────────────
curl https://hkask-pod-my-pod.fly.dev/health  # Health check
fly ssh console -C "litestream generations /data/kask.db"
```

---

## 11. References

- [`.env.example`](../../.env.example) — Complete environment variable template
- [`docs/guides/provider-api-keys.md`](./provider-api-keys.md) — Per-provider API key setup (to be created)
- [`docs/research/cloud-implementation-plans.md`](../research/cloud-implementation-plans.md) — Detailed cloud architecture
- [`docs/research/cloud-deployment-research-report.md`](../research/cloud-deployment-research-report.md) — Provider evaluation
- [Fly.io Machines API](https://fly.io/docs/machines/api/)
- [Litestream Documentation](https://litestream.io/)
- [Conduit Matrix Server](https://conduit.rs/)
