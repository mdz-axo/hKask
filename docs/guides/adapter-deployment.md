---
title: "hKask Adapter Deployment Guide"
audience: [DevOps engineers, ML engineers, system administrators]
last_updated: 2026-06-17
version: "0.28.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle, composition, trust]
---

# hKask Adapter Deployment Guide

**Purpose:** Deploy the hKask training MCP server with adapter lifecycle management — trained LoRA adapters served through Together AI, Runpod, or Baseten.

**Principle:** A trained adapter is inert bits on disk. Without composition logic, provider routing, and lifecycle management, it cannot become a usable inference surface. This guide makes it actionable.

---

## Contents

| Section | Description |
|---------|-------------|
| [§1 Architecture](#1-architecture) | How the adapter system fits into hKask |
| [§2 Prerequisites](#2-prerequisites) | What you need before starting |
| [§3 Environment Configuration](#3-environment-configuration) | All required environment variables |
| [§4 Quick Start](#4-quick-start) | Minimal setup to first deployment |
| [§5 Provider-Specific Setup](#5-provider-specific-setup) | Together AI, Runpod, Baseten configuration |
| [§6 Adapter Lifecycle](#6-adapter-lifecycle) | End-to-end flow: train → store → deploy → infer → teardown |
| [§7 Production Hardening](#7-production-hardening) | Security, persistence, monitoring |
| [§8 Troubleshooting](#8-troubleshooting) | Common issues and resolutions |

---

## 1. Architecture

```
Training Pipeline                    Deployment Pipeline
─────────────────                    ───────────────────
SKILL.md                              LoRAAdapter
   ↓                                     ↓ to_canonical()
training_generate_traces              TrainedLoRAAdapter (hkask-adapter)
   ↓                                     ↓
ChatML JSONL                           AdapterStore (SQLite)
   ↓                                     ↓
training_submit                        AdapterRouter (AdapterPort trait)
   ↓                                     ├── select_provider()  ← P2 consent
TrainingHost                           ├── create_endpoint()   ← real HTTP
   ↓                                     ├── infer()            ← chat completions
LoRAAdapter                            └── teardown_endpoint() ← RAII
   ↓
training_register_adapter
   ↓
hkask-storage (metadata + blob)
   ↓
training_deploy → AdapterRouter
```

**Key insight:** The training pipeline produces adapters. The deployment pipeline makes them actionable. Both use the same canonical type (`TrainedLoRAAdapter`) through the `to_canonical()` conversion on `LoRAAdapter`.

**Three providers, one interface:**

| Provider | Upload | Provision | Inference | Teardown |
|----------|--------|-----------|-----------|----------|
| **Together AI** | HTTP POST to models API | Auto-deployed after upload | `POST /v1/chat/completions` | Auto-expires |
| **Runpod** | vLLM `--lora-modules` | Serverless template | `POST /openai/v1/chat/completions` | Best-effort DELETE |
| **Baseten** | vLLM `--lora-modules` | Model API | `POST /v1/chat/completions` | HTTP DELETE |

---

## 2. Prerequisites

### Required Software

- **Rust toolchain** — `rustc 1.85+`, `cargo`
- **SQLite** — with SQLCipher for encrypted storage (optional, recommended for production)
- **Hugging Face account** — for hosting trained adapters (public or private)
- **At least one provider account** — Together AI, Runpod, or Baseten

### Required Credentials

| Variable | Purpose | Required For |
|----------|---------|-------------|
| `TOGETHER_API_KEY` | Together AI API authentication | Together AI deployment + inference |
| `RUNPOD_API_KEY` | Runpod API authentication | Runpod deployment |
| `RUNPOD_TEMPLATE_ID` | Runpod serverless vLLM template | Runpod deployment |
| `BASETEN_API_KEY` | Baseten API authentication | Baseten deployment |
| `HF_TOKEN` | HuggingFace authentication | Private/gated adapter repos on Together AI |
| `HKASK_MEMORY_DB` | Path to encrypted training database | Persistent adapter storage |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase | Persistent adapter storage |

### Adapter Requirements

Each adapter must:
1. Be hosted on **Hugging Face Hub** (public, private, or gated)
2. Contain `adapter_config.json` and `adapter_model.safetensors`
3. Target a **base model supported by the chosen provider**

---

## 3. Environment Configuration

Create a `.env` file in the project root (or export these variables):

```bash
# ── Required: Database ──────────────────────────────────────────
HKASK_MEMORY_DB=/var/lib/hkask/training.db
HKASK_DB_PASSPHRASE=<your-db-passphrase>

# ── Required: At least one provider ─────────────────────────────
TOGETHER_API_KEY=<your-together-key>
RUNPOD_API_KEY=<your-runpod-key>
RUNPOD_TEMPLATE_ID=<your-runpod-template-id>
BASETEN_API_KEY=<your-baseten-key>

# ── Optional: HuggingFace ───────────────────────────────────────
HF_TOKEN=<your-hf-token>          # For private/gated repos
HKASK_TRAINING_CACHE_DIR=/tmp/hkask-training-cache
HKASK_TRAINING_HOST=together      # Default training host
HKASK_TRAINING_HARNESS=axolotl    # Default training harness
```

---

## 4. Quick Start

### 4.1 Build

```bash
cargo build --release -p hkask-mcp-training
```

### 4.2 Start the Server

```bash
source .env
cargo run --release -p hkask-mcp-training
```

The server starts and:
1. Opens (or creates) the encrypted training database
2. Runs schema migrations for adapter storage and active endpoints
3. Initializes the `AdapterRouter` with all three provider backends
4. Logs any orphaned endpoints from previous sessions
5. Registers all 22 MCP tools

### 4.3 Verify

```bash
# List available tools
kask tool list | grep training_

# Should show: training_deploy, training_deployment_status, training_teardown,
#              training_list_adapters, training_register_adapter, etc.
```

### 4.4 Deploy Your First Adapter

```bash
# Via CLI (delegates to MCP)
kask adapter deploy my-adapter --provider together

# Or directly via MCP tool
kask tool call training_deploy --params '{"adapter_name": "my-adapter", "provider": "Together"}'

# Check status
kask tool call training_deployment_status --params '{"deployment_id": "<endpoint-id>"}'

# Tear down when done
kask tool call training_teardown --params '{"deployment_id": "<endpoint-id>"}'
```

---

## 5. Provider-Specific Setup

### 5.1 Together AI

**How it works:** Adapters are uploaded from Hugging Face Hub to Together AI. They're auto-deployed as dedicated endpoints. Inference uses the standard chat completions API with the adapter's model name.

**Setup:**
1. Create a Together AI account at [together.ai](https://together.ai)
2. Generate an API key from the dashboard
3. Set `TOGETHER_API_KEY` in your environment
4. For private adapter repos, also set `HF_TOKEN` with a HuggingFace access token

**Endpoints used:**
- Upload: `POST https://api.together.ai/v1/models` (async, polls `GET /v1/jobs/{id}`)
- Inference: `POST https://api.together.ai/v1/chat/completions`
- Docs: [docs.together.ai/docs/dedicated-endpoints/adapter](https://docs.together.ai/docs/dedicated-endpoints/adapter)

**Cost:** ~$1.10/hr for Llama-3.3-70B dedicated endpoint. Adapters don't incur cost when idle.

### 5.2 Runpod

**How it works:** Runpod serverless endpoints run vLLM with `--enable-lora`. Adapters are loaded at server start via `--lora-modules name=path`. The serverless endpoint auto-scales to zero when idle.

**Setup:**
1. Create a Runpod account at [runpod.io](https://runpod.io)
2. Generate an API key from Settings → API Keys
3. Create a serverless vLLM template at [console.runpod.io/serverless](https://console.runpod.io/serverless)
4. Set `RUNPOD_API_KEY` and `RUNPOD_TEMPLATE_ID`

**Endpoints used:**
- Inference: `POST https://api.runpod.ai/v2/{template_id}/openai/v1/chat/completions`
- Docs: [docs.runpod.io/serverless/endpoints/manage-endpoints](https://docs.runpod.io/serverless/endpoints/manage-endpoints)

**⚠️ Teardown:** Runpod serverless endpoint deletion is console-only — no REST API. The system logs a warning and you must delete manually at `console.runpod.io/serverless`.

**Cost:** ~$0.79/hr for comparable GPU. Serverless scales to zero when idle.

### 5.3 Baseten

**How it works:** Baseten deploys models as Truss containers with vLLM. The model ID is used to construct the endpoint URL. Inference uses the OpenAI-compatible API.

**Setup:**
1. Create a Baseten account at [baseten.co](https://baseten.co)
2. Generate an API key from Settings → API Keys
3. Set `BASETEN_API_KEY`

**Endpoints used:**
- Inference: `POST https://model-{id}.api.baseten.co/v1/chat/completions`
- Docs: [docs.baseten.co/api-reference](https://docs.baseten.co/api-reference)

**Cost:** ~$0.85/hr for comparable GPU.

---

## 6. Adapter Lifecycle

### 6.1 End-to-End Flow

```
1. TRAIN
   kask tool call training_submit --params '{...}'
   → TrainingJob dispatched to provider
   → On completion, auto-registered as LoRAAdapter

2. STORE
   Adapter metadata stored in SQLite (lora_adapters table)
   Adapter weights stored as blob (lora_blobs table)
   → training_list_adapters to verify

3. SELECT (P2 — informed consent)
   kask adapter deploy <name> --provider together
   → AdapterRouter::select_provider() returns cost estimates
   → User confirms provider selection

4. DEPLOY
   AdapterRouter::create_endpoint()
   → to_canonical() converts LoRAAdapter → TrainedLoRAAdapter
   → upload_adapter() sends to provider
   → provision_endpoint() creates running endpoint
   → EndpointLifecycle starts tracking cost

5. INFER
   AdapterRouter::infer()
   → Endpoint transitions to Active
   → HTTP POST to provider's chat completions API
   → Cost accrues per token

6. TEARDOWN
   AdapterRouter::teardown_endpoint()
   → Endpoint transitions to Draining → Terminated
   → Provider API called to release resources
   → EndpointGuard (RAII) ensures no leaks
```

### 6.2 Cost Tracking

Every endpoint has an `EndpointLifecycle` state machine that:
- Accrues cost per second in billable phases (Provisioning, Ready, Active)
- Tracks `hourly_rate` from the provider's `CostModel`
- Exposes `is_over_budget(limit)` and `time_until_budget_exceeded(limit)`
- Emits CNS spans on every phase transition

### 6.3 Budget Enforcement

```bash
# Deploy with budget awareness
kask tool call training_deploy --params '{"adapter_name": "my-adapter", "provider": "Together"}'
# Response includes estimated_setup_cost and estimated_hourly_cost

# Check if budget exceeded (programmatic)
# EndpointLifecycle::is_over_budget(5.00) → true if cost > $5.00
```

---

## 7. Production Hardening

### 7.1 Database Encryption

```bash
# Generate a strong passphrase
openssl rand -base64 32

# Set in environment
HKASK_DB_PASSPHRASE=<generated-passphrase>
HKASK_MEMORY_DB=/var/lib/hkask/training.db
```

The database uses SQLCipher with AES-256-CBC encryption. Passphrases are derived using Argon2id to produce 256-bit encryption keys.

### 7.2 Orphaned Endpoint Detection

On startup, the `AdapterRouter` queries the `active_endpoints` table for endpoints that were active when the system last shut down. Each orphaned endpoint is logged with:

```
WARN  Orphaned endpoint — may need manual teardown via provider console
      endpoint_id=<uuid> provider=Together model=adapter-<id>
      expertise=solidity-audit phase=active cost=3.45
```

**Action required:** Check the corresponding provider console and manually delete any endpoints that should no longer be running.

### 7.3 Security Considerations

| Concern | Mitigation |
|---------|-----------|
| API keys in environment | Load from OS keychain where available; `.env` for development |
| Adapter access control | `AdapterPort` trait methods are OCAP-gated via `DelegationToken` |
| Endpoint resource leaks | `EndpointGuard` (RAII) + orphan detection on startup |
| Cross-user adapter access | Sovereign-scoped by WebID owner; sharing requires explicit consent |
| Provider API key exposure | Keys used only in-memory via `reqwest::blocking::Client`; never logged |

### 7.4 Systemd Service

```ini
# /etc/systemd/system/hkask-training.service
[Unit]
Description=hKask Training MCP Server
After=network.target

[Service]
Type=simple
User=hkask
WorkingDirectory=/opt/hkask
EnvironmentFile=/etc/hkask/training.env
ExecStart=/opt/hkask/target/release/hkask-mcp-training
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### 7.5 Docker

```dockerfile
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p hkask-mcp-training

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/hkask-mcp-training /usr/local/bin/
ENV HKASK_MEMORY_DB=/data/training.db
VOLUME /data
ENTRYPOINT ["hkask-mcp-training"]
```

---

## 8. Troubleshooting

| Symptom | Likely Cause | Resolution |
|---------|-------------|------------|
| "TOGETHER_API_KEY not set" | Missing environment variable | `export TOGETHER_API_KEY=<key>` or add to `.env` |
| "Adapter not found by ID or skill name" | Adapter not registered | Run `training_list_adapters` to see available adapters |
| "Provider unavailable for adapter composition" | Provider not in backend registry | Verify provider is configured (Together/Runpod/Baseten) |
| "Base model incompatibility" | Adapter trained on unsupported model | Check `ProviderCapability::supported_base_model_families` |
| "No huggingface_repo — skipping upload" | Adapter source not set | Set `AdapterSource::HuggingFace { repo }` on the adapter |
| "Together AI upload returned 401" | Invalid or expired API key | Regenerate at [together.ai](https://together.ai) |
| "Runpod teardown failed (may require console deletion)" | Runpod teardown is console-only | Delete manually at [console.runpod.io/serverless](https://console.runpod.io/serverless) |
| Orphaned endpoint warning on startup | Previous session ended without teardown | Check provider console, manually delete if needed |
| Upload job stuck polling for 5+ minutes | Provider is slow or unreachable | Check provider status page; increase poll timeout |

---

*"The Analytical Engine weaves algebraical patterns just as the Jacquard loom weaves flowers and leaves." — Ada Lovelace, 1843*

*The adapter system weaves trained expertise into inference endpoints. Each adapter is a pattern. Each endpoint is a loom. The composition is the weaving.*
