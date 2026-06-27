# hkask-mcp-communication

MCP server wrapping core `hkask-communication` for Matrix messaging and TTS.

## Architecture

This is a thin MCP wrapper — the binary runs as a child process managed by `McpRuntime`. The daemon owns the Matrix connection and 7R7 listener. The MCP server exposes tools agents call to interact with Matrix.

Tools invoke methods on `hkask_communication::MatrixTransport` which uses `matrix-sdk` against a Conduit homeserver.

## Tools (12)

| Tool | Description |
|------|-------------|
| `tts_speak` | Speak text aloud via system TTS (espeak) |
| `tts_generate` | Generate TTS audio file. Returns path to WAV. |
| `tts_list_voices` | List available system TTS voices |
| `send_message` | Send a message to a Matrix room |
| `create_thread` | Create a threaded conversation (Matrix room) |
| `invite_agent` | Invite another replicant to a Matrix room |
| `list_threads` | List active communication threads |
| `monitor_thread` | Assign a thread to an agent's watchlist |
| `tag_agent` | Pull an agent into a discussion via @mention |
| `upload_file` | Upload a file to the Matrix homeserver |
| `send_file` | Upload and send a file as a room attachment |
| `tts_generate` | Generate TTS audio to a WAV file |

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_MATRIX_URL` | Matrix homeserver URL | `http://localhost:8008` |
| `HKASK_MATRIX_AGENT_USERNAME` | Matrix username for this server | (from keychain) |
| `HKASK_MATRIX_AGENT_PASSWORD` | Matrix password for this server | (from keychain) |
| `HKASK_REPLICANT` | Replicant identity for P4 gate verification | `anonymous` |

## Startup

The server verifies three P4 gates at startup before accepting tool invocations:
1. Authentication (is this replicant authenticated?)
2. Assignment (is this replicant assigned to the communication role?)
3. Capability (does this replicant hold OCAP tokens for these tools?)

Gate 3 capability denials are non-fatal — the server starts in degraded mode.

Requires a running Matrix homeserver (Conduit):
```bash
./scripts/conduit/conduit-docker.sh start
```
