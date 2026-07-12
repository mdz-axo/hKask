---
title: "hKask API Guide"
audience: [developers, operators, agents]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "API"
mds_categories: [domain, composition, lifecycle]
---

# hKask API Guide

**Version:** 0.31.0  
**Base URL:** `http://localhost:8080/api`  
**Provenance:** Endpoint list derived from `create_router()` merges in
`crates/hkask-api/src/lib.rs`. Verify against live `/api-docs/openapi.json`
for the authoritative spec.

## Quick Start

```bash
# Start the API server
kask serve --host 127.0.0.1 --port 8080

# API docs (Swagger UI — requires internet for CDN assets)
open http://localhost:8080/docs

# Offline alternative: raw OpenAPI spec
curl http://localhost:8080/api-docs/openapi.json | jq
```

## Authentication

Endpoints are designed to require OCAP `DelegationToken` Bearer tokens (P4).
Whether enforcement is active depends on the auth middleware configuration.

**IS:** The `Bearer` security scheme is declared in the OpenAPI spec.
**OUGHT:** All endpoints should reject unauthenticated requests per P4.

Obtain a token via the REPL onboarding flow or A2A agent registration.

```bash
curl -H "Authorization: Bearer $HKASK_TOKEN" http://localhost:8080/api/v1/agents
```

## Endpoints

### Chat
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/chat` | Send a chat message (streaming via SSE) |
| GET | `/api/v1/chat/ws` | WebSocket chat (bidirectional streaming) |

### Agent Management
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/agents` | List registered agents |
| GET | `/api/v1/agents/{name}` | Get agent details |
| POST | `/api/v1/pods` | Create a new agent pod |
| GET | `/api/v1/pods` | List active pods |
| POST | `/api/v1/pods/{id}/activate` | Activate a pod |
| POST | `/api/v1/pods/{id}/deactivate` | Deactivate a pod |

### CNS (Cybernetic Nervous System)
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/cns/health` | CNS health status |
| GET | `/api/v1/cns/variety` | Variety counter values |
| GET | `/api/v1/cns/subscribe` | SSE stream of CNS events |

### Memory
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/episodic` | Store episodic h_mem |
| GET | `/api/v1/episodic` | Query episodic memories |
| POST | `/api/v1/consolidation` | Consolidate episodic → semantic |
| GET | `/api/v1/consolidation/status` | Consolidation status |

### Models
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/models` | List available models (DeepInfra, fal.ai, Together AI, OpenRouter, KiloCode) |
| GET | `/api/v1/models/search?q={query}` | Search models |

### Templates & Bundles
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/templates` | List templates |
| POST | `/api/v1/bundles/compose` | Compose a skill bundle |
| POST | `/api/v1/bundles/{id}/apply` | Apply a bundle |
| POST | `/api/v1/bundles/{id}/evolve` | Evolve a bundle |

### Sovereignty
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/sovereignty/consent` | Grant consent |
| DELETE | `/api/v1/sovereignty/consent` | Revoke consent |
| GET | `/api/v1/sovereignty/check` | Check consent status |
| POST | `/api/v1/sovereignty/verify` | Verify sovereignty boundaries |

### Wallet
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/wallet/balance` | Get rJoule balance |
| GET | `/api/v1/wallet/transactions` | List transactions |
| POST | `/api/v1/wallet/deposit` | Create deposit address |
| POST | `/api/v1/wallet/withdraw` | Estimate withdrawal fee |

### Export (Data Portability)
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/export` | Create sovereignty archive |
| GET | `/api/v1/export/{id}` | Download archive |
| POST | `/api/v1/export/upload` | Upload and import archive |

### Specs
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/specs` | Create specification |
| GET | `/api/v1/specs` | List specifications |
| GET | `/api/v1/specs/{id}` | Get specification |
| POST | `/api/v1/specs/{id}/assess` | Assess writing quality |

### Admin
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/admin/invite` | Create invite |
| GET | `/api/v1/admin/invite` | List invites |
| GET | `/api/v1/admin/sessions` | List active sessions |
| GET | `/api/v1/admin/config` | Get server config |

## Error Responses

All errors follow the same format:

```json
{ "error": "Human-readable message" }
```

HTTP status codes:
- `400` — Bad request (validation error, missing field)
- `401` — Unauthorized (missing/invalid token)
- `403` — Forbidden (insufficient OCAP scope, consent denied)
- `404` — Not found (agent, pod, template, etc.)
- `409` — Conflict (state transition violation)
- `429` — Rate limited
- `500` — Internal error
- `503` — Service unavailable (keystore, inference provider down)

## Architecture Notes

- **Equal surface exposure (P3):** Every API endpoint has equivalent CLI and
  MCP surface functionality.
- **OCAP-gated (P4):** All endpoints require a `DelegationToken` Bearer token.
- **CNS-observed (P9):** Every request is traced via CNS spans.
- **Storage facade:** `hkask-storage` re-exports from 9 sub-crates
  (`-core`, `-gallery`, `-kata`, `-hmem`, `-archive`, `-token_registry`,
  `-consent_store`, `-sovereignty`, `-escalation`). API code imports from
  the facade, not sub-crates.
- **REPL extracted:** The interactive REPL lives in `hkask-repl`, bridged
  via `ReplHost` trait. The API does not depend on the REPL.

## Generating Docs

```bash
# Rustdoc (all crates)
cargo doc --no-deps --workspace --open

# OpenAPI spec (server must be running)
curl http://localhost:8080/api-docs/openapi.json > openapi.json
```
