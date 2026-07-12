---
title: "Kata-Kanban Execution Boundary"
audience: [architects, developers, maintainers]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Kata-Kanban"
mds_categories: [domain, composition, trust, lifecycle, curation]
diataxis: reference
---

# Kata-Kanban Execution Boundary

This reference sequence separates the two currently implemented Kata paths. The Kanban MCP exposes task-scoped **prompt generation**. Full Kata execution is available only through an optional `KanbanKataBridge` configured on `KanbanService`; the shown MCP tools do not invoke that bridge. The distinction is operationally important because prompt generation does not execute the manifest’s convergence, budget, or OCAP declarations.

```mermaid
sequenceDiagram
    participant Caller
    participant MCP as Kanban MCP
    participant Service as KanbanService
    participant Task as Task Store
    participant Bridge as KanbanKataBridge
    participant Engine as KataEngine

    Caller->>+MCP: kanban_task_kata_improvement(task_id)
    MCP->>+Service: task_improvement_prompt(task_id)
    Service->>+Task: task_get(task_id)
    Task-->>-Service: Task
    Service-->>-MCP: rendered prompt text
    MCP-->>-Caller: TaskKataResponse

    opt Service configured with KataEngine
        Caller->>+Service: run_improvement_kata(task_id, manifest)
        Service->>+Task: task_get(task_id)
        Task-->>-Service: Task
        Service->>+Bridge: run_improvement_on_task(task, manifest)
        Bridge->>+Engine: execute(manifest, learner, context)
        Engine-->>-Bridge: KataResult
        Bridge-->>-Service: KataResult
        Service-->>-Caller: KataResult
    end

    Note over MCP,Engine: The MCP prompt tools do not enter the optional bridge path.
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-FW-006
verified_date: 2026-07-10
verified_against: mcp-servers/hkask-mcp-kata-kanban/src/lib.rs:680-780; crates/hkask-services-kata-kanban/src/kanban/service_impl/kata.rs:1-210; crates/hkask-services-kata-kanban/src/bridge.rs:18-76; crates/hkask-services-kata-kanban/src/kata/mod.rs:334-498
status: VERIFIED
-->

## Cross-reference


- [Kata PDCA lifecycle state machine](state-kata-pdca.md)
- [Architecture master: Kata](../architecture/hKask-architecture-master.md#kata--cybernetic-capability-development)
