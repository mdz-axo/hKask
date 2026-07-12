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

Agents are registered on the Matrix homeserver via the CLI. The `register-agent` subcommand creates a Matrix account for the agent:

```bash
kask matrix register-agent "my-agent" --homeserver "https://matrix.example.com"
```

This registers the agent on the specified homeserver. To create a human user account instead, use `kask matrix register-user`.

## 4. Verify Registration

```bash
kask matrix status-sidecar
```

Expected output shows:
- Docker container health (Conduit, Caddy)
- API reachability
- Database status

For agent-to-agent message delivery, use the REPL slash commands within a `kask chat` session (see [Use the REPL](use-repl.md) for the `/matrix` and `/msg` commands).

## 5. Test A2A Message Delivery

A2A messaging is performed through the REPL, not the CLI. Start a chat session and use the `/msg` slash command:

```bash
kask chat
```

Inside the REPL:

```
/msg <room_id> "Hello from hKask"
```

See [Use the REPL](use-repl.md) for the full `/matrix` and `/msg` slash command reference.

## 6. Monitor Message Flow

Matrix transport events emit CNS spans. Subscribe to the communication namespace:

```bash
kask cns subscribe --agent curator --spans cns.communication
```

## Troubleshooting

| Issue | Likely Cause | Fix |
|-------|-------------|-----|
| `Connection refused` | Homeserver unreachable | Verify URL and network access |
| `Authentication failed` | Invalid credentials | Check username/password |
| `Room not found` | Agent not registered | Run `kask matrix register-agent` |
| `Feature not enabled` | `matrix` feature disabled | Rebuild with `--features matrix` |

## Notes

- Matrix transport requires the `hkask-communication` crate with the `matrix` feature enabled.
- Full MatrixTransport integration tests require a running Conduit homeserver (Docker sidecar). Unit tests for AgentRegistry (record, resolve, deregister, monitor, watchers) run without a homeserver.
- The 7R7 listener processes incoming messages on a dedicated Matrix room listener thread.
