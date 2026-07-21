---
title: "REPL Turn Pipeline — Control Flow"
audience: [architects, developers]
last_updated: 2026-07-20
version: "0.32.0"
status: "Active"
domain: "Surface"
mds_categories: [composition, lifecycle]
---

# REPL Turn Pipeline — Control Flow

Flowchart of `run_turn_loop()` in `crates/hkask-repl/src/turn.rs`. The turn pipeline is the core cybernetic loop of the REPL: it senses (gas, token usage), decides (iteration cap, tool call extraction), acts (invokes tools, displays responses), and returns to sense. Both the CLI (stdout) and TUI (capture buffer) surfaces share this single loop via the `TurnSink` trait.

```mermaid
flowchart TD
    Start([single_agent_turn called]) --> Reserve[Try reserve gas via GasGovernor]
    Reserve -->|None| GasExhausted[Gas budget exhausted]
    GasExhausted --> ReturnExhausted[Return TurnOutcome budget_exhausted=true]
    Reserve -->|Some guard| BuildInput[Build TurnInput from current_input + tool_results + thread_history]
    BuildInput --> Execute[rt.block_on executor.execute_turn]
    Execute -->|Err| InferenceError[Release gas guard]
    InferenceError --> ReturnError[Return TurnOutcome success=false]
    Execute -->|Ok response| Settle[Settle gas guard with actual token cost]
    Settle --> Extract[extract_tool_calls: structured first, text fallback second]
    Extract --> CheckTools{tool_calls empty?}
    CheckTools -->|Yes| CheckIter{iteration == 1 and has_tools?}
    CheckIter -->|Yes| Nudge[Inject tool-availability nudge]
    CheckIter -->|No| DisplayFinal[Display final response via sink.agent_text]
    Nudge --> DisplayFinal
    DisplayFinal --> AppendThread[Append turn to thread history]
    AppendThread --> MarkSeeded[Mark thread as seeded]
    MarkSeeded --> CnsUpdate[Run on_cns_update closure]
    CnsUpdate --> ReturnSuccess[Return TurnOutcome success=true]
    CheckTools -->|No| DisplayText[Display text portion via sink.agent_text]
    DisplayText --> InvokeTools[For each tool_call: deps.tools.invoke]
    InvokeTools --> FormatResults[format_tool_results]
    FormatResults --> SetInput[current_input = response]
    SetInput --> CheckMax{iteration > max_loops?}
    CheckMax -->|Yes| WarnMax[Warn: max iterations reached]
    WarnMax --> CnsUpdate
    CheckMax -->|No| LoopBack[Continue loop]
    LoopBack --> Reserve
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-REPL-001
verified_date: 2026-07-20
verified_against: crates/hkask-repl/src/turn.rs:130-307
status: VERIFIED
-->

## Key Properties

- **Gas regulation:** Every iteration reserves a heuristic estimate, then settles with the actual token cost. On inference error, the reservation is released (no cost incurred). The `EnergyGuard` logs a warning if dropped without settle/release (panic recovery).
- **Tool call priority:** Structured native function calls (`InferenceResult.tool_calls`) are checked first; `<<tool:...>>` text directives are the fallback. This supports both modern models (native function calling) and legacy models (text directives).
- **Thread seeding:** The thread is marked seeded only on successful (non-error) turns. Subsequent turns skip thread history injection — episodic recall handles conversation context.
- **CNS update:** The `on_cns_update` closure runs after the loop exits, checking algedonic alerts and ticking the LoopScheduler. This is the cybernetic feedback path from the turn back to the regulator.
- **Max iterations:** When `max_loops` is exceeded, the loop yields the current response (not an error). This prevents infinite tool-call loops from blocking the REPL indefinitely.

## Cross-References

- [REPL Specification §6 — Single-Agent Turn Pipeline](../specifications/REPL-specification.md#6-single-agent-turn-pipeline)
- [REPL Specification §10 — Gas Governance](../specifications/REPL-specification.md#10-gas-governance-energyguard)
- [Energy and Economy Explanation](../explanation/energy-and-economy.md)
