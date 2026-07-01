---
title: "MCP Tool Dispatch — Sequence Diagram"
audience: [architects, developers, agents]
last_updated: 2026-06-30
version: "0.31.0"
status: "Active"
domain: "Composition"
mds_categories: ["composition", "trust"]
---

# MCP Tool Dispatch Sequence

**Purpose:** Trace the full MCP tool invocation path from `McpDispatcher::invoke()` through the `GovernedTool` OCAP membrane, to the `RawMcpToolPort` transport layer, with all CNS span emission points, energy budget checks, and error-rejection paths.

**Related:** [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) §P4 — Clear Boundaries (OCAP), [MDS.md](../architecture/core/MDS.md) §6

---

## Dispatch Flow Description

When a caller requests tool execution through the `McpPort` trait, the dispatch flows through three architectural layers:

1. **Dispatcher (`McpDispatcher::invoke`)** — Resolves caller identity and tool metadata (`server_id`), then delegates to the governed membrane. Maps `ToolPortError` variants into `TemplateError` for the caller.

2. **OCAP Membrane (`GovernedTool::invoke`)** — The security boundary where all governance decisions are made. A 7-step hold-settle pipeline:
   - **Step 0:** Cryptographic token signature verification (`token.verify()`)
   - **Step 1:** OCAP authority check — two paths: exact-match (ad-hoc invocation tokens) or domain-based matching via `capabilities_match()` (agent capability tokens)
   - **Step 2:** Energy budget check — `can_proceed()` + `reserve_gas()` hold; emits `GasDepleted` span on rejection, `GasReserved` on success
   - **Step 3:** CNS observability — emits `cns.tool.invoked` span
   - **Step 4:** Delegate to inner `ToolPort` (the raw MCP transport)
   - **Step 5:** Settle gas — `settle_gas()` with refund for over-estimation; emits `GasSettled` + `ToolConsumptionEvent` on direct channel
   - **Step 6:** CNS outcome — emits `cns.tool.completed` span (parented to invoked span)
   - **Step 7:** Record outcome for quality tracking via `CyberneticsLoop::record_outcome()`

3. **Transport (`RawMcpToolPort::invoke`)** — Checks for live Peer connection, calls `McpRuntime::call_tool()` over rmcp stdio, parses `CallToolResult` into `serde_json::Value`.

Per-tool CNS span emission at the server level uses `ToolSpanGuard` (via `execute_tool()`), which emits via `tracing::info!(target: "cns.tool")`. The `Drop` implementation ensures forgotten spans still emit a "dropped" status.

Startup-time P4 enforcement uses `verify_startup_gates()`: Gate 1 (authentication), Gate 2 (role assignment), Gate 3 (per-tool capability query). Gate 3 denials are non-fatal — the server starts in degraded mode.

---

## Tool Dispatch Sequence

```mermaid
sequenceDiagram
    participant Caller as Caller
    participant Disp as McpDispatcher
    participant Gov as GovernedTool
    participant Cyber as CyberneticsLoop
    participant Sink as NuEventSink
    participant Raw as RawMcpToolPort
    participant Rtm as McpRuntime
    participant Peer as MCP Server (Peer)

    Caller->>+Disp: invoke(tool, input, token)
    Disp->>+Disp: get_tool_info(tool) → server_id

    opt GovernedTool absent
        Disp-->>Caller: TemplateError::Mcp(NotConnected)
    end

    Disp->>+Gov: invoke(server, tool, input, token)

    %% Step 0: Cryptographic verification
    Gov->>+Gov: token.verify()
    alt Invalid signature
        Gov-->>Disp: ToolPortError::CapabilityDenied
        Disp-->>Caller: TemplateError::CapabilityDenied
    end

    %% Step 1: OCAP authority
    Gov->>+Gov: verify_capability_exact(token, tool)
    alt Exact match fails
        Gov->>+Raw: get_tool_info(tool)
        Raw->>+Rtm: get_tool_info(tool)
        Rtm-->>-Raw: Option<ToolInfo>
        Raw-->>-Gov: required_capability
        Gov->>+Gov: verify_capability_domain(token, required)
    end
    alt Authority denied
        Gov-->>Disp: ToolPortError::CapabilityDenied
        Disp-->>Caller: TemplateError::CapabilityDenied
    end

    %% Step 2: Energy budget
    Gov->>+Cyber: can_proceed(agent, estimated_cost)
    alt Budget exceeded
        Cyber-->>Gov: false
        Gov->>+Sink: persist(GasDepleted event)
        Sink-->>-Gov: ()
        Gov-->>Disp: ToolPortError::EnergyBudgetExceeded
        Disp-->>Caller: TemplateError::Mcp(EnergyBudgetExceeded)
    end
    Cyber-->>-Gov: true
    Gov->>+Cyber: reserve_gas(agent, estimated_cost)
    Cyber-->>-Gov: Ok(())
    Gov->>+Sink: persist(GasReserved event)
    Sink-->>-Gov: ()

    %% Step 3: Invoked span
    Gov->>+Sink: persist(cns.tool.invoked)
    Sink-->>-Gov: ()

    %% Step 4: Delegate to inner tool
    Gov->>+Raw: invoke(server, tool, args, token)
    Raw->>+Rtm: get_peer(server)
    alt No live connection
        Rtm-->>Raw: None
        Raw->>+Rtm: tool_exists(tool)
        alt Tool not found
            Rtm-->>Raw: false
            Raw-->>Gov: ToolPortError::NotFound
        else Registered but not connected
            Rtm-->>Raw: true
            Raw-->>Gov: ToolPortError::InvocationFailed
        end
    end
    Rtm-->>Raw: Some(Peer)

    Raw->>+Rtm: call_tool(server, tool, arguments)
    Rtm->>+Peer: call_tool(CallToolRequestParams)
    Peer-->>-Rtm: CallToolResult
    Rtm-->>-Raw: CallToolResult

    alt is_error flag set
        Raw-->>Gov: ToolPortError::InvocationFailed
    end
    Raw->>+Raw: parse_call_result → Value
    Raw-->>-Gov: Ok(Value)

    %% Step 5: Settle gas
    Gov->>+Cyber: settle_gas(agent, reserved, actual)
    Cyber-->>-Gov: Ok(refund)
    Gov->>+Sink: persist(GasSettled event)
    Sink-->>-Gov: ()
    opt ToolConsumptionEvent channel wired
        Gov->>+Cyber: send(ToolConsumptionEvent)
        Cyber-->>-Gov: ()
    end

    %% Step 6: Completed span
    Gov->>+Sink: persist(cns.tool.completed, parent=invoked)
    Sink-->>-Gov: ()

    %% Step 7: Quality tracking
    Gov->>+Cyber: record_outcome(server, success, error_kind)
    Cyber-->>-Gov: ()

    Gov-->>-Disp: Result<Value, ToolPortError>
    Disp-->>-Caller: Result<Value, TemplateError>
```

---

## Per-Tool CNS Span (Server Side)

```mermaid
sequenceDiagram
    participant Tool as MCP Tool Handler
    participant Guard as ToolSpanGuard
    participant Log as tracing (cns.tool)

    Tool->>+Guard: new(tool_name, caller)
    Tool->>+Guard: with_ontology(concept) [optional]
    Tool->>+Tool: business logic → Result<Value, McpToolError>
    Tool->>+Guard: finish(result)

    alt Ok
        Guard->>+Log: info! (outcome=ok, caller, ontology)
        Log-->>-Guard: ()
        Guard->>+Tool: record_tool_outcome("success")
        Guard-->>-Tool: JSON output
    else Err
        Guard->>+Log: info! (outcome=error, error_kind, caller)
        Log-->>-Guard: ()
        Guard->>+Tool: record_tool_outcome("error")
        opt heal callback set
            Guard->>+Guard: heal_error_cb(output, tool_name)
        end
        opt experience callback set
            Guard->>+Guard: experience_cb("error")
        end
        Guard-->>-Tool: JSON error string
    end

    Note over Guard,Log: Drop: if neither ok() nor error() called, emits "dropped" span
```

---

## P4 Startup Gates (Verification)

```mermaid
sequenceDiagram
    participant Main as Server main()
    participant Gates as verify_startup_gates()
    participant Daemon as DaemonClient
    participant Handler as DaemonHandler

    Main->>+Gates: verify_startup_gates(client, replicant, role, tools)

    Gates->>+Daemon: auth_query(replicant)
    Daemon->>+Handler: check_auth(replicant)
    Handler-->>-Daemon: (authenticated, webid?)
    Daemon-->>-Gates: AuthResponse
    alt Not authenticated
        Gates-->>Main: McpError::Auth
    end

    Gates->>+Daemon: assignment_query(replicant, role)
    Daemon->>+Handler: check_assignment(replicant, role)
    Handler-->>-Daemon: bool
    Daemon-->>-Gates: AssignmentResponse
    alt Not assigned
        Gates-->>Main: McpError::RoleAssignment
    end

    loop each required tool
        Gates->>+Daemon: capability_query(replicant, tool)
        Daemon->>+Handler: check_capability(replicant, tool)
        Handler-->>-Daemon: bool
        Daemon-->>-Gates: CapabilityResponse
        alt Denied
            Note over Gates: Collect into denied_tools (non-fatal)
        end
    end

    Gates-->>-Main: StartupGateResult { authenticated, assigned, denied_tools }
```

---

## DIAGRAM_ALIGNMENT

| Field | Value |
|-------|-------|
| **id** | `DIAG-IC-007` |
| **verified_date** | `2026-06-30` |
| **verified_against** | `crates/hkask-mcp/src/`, `crates/hkask-cns/src/governed_tool.rs`, `crates/hkask-capability/src/verification/checker.rs` |
| **status** | `VERIFIED` |

### Verification notes

- `crates/hkask-mcp/src/dispatch.rs:187–282` — `McpDispatcher` struct, `McpPort` impl, `with_governed_tool()`, invocation routing through membrane
- `crates/hkask-mcp/src/dispatch.rs:36–114` — `RawMcpToolPort` struct and `ToolPort` impl — live peer check, `call_tool()`, error flag handling, result parsing
- `crates/hkask-mcp/src/dispatch.rs:229–272` — `McpPort::invoke()` — server_id lookup → GovernedTool delegation → ToolPortError → TemplateError mapping
- `crates/hkask-cns/src/governed_tool.rs:79–87` — `GovernedTool` struct with inner port, cybernetics loop, event sink, energy estimator, agent WebID
- `crates/hkask-cns/src/governed_tool.rs:188–474` — `ToolPort` impl — 7-step OCAP membrane: token verify → authority check (exact + domain fallback) → energy budget hold → CNS invoked span → inner delegation → gas settle + consumption event → CNS completed span → quality tracking
- `crates/hkask-cns/src/governed_tool.rs:151–157` — `verify_capability_exact()` — exact tool-name match via `is_valid_for()`
- `crates/hkask-cns/src/governed_tool.rs:164–167` — `verify_capability_domain()` — domain-based match via `capabilities_match()`
- `crates/hkask-cns/src/governed_tool.rs:173–185` — `verify_capability_domain_fallback()` — async tool metadata lookup + domain match
- `crates/hkask-capability/src/verification/checker.rs:20–33` — `CapabilityChecker` struct — signing key, trusted roots, root enforcement flag
- `crates/hkask-capability/src/verification/checker.rs:112–120` — `verify()` — signature verification with optional root anchoring (fail-closed on empty roots)
- `crates/hkask-mcp/src/runtime.rs:124–367` — `McpRuntime` — server registry, tool registry, live connections, `start_server()`, `call_tool()`, `get_tool_info()`
- `crates/hkask-mcp/src/runtime.rs:290–307` — `McpRuntime::call_tool()` — direct Peer invocation via rmcp `CallToolRequestParams`
- `crates/hkask-mcp/src/server.rs:241–403` — `ToolSpanGuard` — per-tool CNS span with `ok()`, `error()`, `finish()`, `Drop` guard for forgotten spans
- `crates/hkask-mcp/src/server.rs:446–458` — `execute_tool()` — automatic span emission + outcome recording
- `crates/hkask-mcp/src/server.rs:462–478` — `execute_tool_semantic()` — span with ontology concept tagging
- `crates/hkask-mcp/src/server.rs:852–861` — `emit_tool_span_with_caller()` — `tracing::info!(target: "cns.tool")` with caller WebID
- `crates/hkask-mcp/src/startup.rs:101–190` — `verify_startup_gates()` — Gate 1 (auth), Gate 2 (assignment), Gate 3 (capability per tool, non-fatal denial)
- `crates/hkask-mcp/src/startup.rs:44–52` — `StartupGateResult` — authenticated, assigned, denied_tools fields
- `docs/architecture/core/PRINCIPLES.md:52–56` — P4 Clear Boundaries (OCAP) — pod boundary as enforcement perimeter
- `docs/DIAGRAMS_INDEX.md:27` — DIAG-DC-005 — existing MCP Tool Dispatch with OCAP constraint enforcement

---

## Cross-References

| Reference | Description |
|-----------|-------------|
| [PRINCIPLES.md §P4](../architecture/core/PRINCIPLES.md) | Clear Boundaries (OCAP) — P4.1 Pod Boundary as OCAP Enforcement Perimeter |
| [DIAGRAMS_INDEX.md DIAG-DC-005](../DIAGRAMS_INDEX.md) | Existing MCP Tool Dispatch with OCAP constraint enforcement (MDS.md §6) |
