# hkask-mcp-communication

Thin MCP wrapper over core `hkask-communication` crate. Matrix transport, agent registry, and TTS.

## Tools (9)

| Tool | Description |
|------|-------------|
| `tts_speak` | Speak text aloud using the system TTS engine (espeak) |
| `tts_generate` | Generate TTS audio file using system TTS. Returns path to WAV file. |
| `tts_list_voices` | List available system TTS voices (espeak) |
| `send_message` | Send a message to a Matrix room |
| `create_thread` | Create a threaded conversation (Matrix room) |
| `invite_agent` | Invite another replicant to a Matrix room |
| `list_threads` | List active communication threads |
| `monitor_thread` | Assign a thread to an agent's watchlist for monitoring |
| `tag_agent` | Pull an agent into a discussion by sending them a tagged message |

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_MATRIX_URL` | Matrix homeserver URL | `http://localhost:8008` |
| `HKASK_MATRIX_REGISTRATION_TOKEN` | Registration token | `hkask-dev` |

## Quick Start

```bash
# Requires a running Matrix homeserver (Conduit)
./scripts/conduit/conduit-docker.sh start

# The server starts automatically with kask
kask chat
```
