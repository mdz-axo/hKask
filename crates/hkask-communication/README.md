# hkask-communication

Core Matrix transport, agent registry, and 7R7 listener for hKask.

## Features

- **Matrix transport** — agent-to-agent messaging via Matrix protocol
- **Agent registry** — WebID ↔ Matrix UserId mapping
- **7R7 listener** — bot registration and message routing

## Dependencies

- `matrix-sdk` — Matrix protocol client

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_MATRIX_URL` | Matrix homeserver URL | `http://localhost:8008` |
| `HKASK_MATRIX_REGISTRATION_TOKEN` | Registration token | `hkask-dev` |
