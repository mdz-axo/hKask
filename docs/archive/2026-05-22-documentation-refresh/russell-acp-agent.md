---
title: "Russell Integration — ACP Agent Architecture"
audience: [hKask developers, Russell operators, architects]
last_updated: 2026-05-20
togaf_phase: "G"
version: "1.0.0"
status: "Active"
---

<!-- TOGAF_DOMAIN: Application -->
<!-- VERSION: 1.0.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-20 -->

# Russell Integration — ACP Agent Architecture

## Overview

Russell is an **ACP agent** registered with hKask. Russell monitors and manages the local Linux AI/ML workstation, and accesses inference capabilities through hKask's MCP server infrastructure.

## Architecture

```mermaid
flowchart TB
    Russell["Russell (ACP Agent)<br/>Local machine monitoring"]
    hKask["hKask (MCP Server Host)<br/>Orchestration layer"]
    Okapi["Okapi (Inference Engine)<br/>Port 11435"]
    
    Russell -->|"MCP tools/call<br/>inference/generate<br/>inference/chat<br/>inference/embed"| hKask
    hKask -->|"HTTP POST<br/>/api/generate"| Okapi
    Okapi -->|response| hKask
    hKask -->|MCP response| Russell
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-RUSSELL-HKASK-001
verified_date: 2026-05-20
verified_against: hKask/docs/integrations/russell-acp-agent.md
status: VERIFIED
-->

## Key Design Decision

**Russell does not connect directly to Okapi.**

All inference requests flow through hKask's MCP infrastructure:

| Aspect | Direct Russell → Okapi | Russell → hKask → Okapi |
|--------|----------------------|------------------------|
| Authentication | Russell manages auth | Inherits hKask auth |
| Routing | Fixed backend | hKask can route to multiple backends |
| Observability | Russell-only tracing | Full CNS span coverage |
| Confidence routing | Russell implements | hKask handles escalation |
| Capability discovery | Russell polls Okapi | hKask exposes via MCP |

## ACP Agent Registration

Russell registers with hKask as an ACP agent:

```json
{
  "agent_id": "russell",
  "agent_type": "local_machine_monitor",
  "capabilities": [
    "system_health",
    "skill_dispatch",
    "journal_management"
  ],
  "mcp_access": [
    "inference/generate",
    "inference/chat",
    "inference/embed",
    "inference/rerank"
  ]
}
```

## MCP Tools Available to Russell

| MCP Tool | Okapi Endpoint | Purpose |
|----------|----------------|---------|
| `inference/generate` | `/api/generate` | Completion generation |
| `inference/chat` | `/api/chat` | Chat completion |
| `inference/embed` | `/api/embed` | Batch embeddings |
| `inference/rerank` | `/api/rerank` | Document reranking |

## Request Flow

### 1. Russell → hKask (MCP)

```json
{
  "tool": "inference/generate",
  "arguments": {
    "model": "qwen3:8b",
    "prompt": "What is the system status?",
    "options": {
      "temperature": 0.7
    }
  }
}
```

### 2. hKask → Okapi (HTTP)

```bash
POST http://127.0.0.1:11435/api/generate
{
  "model": "qwen3:8b",
  "prompt": "What is the system status?",
  "options": {
    "temperature": 0.7
  }
}
```

### 3. Okapi → hKask → Russell (Response)

```json
{
  "model": "qwen3:8b",
  "response": "System status: CPU 45%, Memory 62%, GPU 78%...",
  "done": true
}
```

## Authentication

All requests are authenticated with **Macaroons**:

| Layer | Authentication |
|-------|---------------|
| Russell → hKask | Macaroon with skill caveat |
| hKask → Okapi | Macaroon with discharge (delegated access) |

### Macaroon Structure

```
Russell Macaroon:
  iid: russell-prod-1
  skill: evolution-watcher
  before: 2026-05-21T00:00:00Z
  third_party: okapi-access

Discharge Macaroon (from hKask MCP):
  okapi_access: true
  models: [qwen3:8b, qwen3:70b]
  before: 2026-05-20T12:00:00Z
```

### hKask → Okapi Auth

hKask includes bound macaroon in all requests to Okapi:

```http
POST /api/generate
Authorization: Bearer <primary-macaroon>
X-Macaroon-Discharge: <discharge-macaroon>
```

This enables Okapi to:
- Track per-client usage and quotas (by `iid` caveat)
- Audit requests with full caveat context
- Support multiple hKask instances per Okapi deployment
- Enforce capability isolation (per-skill macaroons)

## Observability

All Russell inference requests are traced through hKask CNS spans:

| Span | Purpose |
|------|---------|
| `cns.agent.request.start` | Russell MCP request received |
| `cns.connector.llm.request` | hKask → Okapi request |
| `cns.connector.llm.response` | Okapi → hKask response |
| `cns.agent.request.end` | MCP response to Russell |

## Configuration

### Russell Configuration

```yaml
# ~/.config/russell/config.yaml
hKask:
  mcp_endpoint: "http://127.0.0.1:8080/mcp"
  agent_id: "russell"
```

### hKask Configuration

```yaml
# ~/.config/hkask/config.yaml
okapi:
  endpoint: "http://127.0.0.1:11435"
  
agents:
  - id: "russell"
    type: "local_machine_monitor"
    mcp_access:
      - "inference/generate"
      - "inference/chat"
      - "inference/embed"
      - "inference/rerank"
```

## Error Handling

### Okapi Unavailable

```
Russell → hKask → Okapi (503 Service Unavailable)
                ↓
        hKask retries (3x, exponential backoff)
                ↓
        hKask → Russell (error with retry suggestion)
```

### Authentication Failure

```
Russell → hKask (invalid capability token)
        ↓
hKask → Russell (401 Unauthorized)
        ↓
Russell refreshes capability token
```

## References

- Russell documentation: `~/Clones/russell/docs/`
- hKask MCP protocol: `~/Clones/hKask/docs/architecture/hKask-architecture-master.md`
- Okapi API: `~/Clones/okapi/docs/api.md`

---

*Russell — Local machine monitoring agent for hKask — v0.21.0*
