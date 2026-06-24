---
title: "ACP Replicant — Zed Configuration"
audience: [developers, agents]
last_updated: 2026-06-17
version: "0.30.0"
status: "Active"
domain: "Deployment"
mds_categories: [lifecycle, composition]
---

# Configuring hKask ACP in Zed

hKask agents can present themselves in Zed via the Agent Client Protocol (ACP). This guide covers setup, configuration, and troubleshooting.

## Prerequisites

- hKask daemon running (`kask daemon start` or via systemd)
- A registered replicant: `kask login <name>` → `kask pod assign <name> acp`
- `hkask-acp` binary built (`cargo build -p hkask-acp`)
- Zed editor (v0.150+ recommended for ACP support)

## Quick Start

### 1. Register the replicant

```bash
kask login alice
kask pod assign alice acp
```

The `acp` role authorizes the replicant to present as an ACP agent.

### 2. Configure Zed

Open Zed's settings (`cmd-,` or `~/.config/zed/settings.json`) and add:

```json
{
  "agent_servers": {
    "hKask": {
      "type": "custom",
      "command": "/path/to/hkask/target/debug/hkask-acp",
      "args": [],
      "env": {
        "HKASK_REPLICANT": "alice",
        "HKASK_MODEL": "qwen3:8b"
      }
    }
  }
}
```

Replace `/path/to/hkask` with your hKask checkout path. Replace `qwen3:8b` with your preferred model.

### 3. Start a thread

In Zed, open the Agent Panel (`cmd-?` on macOS, `ctrl-?` on Linux/Windows) and select "hKask" from the agent picker. Start a new thread.

## Configuration Reference

### `agent_servers` entry

| Field | Required | Description |
|-------|----------|-------------|
| `type` | Yes | Always `"custom"` for external binaries |
| `command` | Yes | Absolute path to `hkask-acp` binary |
| `args` | No | Additional CLI arguments (none needed) |
| `env` | No | Environment variables passed to the replicant process |

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HKASK_REPLICANT` | Yes | `"acp-replicant"` | Replicant identity — must match a registered pod |
| `HKASK_MODEL` | No | `"qwen3:8b"` | Model passed to inference router |
| `RUST_LOG` | No | `"hkask.acp=info"` | Tracing filter for debugging |

### Provider Configuration

The ACP replicant uses hKask's centralized inference router. Provider configuration is read from `.env` (or individual env vars: `DEEPINFRA_API_KEY`, `OPENROUTER_API_KEY`, etc.). Set these in Zed's `env` block or in the daemon's environment:

```json
"env": {
  "HKASK_REPLICANT": "alice",
  "HKASK_MODEL": "together:meta-llama/Llama-3.3-70B-Instruct-Turbo",
  "TOGETHER_API_KEY": "your-key"
}
```

## Architecture

```
Zed (ACP Client)
  │ JSON-RPC 2.0 over stdio
  ▼
hkask-acp (subprocess)
  │ Unix socket
  ▼
hKask Daemon (~/.config/hkask/daemon.sock)
  │
  ├── Auth (P4 Gate 1)
  ├── Capability (P4 Gate 3)
  ├── Tool dispatch → MCP servers
  └── Memory encoding → episodic store
```

## How It Differs from MCP Servers

MCP servers provide *tools* to the IDE (web search, file operations, etc.). The ACP replicant provides an *agent* — a bidirectional conversational presence that:

- Streams inference output as it's generated (`agent_message_chunk`)
- Reports tool calls with status transitions (`pending → in_progress → completed`)
- Maps finish reasons to structured `StopReason` values
- Encodes every interaction as an episodic memory triple
- Accumulates experience across sessions (same memory store as `kask chat`)

## Cloud Deployment

For remote deployments where the hKask daemon runs on a cloud server (not the local machine), use the cloud gateway transport instead of the local Unix socket daemon.

### Prerequisites

- Cloud gateway deployed and running (see [`admin-setup-guide.md`](../guides/admin-setup-guide.md) §5.5)
- Client certificate issued for your replicant (CN must match replicant name)
- DelegationToken issued via `kask token issue`

### Configuration

```json
{
  "agent_servers": {
    "hKask": {
      "type": "custom",
      "command": "/path/to/hkask/target/release/hkask-acp",
      "args": [],
      "env": {
        "HKASK_REPLICANT": "alice",
        "HKASK_MODEL": "qwen3:8b",
        "HKASK_CLOUD_GATEWAY": "hkask.example.com:9443",
        "HKASK_CLIENT_CERT": "/home/alice/.hkask/alice.crt",
        "HKASK_CLIENT_KEY": "/home/alice/.hkask/alice.key",
        "HKASK_SERVER_CA": "/home/alice/.hkask/ca.crt",
        "HKASK_DELEGATION_TOKEN": "{...paste token JSON from kask token issue...}"
      }
    }
  }
}
```

### Cloud Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `HKASK_CLOUD_GATEWAY` | Yes | `host:port` of the cloud gateway |
| `HKASK_CLIENT_CERT` | Yes | Path to client TLS certificate (PEM) |
| `HKASK_CLIENT_KEY` | Yes | Path to client private key (PEM) |
| `HKASK_SERVER_CA` | Yes | Path to server CA certificate (PEM) |
| `HKASK_DELEGATION_TOKEN` | Yes | JSON DelegationToken from `kask token issue` |

### How Discovery Works

The ACP agent checks `HKASK_CLOUD_GATEWAY` at startup. If set, it connects to the cloud gateway via mTLS instead of the local daemon socket. If any cloud env var is missing or misconfigured, it falls back to the local daemon (or degraded mode).

### Token Lifecycle

```bash
# Issue a token (7 day TTL)
kask token issue --replicant alice \
  --capabilities curator:health \
  --capabilities episodic:store \
  --capabilities semantic:search \
  --ttl 168h

# List active tokens
kask token list --replicant alice

# Revoke a compromised token
kask token revoke tok_a1b2c3...
```

### Architecture (Cloud)

```
Zed (ACP Client)
  │ JSON-RPC 2.0 over stdio
  ▼
hkask-acp (subprocess)
  │ mTLS 1.3 + DelegationToken
  ▼
hKask Cloud Gateway (hkask.example.com:9443)
  │ Unix socket (local to cloud server)
  ▼
hKask Daemon
  │
  ├── Auth → CN→WebID derivation
  ├── Capability → DelegationToken per request
  ├── Tool dispatch → MCP servers
  └── Memory encoding → episodic store
```

## Troubleshooting

### Agent not appearing in Zed

- Verify the binary path in `command` is absolute and executable
- Check Zed's ACP logs: `dev: open acp logs` from the Command Palette
- Ensure the daemon is running: `ls ~/.config/hkask/daemon.sock`

### "Replicant not authenticated"

Run `kask login <replicant>` to create a session.

### "Replicant not assigned to the acp MCP role"

Run `kask pod assign <replicant> acp`.

### "Startup gates failed"

The daemon socket is unreachable or the replicant lacks capability tokens. Check:
- Daemon is running: `kask daemon status`
- Capability tokens exist: `kask pod list` shows the replicant as `Activated`

### No inference output

- Verify the model is available: `kask model list` or check provider API keys
- Check `RUST_LOG=hkask.acp=debug` for inference router logs
- The replicant connects to the same inference infrastructure as `kask chat`
