---
title: "REPL Tool Invocation — Sequence"
audience: [architects, developers, security-reviewers]
last_updated: 2026-07-20
version: "0.32.0"
status: "Active"
domain: "Surface"
mds_categories: [composition, trust]
---

# REPL Tool Invocation — Sequence

Sequence diagram of the tool invocation chain from the REPL turn loop through `GovernedTool` to the MCP runtime. This is the OCAP (Object Capabilities) boundary: every tool call is authorized via a `DelegationToken` minted from the session's A2A secret, and energy is charged through the `GovernedTool` membrane.

```mermaid
sequenceDiagram
    participant Turn as run_turn_loop
    participant Extract as extract_tool_calls
    participant Invoker as ReplToolInvoker
    participant ToolAug as tool_augmented::invoke_tool_call
    participant Gov as GovernedTool
    participant Raw as RawMcpToolPort
    participant Runtime as McpRuntime
    participant Server as MCP Server (child process)

    Turn->>Turn: rt.block_on executor.execute_turn
    Turn->>Extract: extract_tool_calls(response, structured)
    Extract-->>Turn: ParsedResponse { text, tool_calls }
    loop For each tool_call
        Turn->>Invoker: deps.tools.invoke(call)
        Invoker->>ToolAug: invoke_tool_call(call, governed_tool, webid, a2a_secret, host)
        ToolAug->>ToolAug: DelegationToken::new(Tool, call.tool, Execute, user_webid, agent_webid, signing_key)
        ToolAug->>Gov: governed_tool.invoke(server, tool, args, token)
        Gov->>Gov: Authorize via OCAP token
        Gov->>Gov: Charge gas via EnergyBudget
        Gov->>Gov: Emit cns.tool.invoke span
        Gov->>Raw: raw_mcp_tool_port.invoke(server, tool, args)
        Raw->>Runtime: runtime.call_tool(server, tool, args)
        Runtime->>Server: JSON-RPC: tools/call
        Server-->>Runtime: tool result
        Runtime-->>Raw: serde_json::Value
        Raw-->>Gov: result
        Gov->>Gov: Emit cns.tool.result span
        Gov-->>ToolAug: Result<Value>
        ToolAug-->>Invoker: Result<Value>
        Invoker-->>Turn: Result<Value>
        Turn->>Turn: sink.tool_log(formatted result)
    end
    Turn->>Turn: format_tool_results → feed back to next iteration
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-REPL-003
verified_date: 2026-07-20
verified_against: crates/hkask-repl/src/tool_augmented.rs:238-258; crates/hkask-repl/src/deps.rs:262-293; crates/hkask-cns/src/governed_tool.rs
status: VERIFIED
-->

## Security Properties

- **OCAP Authorization:** Every tool invocation requires a `DelegationToken` minted from the session's A2A secret. The token binds the resource (`DelegationResource::Tool`), the tool name, the action (`DelegationAction::Execute`), the user WebID (the authorizing principal), and the agent WebID (the delegated principal). The token is signed with `derive_signing_key(a2a_secret)`.
- **A2A Secret Handling:** The secret is wrapped in `ZeroizingSecret` in both the REPL turn pipeline (`lib.rs`) and the `ReplToolInvoker` (`deps.rs`). The bytes are scrubbed from memory when the invoker is dropped. The previous implementation stored the secret as a plain `Vec<u8>`, defeating the `ZeroizingSecret` protection.
- **Gas Charging:** `GovernedTool` charges gas for tool execution via its internal `EnergyBudget`. This is separate from the inference gas reservation — tool calls have their own energy accounting.
- **CNS Observability:** Every tool invocation emits `cns.tool.invoke` and `cns.tool.result` spans, providing cybernetic observability of the tool-call surface.
- **Two Parse Paths:** `extract_tool_calls` checks structured native function calls first (`InferenceResult.tool_calls` when `finish_reason == "tool_calls"`), then falls back to `<<tool:server/name\n{args}\n>>` text directives. This supports both modern models (native function calling) and legacy models (text directives).

## Cross-References

- [REPL Specification §6.2 — Tool Call Parsing](../specifications/REPL-specification.md#62-tool-call-parsing-two-priority-levels)
- [REPL Specification §6.3 — Tool Call Invocation](../specifications/REPL-specification.md#63-tool-call-invocation)
- [Sovereignty and OCAP Explanation](../explanation/sovereignty-and-ocap.md)
- [REPL Turn Pipeline Flowchart](flowchart-repl-turn-pipeline.md)
