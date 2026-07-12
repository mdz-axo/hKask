---
title: "Template Manifest Cascade Execution"
audience: [architects, developers, agents]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Templates"
mds_categories: [domain, execution, curation]
diataxis: "reference"
---

# Template Manifest Cascade Execution

## Description

The `ManifestExecutor` in `hkask-templates` drives the select → populate → execute cascade for `BundleManifest` execution. Steps are sorted by ordinal and dispatched by `action`: **select** (render selector template → inference → parse JSON result), **populate** (render template with context map → produce filled prompt), and **execute** (invoke MCP tool via `McpPort` with context-bound parameters). Three template types compose the skill taxonomy: **WordAct** renders system prompts, **FlowDef** orchestrates multi-step PDCA cascades, and **KnowAct** drives metacognition decisions. The recursive cascade is bounded by the matryoshka depth limit (`SYSTEM_MAX_RECURSION` = 7). Every execute step routes through `GovernedTool` for energy accounting (gas + rJoule budgets). The PDCA convergence loop re-enters from `loop_target` until the threshold is met, max iterations exhausted, or `abort`/`escalate` is triggered.

**Key source:** `crates/hkask-templates/src/executor.rs:69-84` (`ManifestExecutor` struct), `executor.rs:209-686` (`execute_manifest` cascade loop), `executor.rs:230` (`matryoshka_limit = SYSTEM_MAX_RECURSION`), `executor.rs:386-475` (loop action with depth guard), `executor.rs:232-245` (gas + rJoule tracking), `crates/hkask-capability/src/token_types.rs:14` (`SYSTEM_MAX_RECURSION = 7`).

```mermaid
flowchart TD
    Start(["execute_manifest(manifest, context)"]) --> Sort[Sort steps by ordinal]
    Sort --> Init[Initialize convergence context<br/>gas_cap / rjoule_cap / matryoshka_limit=7]
    Init --> Loop{"cascade loop<br/>iteration ≤ max_iterations?"}

    Loop -->|yes| StepDispatch{"step.action?"}

    %% ── abort / escalate (terminal exits) ──
    StepDispatch -->|"abort"| Converged(["exit: converged<br/>cns.skill.converged"])
    StepDispatch -->|"escalate"| Escalated(["exit: escalated<br/>cns.skill.escalated"])

    %% ── choice (branching) ──
    StepDispatch -->|"choice"| EvalChoice[evaluate_choice()<br/>parse condition → target ordinal]
    EvalChoice --> Jump{target found?}
    Jump -->|yes| StepDispatch
    Jump -->|no| NextStep

    %% ── loop (recursive re-entry) ──
    StepDispatch -->|"loop"| IncDepth[recursion_depth += 1]
    IncDepth --> DepthCheck{"depth > matryoshka_limit (7)?"}
    DepthCheck -->|yes| DepthExceeded(["exit: maxed_out<br/>Matryoshka depth exceeded"])
    DepthCheck -->|no| CheckConv{convergence met?<br/>or max_iterations exhausted?}
    CheckConv -->|yes| MaxedOut(["exit: maxed_out<br/>energy_spent"])
    CheckConv -->|no| Reenter[Reset step_idx → loop_target<br/>continue cascade]
    Reenter --> StepDispatch

    %% ── select (template → inference → parse) ──
    StepDispatch -->|"select"| RenderSelect[render_step_template()<br/>minijinja or inline]
    RenderSelect --> Infer[inference.generate()<br/>LLMParameters + timeout]
    Infer --> Parse[parse_json_response()]
    Parse --> UpdateCtx[Update context map]
    UpdateCtx --> DeductGas["Deduct gas + rJoule<br/>per-token energy accounting"]

    %% ── populate (template fill) ──
    StepDispatch -->|"populate"| RenderPop[render_step_template()<br/>fill template with context]
    RenderPop --> UpdateCtxPop[Store populated result<br/>in context map]

    %% ── execute (MCP tool invoke) ──
    StepDispatch -->|"execute"| BindParams[bind_parameters()<br/>resolve dot-path refs]
    BindParams --> GovTool[GovernedTool.invoke()<br/>OCAP: DelegationToken check<br/>energy budget debited]
    GovTool --> ToolResult[Tool result → context map]
    ToolResult --> DeductGasExec["Deduct gas + rJoule"]

    %% ── convergence & budget checks ──
    DeductGas --> GasCheck{"gas_hard_limit &<br/>gas_used ≥ gas_cap?"}
    GasCheck -->|yes| GasExhausted(["exit: maxed_out<br/>cns.skill.gas_exhausted"])
    GasCheck -->|no| RJCheck{"rjoule_hard_limit &<br/>rjoule_used ≥ rjoule_cap?"}
    RJCheck -->|yes| RJExhausted(["exit: maxed_out<br/>cns.skill.rjoule_exhausted"])
    RJCheck -->|no| NextStep[step_idx += 1<br/>advance to next step]

    DeductGasExec --> GasCheck

    UpdateCtxPop --> NextStep

    NextStep --> MoreSteps{"more steps?"}
    MoreSteps -->|yes| StepDispatch
    MoreSteps -->|no| EndIter{"iteration ≥ max_iterations?"}
    EndIter -->|yes| MaxedOut
    EndIter -->|no| Loop

    Loop -->|no, cascading| MaxedOut

    %% ── Template type taxonomy (legend) ──
    subgraph TemplateTypes["Template Types (WordAct / FlowDef / KnowAct)"]
        WA[WordAct<br/>render system prompt<br/>agent persona + charter]
        FD[FlowDef<br/>orchestrate PDCA steps<br/>select → populate → execute]
        KA[KnowAct<br/>metacognition decisions<br/>single-template invocation]
    end

    WA -.->|"rendered by"| RenderSelect
    FD -.->|"cascade orchestrated by"| StepDispatch
    KA -.->|"invoked via"| Infer

    %% ── Dependency graph constraint ──
    subgraph DepCheck["Acyclic Dependency Constraint"]
        DepGraph["steps form a DAG<br/>ordinal ordering enforced<br/>no cyclic dependencies<br/>loop re-entry is bounded"]
    end

    Sort -.-> DepGraph
```

## Template Type Taxonomy

| Type | Purpose | Invocation | Example |
|------|---------|------------|---------|
| **WordAct** | Render system prompt / persona | Step `action: "populate"` with persona context | Agent persona YAML → system prompt |
| **FlowDef** | Orchestrate multi-step PDCA cascade | `execute_manifest()` full cascade | Kata improvement cycles, skill bundles |
| **KnowAct** | Single-template metacognition decision | `execute_knowact()` direct call | `metacognition-diagnose.j2`, `metacognition-escalate.j2` |

## Energy Accounting (Gas + rJoule)

Every select and execute step deducts from two parent-allocated budgets:

| Budget | Unit | Deduction | Hard Limit |
|--------|------|-----------|------------|
| **Gas** | `u64` cycles | `gas_cost_per_iter` per step + per-token inference cost | `gas_hard_limit`: cascade aborts if `gas_used ≥ gas_cap` |
| **rJoule** | `f64` energy | Per-token cost from inference provider/model config | `rjoule_hard_limit`: cascade aborts if `rjoule_used ≥ rjoule_cap` |

Alert thresholds (`gas_alert_threshold`, `rjoule_alert_threshold`) fire CNS warnings once per cascade when usage exceeds the threshold fraction.

## Matryoshka Depth Limit

The recursive cascade is bounded by `SYSTEM_MAX_RECURSION` (7), shared across all depth-constrained systems:

- Capability attenuation chain: max 7 levels (`SYSTEM_MAX_ATTENUATION`)
- Template cascade recursion: max 7 nested loops
- Improv cascade: max 7 total mode applications
- Goal sub-goal nesting: max 7 levels

When `recursion_depth > matryoshka_limit`, the cascade exits with `maxed_out` / `energy_spent`.

---

<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-004
verified_date: 2026-07-01
verified_against: crates/hkask-templates/src/executor.rs (ManifestExecutor:69-84, execute_manifest:209-686, matryoshka_limit:230, loop action:386-475, select:478-512, gas tracking:232-245/493-526, rJoule tracking:240-245/528-545), crates/hkask-capability/src/token_types.rs (SYSTEM_MAX_RECURSION:14), crates/hkask-api/src/routes/templates.rs (TemplateResponse:30-40, WordAct/FlowDef/KnowAct taxonomy:27-28), crates/hkask-improv/src/cascade.rs (MATRYOSHKA_LIMIT:17-21)
status: VERIFIED
-->

## Cross-Reference

- [`hKask-architecture-master.md` § Template System & Cascade Execution](../architecture/hKask-architecture-master.md#template-system--cascade-execution)
- [`executor.rs`](crates/hkask-templates/src/executor.rs) — `ManifestExecutor`, `execute_manifest()`, PDCA cascade loop
- [`token_types.rs`](crates/hkask-capability/src/token_types.rs) — `SYSTEM_MAX_RECURSION` = 7
- [`cascade.rs`](crates/hkask-improv/src/cascade.rs) — `MATRYOSHKA_LIMIT`, improv cascade
- [Template registry routes](crates/hkask-api/src/routes/templates.rs) — WordAct / FlowDef / KnowAct API
