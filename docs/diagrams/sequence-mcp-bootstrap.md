---
title: "MCP Bootstrap and Tool Dispatch — Sequence Diagram"
diataxis: reference
---

# MCP Bootstrap and Tool Dispatch — Sequence Diagram

**Diataxis quadrant:** How-To / Explanation  
**Domain ontology tier:** Core  
**Purpose:** Show the startup sequence for an MCP server and the tool dispatch path through the OCAP membrane. Used as reference for bootstrapping new MCP servers.  
**Verified against:** `crates/hkask-mcp/src/lib.rs`, `crates/hkask-mcp/src/dispatch.rs`, `crates/hkask-cns/src/governed_tool.rs`  
last-verified-against: "3d1a876f45e3ce64864c3453f1e71d75b2f14376"

```mermaid
sequenceDiagram
    participant CLI as kask daemon start
    participant BOOT as bootstrap_mcp_server
    participant GATES as verify_startup_gates
    participant CTX as ToolContext
    participant OCAP as GovernedTool membrane
    participant TOOL as Tool implementation
    participant CNS as CnsRuntime (span emission)

    CLI->>BOOT: Start MCP server (e.g., hkask-mcp-codegraph)
    BOOT->>BOOT: Register tools via mcp_server! macro
    BOOT->>CTX: Build ToolContext (name, version, description)
    CTX->>GATES: verify_startup_gates()
    
    alt Startup gates pass
        GATES-->>CTX: StartupGateResult::Passed
        BOOT->>BOOT: run_stdio_server()
        Note over BOOT: Server listening on stdio
    else Startup gates fail
        GATES-->>CTX: StartupGateResult::Failed(reason)
        BOOT-->>CLI: Error: startup gates not satisfied
    end

    Note over CLI,TOOL: --- Tool invocation at runtime ---

    CLI->>OCAP: Tool invocation request (tool_name, params, capability_token)
    
    alt OCAP check passes
        OCAP->>OCAP: Reserve energy from gas budget
        OCAP->>CNS: Emit cns.tool.reserved ν-event
        OCAP->>TOOL: Delegate to tool implementation
        TOOL-->>OCAP: Tool result
        OCAP->>OCAP: Settle energy consumption
        OCAP->>CNS: Emit cns.tool.completed ν-event
        OCAP-->>CLI: Tool result (with energy cost)
    else OCAP check fails
        OCAP->>CNS: Emit cns.tool.denied ν-event
        OCAP-->>CLI: Error: capability denied (P4 violation)
    end
```

**Node-to-code mapping:**

| Step | Source |
|------|--------|
| `bootstrap_mcp_server` | `crates/hkask-mcp/src/lib.rs` |
| `mcp_server!` macro | `crates/hkask-mcp/src/lib.rs` |
| `verify_startup_gates` | `crates/hkask-mcp/src/startup.rs` |
| `ToolContext` + `impl_tool_context!` | `crates/hkask-mcp/src/lib.rs` |
| GovernedTool OCAP membrane | `crates/hkask-cns/src/governed_tool.rs` |
| Energy reserve/settle | `crates/hkask-cns/src/energy.rs` |
| CNS span emission | `crates/hkask-cns/src/runtime.rs` |
| `BUILTIN_SERVERS` (16 registrations) | `crates/hkask-mcp/src/lib.rs` |

**Cardinality:** 16 MCP servers registered in `BUILTIN_SERVERS` constant. Each follows this bootstrap sequence. Tool dispatch flows through a single `GovernedTool` instance per invocation. 6 OCAP membrane steps per invocation: OCAP check → energy reserve → ν-event → delegate → settle → ν-event.
