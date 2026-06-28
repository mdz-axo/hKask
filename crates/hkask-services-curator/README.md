# hkask-services-curator — Curator Daemon Service

Curator daemon service: escalation handling, alert management, algedonic log querying, and curator agent dispatch.

**Version:** v0.31.0 | **Crate:** `hkask-services-curator`

## Key Types

| Type | Purpose |
|------|---------|
| `EscalationResponse` | Structured escalation with severity, source, timestamp, and resolution state |
| `CuratorService` | Primary service struct |

## Operations

| Method | Purpose |
|--------|---------|
| `list_escalations` | Query all open escalations from the algedonic log |
| `resolve` | Mark an escalation as resolved with identity attribution |
| `dismiss` | Dismiss an escalation (non-actionable) with identity attribution |

## Dependencies

- `hkask-types` — CNS spans, nu-event, WebID
- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-services-context` — `AgentService` context
- `hkask-agents` — Curator agent types and dispatch
- `hkask-cns` — Algedonic log, escalation events
- `hkask-storage` — Persistent escalation store
- `hkask-communication` — Matrix transport for curator notifications
