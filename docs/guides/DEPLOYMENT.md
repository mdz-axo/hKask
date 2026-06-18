---
title: "hKask Deployment Guide"
audience: [DevOps engineers, system administrators, deployment teams]
last_updated: 2026-06-17
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle]
---

# hKask Deployment Guide

---

## Contents

| Section | Description |
|---------|-------------|
| [§1 Overview](#1-overview) | Platform components and key features |
| [§2 Prerequisites](#2-prerequisites) | System requirements and dependencies |
| [§3 Quick Start](#3-quick-start) | Minimal setup to get running |
| [§4 Configuration](#4-configuration) | Environment, database, LLM, and API config |
| [§5 Production Deployment](#5-production-deployment) | Full production setup with systemd and TLS |
| [§6 Health Checks](#6-health-checks) | Liveness and readiness endpoints |
| [§7 Monitoring](#7-monitoring) | CNS metrics and alerting |
| [§8 Backup & Recovery](#8-backup--recovery) | Database backup and restore procedures |
| [§9 Security Hardening](#9-security-hardening) | Production security checklist |
| [§10 Troubleshooting](#10-troubleshooting) | Common issues and resolutions |
| [§11 Upgrade Path](#11-upgrade-path) | Version migration instructions |
| [§12 API Reference](#12-api-reference) | HTTP API endpoint reference |

---

## 1. Overview

hKask (ℏKask - "A Minimal Viable Container for Agents") is a minimal agent-native container platform. This guide covers production deployment of the `kask` binary and supporting infrastructure.

**Components:**
- `kask` binary — Single binary (daemon, API, MCP servers, agents)
- Caddy (Docker sidecar) — TLS termination, reverse proxy
- Conduit (Docker sidecar) — Matrix homeserver for agent communication
- SQLCipher-encrypted SQLite — Persistent storage for all user data
- Inference Router — Multi-provider cloud LLM inference (DeepInfra, Together AI, fal.ai, RunPod, Baseten)

**Key Features:**
- Browser terminal (xterm.js + WebSocket) — primary user access via OAuth sign-in
- API-based chat endpoint (`POST /api/chat`)
- Multi-tenant with Admin/Member roles
- Template registry with hLexicon validation
- Agent pod management (bot/replicant lifecycle)
- CNS monitoring with algedonic alerts
- User sovereignty enforcement (Magna Carta)
- Portable encrypted backup via `kask backup export`

---

## 2. Prerequisites

### 2.1 System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| **OS** | Linux (kernel 5.4+) | Linux (Ubuntu 22.04+, RHEL 9+) |
| **CPU** | 2 cores | 4+ cores |
| **RAM** | 2 GB | 4+ GB |
| **Disk** | 20 GB | 20+ GB SSD |
| **Rust** | 1.91+ | Latest stable |

### 2.2 External Dependencies

| Dependency | Purpose | Required | Default |
|------------|---------|----------|---------|
| **DeepInfra** | Cloud LLM inference | Optional (requires API key) | `https://api.deepinfra.com/v1/openai` |
| **Together AI** | Cloud LLM inference | Optional (requires API key) | `https://api.together.xyz/v1` |
| **fal.ai** | Cloud LLM inference | Optional (requires API key) | `https://fal.ai/api` |
| **RunPod** | Cloud LLM inference | Optional (requires API key) | `https://api.runpod.io/v2` |
| **Baseten** | Cloud LLM inference | Optional (requires API key) | `https://api.baseten.co/v1` |
| **SQLite** | Database engine | Bundled (rusqlite) | — |
| **Git** | Template loading (optional) | Optional | — |

### 2.3 Inference Provider Setup

hKask uses a multi-provider cloud inference router. Supported providers: DeepInfra (DI), Together AI (TG), fal.ai (FA), RunPod (RP), Baseten (BT).

```bash
# Configure DeepInfra API key
export DEEPINFRA_API_KEY="your-api-key-here"

# Optional: override default base URL
export DEEPINFRA_BASE_URL="https://api.deepinfra.com/v1/openai"
```

---

## 3. Quick Start

### 3.1 Admin Setup

For a full step-by-step server deployment including OAuth, Caddy + Conduit sidecars, DNS, and first sign-in, see the **[Admin Install Guide](admin-install-guide.md)**.

Quick start (development only):
```bash
cargo build --release --bin kask
cp target/release/kask /usr/local/bin/kask
kask init --profile dev
kask daemon start
```

### 3.2 Browser Terminal (Primary Access)

Users access hKask entirely through a browser. No binary to install, no SSH setup.

```
https://hkask.your-domain.com
  │
  ├── /login       — OAuth sign-in (GitHub / Google)
  ├── /terminal    — xterm.js terminal via WebSocket → kask repl
  └── /api/v1/*    — REST API
```

After OAuth sign-in, the browser opens a WebSocket-connected xterm.js terminal
running `kask repl`. All standard CLI operations are available:

```bash
# Inside browser terminal (kask repl)
kask> chat --interactive
kask> template list
kask> cns health
kask> sovereignty status
kask> pod list
```

### 3.3 API Server

The API server is part of the single `kask` binary and starts with the daemon:

```bash
# Test chat endpoint
curl -X POST https://hkask.your-domain.com/api/chat \
  -H "Content-Type: application/json" \
  -d '{"input": "What is the capital of France?", "template_id": null}'
```

### 3.4 Verify Installation

```bash
# Check CNS health
curl -s https://hkask.your-domain.com/api/cns/health | jq

# List templates
kask template list

# Check sovereignty status
kask sovereignty status

# Verify sidecars
kask matrix status-sidecar
```

---

## 4. Configuration

### 4.1 Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DI_BASE_URL` | DeepInfra base URL | `https://api.deepinfra.com` | No |
| `DI_API_KEY` | DeepInfra API key (also `DEEPINFRA_API_KEY`) | — | For DI provider |
| `FA_API_KEY` | fal.ai API key | — | For fal.ai provider |
| `HKASK_DATABASE_URL` | SQLite database path | `./data/hkask.db` | No |
| `HKASK_LOG_LEVEL` | Logging verbosity | `info` | No |
| `RUST_LOG` | Rust tracing filter | — | No |
| `HKASK_SOAP_MODEL` | Model for SOAP inference | `qwen3:8b` | No |
| `HKASK_SOAP_TEMPERATURE` | Temperature for SOAP | `0.2` | No |
| `HKASK_SOAP_MAX_TOKENS` | Max tokens for SOAP | `2048` | No |
| `HKASK_SOAP_TIMEOUT_SECS` | SOAP inference timeout | `30` | No |
| `HKASK_SOAP_PERSONA_PATH` | Jack persona file path | `hkask-templates/personas/jack-nurse.md` | No |

Model names use 2-letter provider prefixes for routing:
- `DI/` → DeepInfra (cloud) — requires `DI_API_KEY`
- `FA/` → fal.ai (cloud) — requires `FA_API_KEY`
- No prefix → defaults to DeepInfra

API keys can be set in environment variables or in a `providers.env` file. The `kask` binary auto-loads `.env` on startup via `dotenvy`. For OAuth credentials (GitHub/Google client IDs and secrets), see the [Admin Install Guide](admin-install-guide.md).

### 4.2 Chat Configuration

Chat endpoints use the following default LLM parameters:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `temperature` | 0.7 | Balanced creativity/coherence |
| `top_p` | 0.9 | Nucleus sampling threshold |
| `top_k` | 40 | Token sampling limit |
| `max_tokens` | 512 | Maximum response length |
| `frequency_penalty` | 0.0 | No repetition penalty |
| `presence_penalty` | 0.0 | No novelty bonus |

To customize chat behavior, modify the source code in:
- CLI: `crates/hkask-cli/src/main.rs:process_chat_input_async()`
- API: `crates/hkask-api/src/routes.rs:chat()`

### 4.3 Example Configuration

```bash
# Production environment
export DI_API_KEY="your-deepinfra-key"
export HKASK_DATABASE_URL="/var/lib/hkask/hkask.db"
export HKASK_LOG_LEVEL="warn"
export RUST_LOG="hkask=info,hyper=warn"

# Custom model configuration
export HKASK_SOAP_MODEL="qwen3:32b"
export HKASK_SOAP_TEMPERATURE="0.3"
export HKASK_SOAP_MAX_TOKENS="4096"
```

### 4.4 Database Location

hKask stores all data on the cloud server. The default database path is set during `kask init --profile server`.

```bash
mkdir -p /var/lib/hkask
export HKASK_DATABASE_URL="/var/lib/hkask/hkask.db"
```

---

## 5. Production Deployment

### 5.1 Systemd Service (Linux)

Create `/etc/systemd/system/hkask.service`:

```ini
[Unit]
Description=hKask Server
After=network.target docker.service

[Service]
Type=simple
User=hkask
Group=hkask
ExecStart=/usr/local/bin/kask daemon --host 0.0.0.0 --port 8080
Environment=DI_API_KEY=${DEEPINFRA_KEY}
Environment=HKASK_DATABASE_URL=/var/lib/hkask/hkask.db
Environment=RUST_LOG=hkask=info
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable hkask
sudo systemctl start hkask
sudo systemctl status hkask
```

### 5.2 Docker Deployment

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

ENV HKASK_DATABASE_URL=/home/hkask/hkask.db

EXPOSE 8080

CMD ["kask", "daemon", "--host", "0.0.0.0", "--port", "8080"]
```

Build and run:
```bash
docker build -t hkask:latest .
docker run -d -p 8080:8080 --name hkask hkask:latest
```

### 5.3 Kubernetes Deployment — Future (not supported in v0.27.0)

> **Note:** Kubernetes multi-replica deployment is not supported in v0.27.0. hKask uses SQLCipher (single-writer SQLite) which cannot support multi-replica deployments. The single-server model described in §5.1 (systemd) and §5.2 (Docker) is the only supported production path. The Kubernetes manifest below is a placeholder for a future version with a distributed storage backend.

```yaml
# FUTURE — not supported in v0.27.0
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hkask-api
spec:
  replicas: 1
  selector:
    matchLabels:
      app: hkask-api
  template:
    metadata:
      labels:
        app: hkask-api
    spec:
      containers:
      - name: hkask-api
        image: hkask:latest
        ports:
        - containerPort: 8080
        env:
        - name: DI_API_KEY
          valueFrom:
            secretKeyRef:
              name: hkask-secrets
              key: deepinfra-api-key
        - name: HKASK_DATABASE_URL
          value: "/data/hkask.db"
        volumeMounts:
        - name: data
          mountPath: /data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: hkask-data-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: hkask-api
spec:
  selector:
    app: hkask-api
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
```

---

## 6. Health Checks

### 6.1 API Health Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/cns/health` | GET | CNS health status |
| `/api/sovereignty/status` | GET | User sovereignty status |
| `/api/templates` | GET | Template registry status |

### 6.2 Example Health Check

```bash
# Check CNS health
curl -s https://hkask.your-domain.com/api/cns/health | jq

# Expected response
{
  "overall_deficit": 0,
  "critical_count": 0,
  "warning_count": 0,
  "healthy": true
}
```

### 6.3 CLI Health Check

From the browser terminal (`kask repl`):

```bash
# Check CNS
kask cns health

# Check sovereignty status
kask sovereignty status
```

---

## 7. Monitoring

### 7.1 Key Metrics

| Metric | Alert Threshold | Action |
|--------|-----------------|--------|
| CNS variety deficit | >100 | Investigate tool usage patterns |
| Algedonic alerts | >5/hour | Escalate to on-call |
| API latency p99 | >500ms | Scale horizontally |
| Database size | >10GB | Archive old data |

### 7.2 Log Analysis

```bash
# View recent errors
journalctl -u hkask -p err --no-pager

# Search for CNS alerts
journalctl -u hkask | grep "ALGEDONIC ALERT"

# Monitor variety counters
journalctl -u hkask | grep "variety"
```

### 7.3 Observability via CNS

CNS provides programmatic observability through:
- `cns.*` spans emitted via `NuEventSink`
- Variety counters tracked per bot/capability
- Algedonic alerts escalated to Curator/human when variety deficit >100

**No visual dashboards.** All monitoring is programmatic:
- Query CNS spans via `hkask-cns` crate APIs
- Check variety counters in application logs
- Algedonic alerts appear in journal logs: `journalctl -u hkask | grep "ALGEDONIC ALERT"`

This is a deliberate design decision: hKask is a headless system with no visual UI.

---

## 8. Backup & Recovery

### 8.1 Portable Encrypted Backup

hKask backups are portable, encrypted SQLCipher archives exported via `kask backup export`.
The archive contains the user's full triple set and can be uploaded to a new server for migration.

```bash
# Export encrypted backup archive
kask backup export --passphrase "user-chosen-passphrase"

# The archive is encrypted with the user-provided passphrase (never stored on server)
# Downloadable via scp or the API
```

**Key properties:**
- Single SQLCipher-encrypted SQLite file
- User-provided passphrase at export time (server never stores it)
- Portable across servers — upload to a new server and resume
- `CnsSpan::BackupExport` emitted for observability

### 8.2 Scheduled Auto-Export

```bash
# Configure daily automatic exports, keep last 7
kask config set backup.auto-export.frequency daily
kask config set backup.auto-export.retention 7

# Archives stored at: /var/lib/hkask/exports/{webid}/
```

### 8.3 Server Migration

Download the archive from old server, upload to new server:

```bash
kask backup upload --server https://new-server.hkask.example
```

See the [Deployment & Multi-User Plan](../plans/deployment-and-backup.md#4-backup-model--server-side-export-as-portable-sovereignty-archive) for full details.

---

## 9. Security Hardening

### 9.1 File Permissions

```bash
# Set secure permissions
chown -R hkask:hkask /var/lib/hkask
chmod 700 /var/lib/hkask
chmod 600 /var/lib/hkask/hkask.db
```

### 9.2 Network Security

- Caddy handles TLS termination (auto Let's Encrypt)
- Run API behind Caddy reverse proxy
- Restrict inference provider access to internal network
- Use firewall rules to limit API access

### 9.3 Capability Security

- Rotate `HKASK_CAPABILITY_SECRET` regularly
- Use short-lived capability tokens (default: 30 days)
- Audit capability grants with `kask bot list`

---

## 10. Troubleshooting

### 10.1 Common Issues

| Issue | Cause | Resolution |
|-------|-------|------------|
| `Inference error: error sending request` | Provider unreachable | Verify provider URL and network connectivity |
| `Database locked` | Concurrent access | Ensure single writer; use WAL mode |
| `Template not found` | Registry empty | Register templates with `kask template register` |
| `Capability denied` | Missing/invalid token | Grant capability with `kask bot grant` |
| `Chat response slow` | High inference latency | Check provider load; reduce `max_tokens` |
| `WebSocket disconnected` | Session expired | Re-authenticate via OAuth sign-in |

### 10.2 Chat-Specific Issues

**Empty or generic responses:**
- Increase `temperature` for more creative outputs
- Check template selection (auto-select may not match intent)
- Verify prompt format with `RUST_LOG=debug`

**Template not applied:**
- Explicit template ID: `kask chat --interactive --template prompt/selector`
- Check template exists: `kask template get <id>`
- Verify template type matches input (prompt, cognition, process)

**Inference timeout:**
- Increase request timeout in `InferenceConfig` (default: 120s)
- Check provider server load
- Reduce `HKASK_SOAP_MAX_TOKENS`

### 10.3 Debug Mode

```bash
# Enable verbose logging
export RUST_LOG=debug

# View detailed CNS spans
kask cns health --verbose

# Test DeepInfra connectivity
curl -s https://api.deepinfra.com/v1/openai/chat/completions \
  -H "Authorization: Bearer $DI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "meta-llama/Llama-3.3-70B-Instruct", "messages": [{"role": "user", "content": "test"}]}'

# Test DeepInfra embeddings
curl https://api.deepinfra.com/v1/embeddings \
  -H "Authorization: Bearer $DEEPINFRA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "Qwen/Qwen3-Embedding-0.6B", "input": ["test"]}'
```

### 10.4 Support

- Documentation: `docs/` directory
- Issue tracker: https://github.com/mdz-axo/hKask/issues
- Architecture: `docs/architecture/hKask-architecture-master.md`
- API Reference: `kask docs openapi` or `docs/openapi.json`
- Step-by-step setup: [Admin Install Guide](admin-install-guide.md)

---

## 11. Upgrade Path

### 11.1 Version Compatibility

| From Version | To Version | Breaking Changes | Migration Required |
|--------------|------------|------------------|-------------------|
| v0.20.x | v0.21.0 | CNS namespace (`okh.*` → `cns.*`) | No |
| v0.21.0 | v1.0.0 | None expected | No |

### 11.2 Upgrade Procedure

```bash
# Backup database
cp /var/lib/hkask/hkask.db /backup/hkask-pre-upgrade.db

# Stop service
sudo systemctl stop hkask

# Install new binary
cp target/release/kask /usr/local/bin/

# Start service
sudo systemctl start hkask

# Verify
kask --version
curl -s https://hkask.your-domain.com/api/cns/health
```

---

## 12. API Reference

### 12.1 Chat Endpoints

**POST /api/chat**
Curator chat with Okapi inference.

```bash
curl -X POST https://hkask.your-domain.com/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "input": "What is the capital of France?",
    "template_id": null
  }'
```

Response:
```json
{
  "output": "Paris is the capital of France.",
  "template_id": "auto-select"
}
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `input` | string | Yes | User input text |
| `template_id` | string | No | Template ID (auto-select if null) |

**Response:**
| Field | Type | Description |
|-------|------|-------------|
| `output` | string | LLM response text |
| `template_id` | string | Template used (explicit or auto-select) |

### 12.2 Template Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/templates` | List all templates |
| GET | `/api/templates/:id` | Get template by ID |
| POST | `/api/templates` | Register new template |
| GET | `/api/templates/search/:term` | Search by lexicon term |

### 12.3 Pod Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/pods` | List all pods |
| POST | `/api/pods` | Create new pod |
| POST | `/api/pods/:id/activate` | Activate pod |
| POST | `/api/pods/:id/deactivate` | Deactivate pod |
| GET | `/api/pods/:id/status` | Get pod status |

### 12.4 CNS Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/cns/health` | CNS health status |
| GET | `/api/cns/alerts` | Algedonic alerts |
| GET | `/api/cns/variety` | Variety counters |

### 12.5 Sovereignty Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/sovereignty/status` | User sovereignty status |
| POST | `/api/sovereignty/consent/grant` | Grant consent |
| POST | `/api/sovereignty/consent/revoke` | Revoke consent |
| GET | `/api/sovereignty/access/check` | Check data access |

### 12.6 Generate OpenAPI Spec

```bash
# Generate OpenAPI specification
kask docs openapi -o docs/openapi.json

# Or via API (if running)
curl -s https://hkask.your-domain.com/api/openapi.json -o openapi.json
```

---

*This deployment guide is part of hKask v0.27.0 documentation. For architecture details, see `docs/architecture/hKask-architecture-master.md`. For step-by-step server setup, see [Admin Install Guide](admin-install-guide.md).*

**ℏKask - A Minimal Viable Container for Agents**
