---
title: "hKask Operations Runbook"
audience: [operators, deployers, project maintainers]
last_updated: 2026-06-18
version: "0.31.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [lifecycle, trust]
---

# hKask Operations Runbook

**Purpose:** Operational guide for deploying and maintaining hKask in production (cloud server) and development environments.

**Governing Principles:** P2 (Affirmative Consent), P4 (Clear Boundaries)

---

## 1. Deployment Architecture

hKask deploys as a single binary (`kask`) with a Unix socket daemon at `~/.config/hkask/daemon.sock`. The daemon handles:
- Agent authentication and session management
- MCP server role assignment
- Capability verification (OCAP)
- Dual memory encoding (episodic + semantic)

### Cloud Server Deployment

```
┌─────────────────────────────────────────┐
│  Cloud Server (single machine)          │
│                                         │
│  ┌──────────┐  ┌──────────────────────┐ │
│  │ Conduit   │  │ kask daemon          │ │
│  │ (Docker)  │  │ (~/.config/hkask/)   │ │
│  │ :8008     │  │ daemon.sock          │ │
│  └──────────┘  └──────────────────────┘ │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │ kask CLI / API / MCP servers      │  │
│  │ All connect via daemon.sock       │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

---

## 2. Prerequisites

### Conduit Matrix Homeserver

```bash
# Start Conduit (Docker or Podman — auto-detected)
./scripts/conduit-docker.sh start

# Verify
./scripts/conduit-docker.sh status
# Expected: Conduit running on http://localhost:8008
```

### Provider API Keys

```bash
# Copy template, fill in keys
cp .env.example .env

# Load into OS keychain (encrypted at rest)
kask keystore load --path .env --shred
```

### Build

```bash
cargo build --release
# Binary at: target/release/kask
```

---

## 3. Startup

```bash
# Start daemon (background)
kask daemon start

# Verify daemon is running
kask daemon status
# Expected: daemon.sock present, PID file exists

# Start a chat session (from within the browser terminal)
# Interactive chat with Curator, default model
/chat

# Named agent with specific model
/chat Alice -m llama3.1:70b
```

---

## 4. Health Checks

```bash
# Build health
cargo check --workspace

# Test health
cargo test --workspace

# Lint health
cargo clippy --workspace -- -D warnings

# Documentation health
bash docs/ci/check-links.sh

# Sovereignty compliance
kask sovereignty verify
```

---

## 5. Key Rotation

```bash
# View current keys (masked)
kask keystore list

# Update a key
kask keystore set DI_API_KEY "new-key-value"

# Delete a key
kask keystore delete OLD_KEY_NAME

# Bulk reload from .env
kask keystore load --path new-.env --shred
```

---

## 6. Troubleshooting

### Daemon Won't Start

```bash
# Check for stale socket
ls -la ~/.config/hkask/daemon.sock
rm ~/.config/hkask/daemon.sock  # Remove stale socket

# Check permissions
ls -la ~/.config/hkask/
# Should be 700 (owner only)

# Restart
kask daemon start
```

### Inference Failures

```bash
# Check provider configuration
kask settings show

# Verify API keys are loaded
kask keystore list

# Test with explicit provider
echo "test" | kask repl -f - -m DI/meta-llama/Llama-3.3-70B-Instruct
```

### MCP Server Connection Issues

```bash
# Verify daemon is running
kask daemon status

# Check replicant assignment
kask pod list

# Verify role assignment
kask pod list  # Shows assigned roles per replicant
```

### Conduit Not Responding

```bash
# Check Docker/Podman
docker ps | grep conduit  # or: podman ps | grep conduit

# Restart Conduit
./scripts/conduit-docker.sh restart

# Check logs
docker logs conduit  # or: podman logs conduit
```

---

## 7. Log Locations

| Component | Log Location |
|-----------|-------------|
| Daemon | `~/.config/hkask/daemon.log` |
| CLI sessions | stdout/stderr (CLI does not persist session output by default) |
| Conduit | Docker/Podman logs (`docker logs conduit`) |
| MCP servers | stdout/stderr of spawned process |
| CNS events | SQLite database (via `hkask-storage`) |

---

## 8. QA & Testing

Run automated QA on every change:

```bash
# Property-based fuzzing (fast, runs on every CI push)
cargo test -p hkask-types-fuzz -p hkask-cns-fuzz -p hkask-inference-fuzz \
           -p hkask-wallet-fuzz -p hkask-storage-fuzz -p hkask-templates-fuzz \
           -p hkask-memory-fuzz -p hkask-services-core-fuzz -p hkask-improv-fuzz

# Coverage-guided fuzzing (nightly, finds edge cases)
cargo +nightly bolero test -p hkask-types-fuzz fuzz_cns_span_parse_never_panics -T 60s -e libfuzzer

# Mutation testing (measures test suite quality)
cargo mutants -p hkask-types --timeout 120

# Triage bolero failures with LLM classifier
export DEEPINFRA_API_KEY="your-key"
cargo test -p hkask-types-fuzz 2>&1 | kask qa triage

# Suggest fuzz targets from surviving mutants
export DEEPINFRA_API_KEY="your-key"
cargo mutants -p hkask-types --timeout 120 2>&1 | grep "Uncaught" | kask qa suggest-fuzz
```

### Interpreting QA Output

| Command | Output | Action |
|---------|--------|--------|
| `kask qa triage` | "No bolero failures detected" | System is healthy |
| `kask qa triage` | "HIGH confidence: ..." | Check for auto-repair PR |
| `kask qa triage` | "LOW confidence: ..." | Open investigation issue |
| `kask qa suggest-fuzz` | "→ [suggestion]" | Consider adding suggested fuzz target |
| `cargo mutants` | "Uncaught mutants in ..." | Test gap — add test or fuzz target |

---

## 9. Backup & Recovery

### Back Up These Files

- `~/.config/hkask/settings.json` — user settings
- `~/.config/hkask/` — daemon state (SQLite databases)
- OS keychain entries — provider API keys

### Recovery Procedure

1. Restore `~/.config/hkask/` from backup
2. Re-load API keys: `kask keystore load --path .env`
3. Start Conduit: `./scripts/conduit-docker.sh start`
4. Start daemon: `kask daemon start`
5. Verify: `kask sovereignty verify`

---

## 10. Shutdown

```bash
# Stop daemon
kask daemon stop

# Stop Conduit
./scripts/conduit-docker.sh stop
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.28.0*
