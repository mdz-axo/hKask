---
title: "How to Setup Matrix Transport — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Setup Matrix Transport

**Goal:** Configure hKask to communicate via the Matrix protocol for agent-to-agent (A2A) messaging.

**Prerequisites:** A running Matrix homeserver (e.g., Conduit, Synapse), or a Matrix.org account. The `matrix` feature must be enabled (it is on by default).

## 1. Enable the Matrix Feature

The `matrix` feature is enabled by default in `hkask-communication`. Verify:

```bash
cargo build --features matrix
```

If you disabled it previously, re-enable it in your `.cargo/config.toml` or pass `--features matrix` to build commands.

## 2. Configure Matrix Credentials

Set these environment variables:

```bash
export HKASK_MATRIX_HOMESERVER="https://matrix.example.com"
export HKASK_MATRIX_USERNAME="@my-agent:example.com"
export HKASK_MATRIX_PASSWORD="your-password"
```

Alternatively, add them to `~/.hkask/settings.yaml`:

```yaml
matrix:
  homeserver: "https://matrix.example.com"
  username: "@my-agent:example.com"
  password: "your-password"
```

## 3. Register Your Agent

Agents are registered in the `AgentRegistry` which maps agent IDs to Matrix rooms:

```bash
kask matrix register --agent-id "my-agent" --display-name "My hKask Agent"
```

This creates a Matrix room for the agent and registers it in the local agent registry.

## 4. Verify Registration

```bash
kask matrix status
```

Expected output shows:
- Connection status: `Connected`
- Homeserver: your configured URL
- Registered agents: list of agent IDs

## 5. Test A2A Message Delivery

Send a test message between agents:

```bash
kask matrix send --to "other-agent" --message "Hello from hKask"
```

Or from within a chat session:

```
kask> /matrix send other-agent "Hello from hKask"
```

## 6. Monitor Message Flow

Matrix transport events emit CNS spans:

```bash
kask cns spans --namespace cns.communication --recent 10
```

## Troubleshooting

| Issue | Likely Cause | Fix |
|-------|-------------|-----|
| `Connection refused` | Homeserver unreachable | Verify URL and network access |
| `Authentication failed` | Invalid credentials | Check username/password |
| `Room not found` | Agent not registered | Run `kask matrix register` |
| `Feature not enabled` | `matrix` feature disabled | Rebuild with `--features matrix` |

## Notes

- Matrix transport requires the `hkask-communication` crate with the `matrix` feature enabled.
- Full MatrixTransport integration tests require a running Conduit homeserver (Docker sidecar). Unit tests for AgentRegistry (record, resolve, deregister, monitor, watchers) run without a homeserver.
- The 7R7 listener processes incoming messages on a dedicated Matrix room listener thread.
