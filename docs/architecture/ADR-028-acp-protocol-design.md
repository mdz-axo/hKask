---
title: "ADR-028: ACP Protocol Design (JSON-RPC 2.0 over stdio)"
audience: [architects, developers]
last_updated: 2026-05-29
version: "1.0.0"
status: "Deferred"
domain: "Technology"
ddmvss_categories: [interface, composition]
---

# ADR-028: ACP Protocol Design

**Date:** 2026-05-29 (retroactive)  
**Status:** ⚠️ DEFERRED — Transport layer not implemented in v0.21.x  
**Supersedes:** N/A

> **⚠️ DEFERRED (v0.21.x):** The transport layer types described in this ADR (`AcpTransport`, `AcpWireMessage`, `AcpWireResponse`, `LoopbackHttpTransport`, `StdioTransport`) have been **removed from the codebase**. The `AcpPort` trait and `RussellAcpAdapter` remain active; only the wire-protocol transport abstraction and its implementations were deleted. This ADR is preserved as a design record. Revisit if/when multi-transport ACP is needed.

## Context

hKask agents need bidirectional communication — both within the local hKask instance (pod-to-pod) and across system boundaries (hKask ↔ Russell). A protocol must support agent registration, capability-bearing message passing, and transport flexibility.

## Decision

**JSON-RPC 2.0 over stdio** with three transport options.

```
Transport options:  ⚠️ DEFERRED — not in v0.21.x
├── InProcessMcpTransport — Co-located servers (no network, shared memory)
├── StdioTransport           — Child process servers (process isolation)  ⚠️ REMOVED
└── HttpMcpTransport         — Remote servers (HTTPS + OCAP tokens)
```

Key design elements:
1. **JSON-RPC 2.0 messages** — `AcpWireMessage` / `AcpWireResponse` serialized as JSON  ⚠️ REMOVED — types deleted from codebase
2. **Port-based architecture** — `AcpPort` trait for agent operations, `AcpTransport` trait for wire protocol  ⚠️ REMOVED — trait deleted from codebase
3. **Bidirectional bridges** — `RussellAcpAdapter` implements cross-system communication
4. **Capability-bearing messages** — every ACP message carries a `CapabilityToken` in its context

## Rationale

1. **JSON-RPC 2.0 is simple.** [^jsonrpc] Request/response semantics with error codes. No streaming, no multiplexing — complexity is in the agent semantics, not the transport.

2. **Stdio provides zero-config process isolation.** Child processes communicate over stdin/stdout. No port allocation, no TLS certificates, no network configuration. Ideal for local MCP servers and co-located agents.

3. **Transport abstraction enables future flexibility.** The `AcpTransport` trait allows swapping stdio for HTTP or WebSocket without changing agent logic. The `RussellAcpAdapter` demonstrates this with bidirectional bridging.  ⚠️ DEFERRED: `AcpTransport` and its implementations (`LoopbackHttpTransport`, `StdioTransport`) were removed; `RussellAcpAdapter` uses `AcpPort` directly.

4. **Capability-first design.** [^miller-robust] Every message carries its authority explicitly. No ambient authority — agents cannot invoke operations they don't have tokens for, regardless of transport.

5. **Constraint compliance.** ~~Three transport implementations satisfy P1 (two consumers for `AcpTransport`).~~  ⚠️ DEFERRED: `AcpTransport` and its implementations were removed; P1 compliance via `AcpPort` consumers instead. The port trait boundary satisfies Cockburn's hexagonal purity.

## Consequences

### Positive

- Simple, debuggable protocol (human-readable JSON)
- Transport-independent agent logic
- Capability enforcement at protocol boundary
- Cross-system bridging demonstrated (Russell)

### Negative

- No streaming support (messages are discrete)
- Stdio transport requires child process management (`McpSupervisor`)
- HTTP transport introduces network attack surface (mitigated by OCAP tokens)

### Alternative Rejected

**gRPC or Protobuf** would provide streaming and schema enforcement but adds:
- Build-time code generation dependency
- Binary wire format (harder to debug)
- Complexity that exceeds the "minimal" budget

**WebSocket** rejected for local communication because stdio is simpler for co-located processes.

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (No trait without two consumers) | ⚠️ DEFERRED — `AcpTransport`, `LoopbackHttpTransport`, `StdioTransport` removed; P1 now satisfied by `AcpPort` consumers |
| P3 (No module directory without encapsulation) | ✅ `acp/` directory encapsulates all ACP types |
| Cockburn (Hexagonal purity) | ✅ `AcpPort` trait isolates agent logic from transport |
| Miller (No ambient authority) | ✅ Every message carries explicit capability tokens |

## References

[^jsonrpc]: JSON-RPC Working Group. (2010). *JSON-RPC 2.0 Specification*. https://www.jsonrpc.org/specification
[^miller-robust]: Miller, M. S. (2006). *Robust Composition*. Johns Hopkins University.

---

*ℏKask — A Minimal Viable Container for Agents — ADR-028 — v0.21.0*
