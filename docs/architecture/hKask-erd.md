---
title: "hKask Entity Relationship Diagram"
audience: [data architects, database developers, agents]
last_updated: 2026-05-22
togaf_phase: "C — Data"
version: "0.21.0"
status: "Active"
domain: "Data"
---

<!-- TOGAF_DOMAIN: Data -->
<!-- VERSION: 0.21.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-22 -->

# hKask Entity Relationship Diagram

**Version:** v0.21.0  
**Status:** Pre-alpha — MVP in progress

---

## Core Entities

```mermaid
erDiagram
    BOT ||--o{ MANIFEST : executes
    MANIFEST ||--|{ STEP : defines
    MANIFEST ||--o{ TEMPLATE : discovers
    TEMPLATE ||--o{ HLEXICON : grounded_by
    TEMPLATE ||--|{ FIELD : declares
    REGISTRY ||--o{ TEMPLATE : contains
    CNS ||--o{ OUTCOME : observes
    REGISTRY ||--o{ BOT : serves
    
    BOT {
        string name
        string type "Bot|Replicant"
        string manifest_ref
        array capabilities
        string editor "curator-or-human-admin"
    }
    
    MANIFEST {
        string id
        string name
        array steps
        string template_ref
        string model_tier "fast_local|balanced|high_quality"
        int matroshka_depth
    }
    
    STEP {
        int ordinal
        string action "select|populate|execute"
        string template_ref
        string renderer "minijinja"
        string model_tier
        string mcp
        object output_schema
    }
    
    TEMPLATE {
        string id
        string template_type "Prompt|Process|Cognition"
        array lexicon_terms
        object contract
        string source_path
        string content_type "jinja2|yaml|markdown"
    }
    
    FIELD {
        string name
        string type
        bool required
        string description
    }
    
    HLEXICON {
        string term
        string domain "WordAct|FlowDef|KnowAct"
        string definition
        string academic_citation
    }
    
    REGISTRY {
        string root_path
        string index_method "filesystem|sqlite"
        string template_type_filter
    }
    
    CNS {
        string span "cns.prompt.*|cns.tool.*|cns.agent_pod.*"
        object outcome
        float confidence
        timestamp emitted_at
    }
```

---

## Architecture Layers

```mermaid
graph TB
    subgraph "Soft Layer (Mutable — Outside LOC Budget)"
        YAML[Manifest Files<br/>.yaml]
        J2[Template Files<br/>.j2]
        MD[Cascade Files<br/>.yaml]
    end
    
    subgraph "Hard Layer (Fixed)"
        HKASK_TYPES[hkask-types<br/>ID types, ν-event, hLexicon]
        HKASK_STORAGE[hkask-storage<br/>SQLite + SQLCipher]
        HKASK_TEMPLATES[hkask-templates<br/>Registry, Manifest Executor]
        HKASK_CNS[hkask-cns<br/>Outcome ingestion, spans]
        HKASK_AGENTS[hkask-agents<br/>Pod lifecycle, ACP]
        HKASK_MCP[hkask-mcp<br/>MCP runtime]
    end
    
    subgraph "Testing (Single Crate — Outside LOC Budget)"
        HKASK_TESTING[hkask-testing<br/>Unit/Integration Tests<br/>Test Harnesses]
    end
    
    subgraph "External (Outside LOC Budget)"
        OKAPI[Okapi<br/>Inference orchestration]
        ACP[ACP Protocol<br/>acp-runtime]
        MCP[MCP Protocol<br/>rmcp]
    end
    
    YAML -. loads .- HKASK_TEMPLATES
    J2 -. renders .- HKASK_TEMPLATES
    MD -. composes .- HKASK_TEMPLATES
    
    HKASK_TEMPLATES --> HKASK_STORAGE
    HKASK_TEMPLATES --> HKASK_CNS
    HKASK_TEMPLATES --> HKASK_MCP
    HKASK_MCP --> OKAPI
    HKASK_AGENTS --> ACP
    HKASK_AGENTS --> HKASK_TEMPLATES
    
    HKASK_TESTING -. verifies .- HKASK_TYPES
    HKASK_TESTING -. verifies .- HKASK_STORAGE
    HKASK_TESTING -. verifies .- HKASK_TEMPLATES
    HKASK_TESTING -. verifies .- HKASK_CNS
    HKASK_TESTING -. verifies .- HKASK_AGENTS
```

---

## Data Flow: Dispatch Pattern

```mermaid
sequenceDiagram
    participant User
    participant CLI as CLI/API/ACP
    participant Bot as registry-dispatch-bot
    participant Manifest as dispatch.yaml
    participant Registry as RegistryIndex
    participant Selector as selector.j2
    participant FastLLM as Fast Local Model
    participant Template as Selected Template
    participant TargetLLM as Target Model/Tool
    participant CNS as CNS Span Emitter
    
    User->>CLI: Submit raw prompt
    CLI->>Bot: template:dispatch message
    Bot->>Manifest: Load dispatch.yaml
    Manifest->>Bot: Execute step 1 (select)
    Bot->>Registry: List available templates
    Registry-->>Bot: Return registry index
    Bot->>Selector: Render selector.j2 with index + raw_prompt
    Selector-->>Bot: Rendered selector prompt
    Bot->>FastLLM: Call with selector prompt
    FastLLM-->>Bot: {selected_template_id, rationale, confidence}
    Bot->>CNS: Emit cns.prompt.select span
    
    Bot->>Manifest: Execute step 2 (populate)
    Bot->>Registry: Get selected_template_id
    Registry-->>Bot: Return template
    Bot->>Template: Render with raw_prompt bindings
    Template-->>Bot: Rendered document
    
    Bot->>Manifest: Execute step 3 (execute)
    Bot->>TargetLLM: Submit rendered document
    TargetLLM-->>Bot: Result
    Bot->>CNS: Emit cns.prompt.render + cns.prompt.outcome spans
    
    Bot-->>CLI: Return result
    CLI-->>User: Display result
```

---

## Manifest Step Grammar

```mermaid
stateDiagram-v2
    [*] --> LoadManifest
    LoadManifest --> ExecuteStep
    ExecuteStep --> Select: action = "select"
    ExecuteStep --> Populate: action = "populate"
    ExecuteStep --> Execute: action = "execute"
    
    Select --> RenderTemplate: Load template_ref
    RenderTemplate --> CallModel: minijinja render
    CallModel --> EmitCNSSelect: inference.call()
    EmitCNSSelect --> NextStep
    
    Populate --> GetTemplate: Fetch selected_template_id
    GetTemplate --> BindFields: Jinja2 field binding
    BindFields --> NextStep
    
    Execute --> ResolveTarget: From template contract
    ResolveTarget --> InvokeTool: MCP tool or LLM
    InvokeTarget --> EmitCNSOutcome: Record outcome
    EmitCNSOutcome --> NextStep
    
    NextStep --> ExecuteStep: More steps
    NextStep --> [*]: All steps complete
    
    note right of ExecuteStep
        Matroshka depth enforced
        Default: 7, configurable
    end note
```

---

## CNS Span Hierarchy

```mermaid
graph LR
    subgraph "cns.prompt.*"
        CPS[cns.prompt.select]
        CPR[cns.prompt.render]
        CPO[cns.prompt.outcome]
    end
    
    subgraph "cns.tool.*"
        CTI[cns.tool.invocation]
        CTR[cns.tool.result]
    end
    
    subgraph "cns.agent_pod.*"
        CAP[cns.agent_pod.populated]
        CAR[cns.agent_pod.registered]
        CAA[cns.agent_pod.activated]
    end
    
    CPS --> CPR
    CPR --> CPO
    CPO --> CTI
    CTI --> CTR
    
    CAP --> CAR
    CAR --> CAA
    
    style CPS fill:#f9f,stroke:#333
    style CPR fill:#f9f,stroke:#333
    style CPO fill:#f9f,stroke:#333
```

---

## Key Invariants

| Invariant | Description | Enforcement |
|-----------|-------------|-------------|
| **Loom/Thread** | Rust is fixed logic; YAML/Jinja2 is mutable content | Architecture boundary |
| **Unified Registry** | Single registry with `template_type` discriminator | P1 (no trait without 2 consumers) |
| **Manifest Execution** | Generic step interpreter applies to any manifest | ~50 LOC core loop |
| **Matroshka Depth** | Recursion limit enforced across all template chains | Rust executor |
| **CNS Observation** | All template outcomes emitted as spans | Port requirement |
| **hLexicon Grounding** | Templates declare terms; validator checks existence | Render-time check |

---

## CNS (Cybernetic Nervous System) ERD

The cybernetic core — ν-events, observers, and algedonic alerts:[^beer-vsm][^ashby-law]

```mermaid
erDiagram
    CYBERNETIC_EVENT ||--|| OBSERVER_REF : "produced_by"
    CYBERNETIC_EVENT ||--|| CYBERNETIC_PHASE : "in"
    CYBERNETIC_EVENT ||--|| OBSERVATION : "contains"
    CYBERNETIC_EVENT ||--o{ REGULATION : "computes"
    CYBERNETIC_EVENT ||--o{ ACTION : "triggers"
    CYBERNETIC_EVENT ||--o{ OUTCOME : "yields"
    CYBERNETIC_EVENT ||--o{ CYBERNETIC_EVENT : "parent_event"
    
    CYBERNETIC_PHASE: "Observe"
    CYBERNETIC_PHASE: "Regulate"
    CYBERNETIC_PHASE: "Act"
    CYBERNETIC_PHASE: "MetaObserve"
    
    OBSERVER_REF {
        string webid "Observer identity"
        string role "bot|replicant|human"
    }
    
    OBSERVATION {
        json telemetry "Raw sensor data"
        json pattern "Recognized patterns"
        json state_estimate "Current state"
    }
    
    REGULATION {
        json contract "Expected behavior"
        json error_signal "Deviation from contract"
        string corrective_action "Action to restore equilibrium"
    }
    
    ACTION {
        string tool_invocation "Tool called"
        string template_render "Template rendered"
        string memory_write "Memory updated"
    }
    
    OUTCOME {
        json result "Execution result"
        float confidence "Bayesian confidence"
        timestamp completed_at "Completion time"
    }
    
    CYBERNETIC_EVENT {
        uuid id
        timestamp emitted_at
        uuid parent_event
        int recursion_depth
        int variety_counter
        bool algedonic_alert "Variety deficit >100"
    }
```

**Cybernetic Flow:**
1. **Observe** — Telemetry capture, pattern recognition, state estimation
2. **Regulate** — Contract validation, error signal computation, corrective action
3. **Act** — Tool invocation, template render, memory write
4. **Meta-Observe** — Observe the observation (second-order cybernetics)

**Algedonic Alert:** Triggers when variety deficit >100, escalates to Curator/human.

---

## References

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model.
[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Law of Requisite Variety.

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*The Rust is the loom. The YAML/Jinja2 is the thread.*
*MVP in progress.*
