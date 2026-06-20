# hkask-mcp-communication

Thin MCP wrapper over core `hkask-communication` crate. Matrix transport, agent registry, and room management.

## Tools (18)

| Tool | Description |
|------|-------------|
| `health_check` | Server health check |
| `is_healthy` | Health status query |
| `login` | Matrix login |
| `create_room` | Create Matrix room |
| `list_rooms` | List Matrix rooms |
| `invite_user` | Invite user to room |
| `send_message` | Send message to room |
| `get_messages` | Get messages from room |
| `monitor_thread` | Monitor a thread |
| `current_user_id` | Get current user ID |
| `record_mapping` | Record agent-to-user mapping |
| `deregister` | Deregister agent |
| `resolve` | Resolve WebID to Matrix UserId |
| `get_watchers` | Get active watchers |
| `reconnect` | Reconnect to Matrix |
| `start` | Start the server |
| `stop` | Stop the server |
| `run` | Main run loop |

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_MATRIX_URL` | Matrix homeserver URL | `http://localhost:8008` |
| `HKASK_MATRIX_REGISTRATION_TOKEN` | Registration token | `hkask-dev` |
