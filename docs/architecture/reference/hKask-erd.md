---
title: "hKask Entity Relationship Diagram"
audience: [data architects, database developers, agents]
last_updated: 2026-06-05
version: "0.23.00"
status: "Active"
domain: "Data"
ddmvss_categories: [persistence]
---


# hKask Entity Relationship Diagram

**Version:** v0.23.00
**Status:** Pre-alpha — MVP in progress

---

## Contents

| Section | Description |
|---------|-------------|
| [Core Entities](#core-entities) | Primary entity relationships in the data model |
| [Architecture Layers](#architecture-layers) | Three-tier architecture layer ERD |
| [Data Flow: Dispatch Pattern](#data-flow-dispatch-pattern) | MCP dispatch data flow diagram |
| [Manifest Step Grammar](#manifest-step-grammar) | Step grammar entity relationships |
| [CNS Span Hierarchy](#cns-span-hierarchy) | CNS observability span hierarchy |
| [Key Invariants](#key-invariants) | Data model invariants and constraints |
| [CNS ERD](#cns-cybernetic-nervous-system-erd) | Cybernetic Nervous System entity relationships |
| [References](#references) | Citations and references |

---

## Core Entities

Core entity relationships in the hKask data model, following the entity-relationship approach to data modeling:[^chen-er]

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
        string action "select|populate|execute|feedback|validate|retrieve"
        string template_ref
        string renderer "minijinja"
        string model_tier
        string mcp
        object output_schema
        object feedback
        object validation_rules
    }
    
    TEMPLATE {
        string id
        string template_type "WordAct|KnowAct|FlowDef"
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
        string span "cns.* (15 canonical namespaces)"
        object outcome
        float confidence
        timestamp emitted_at
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-ERD-001
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/; crates/hkask-templates/src/; crates/hkask-agents/src/
status: VERIFIED
-->

---

## Architecture Layers

The hKask codebase is organized into mutable and fixed layers, adhering to the layered architecture pattern:[^fowler-layers]

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

<!-- DIAGRAM_ALIGNMENT
id: DIAG-ERD-002
verified_date: 2026-05-24
verified_against: Cargo.toml workspace members; crates/*/src/lib.rs
status: VERIFIED
-->

---

## Data Flow: Dispatch Pattern

Template dispatch follows a message routing pattern common in enterprise integration:[^hohpe-eip]

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

<!-- DIAGRAM_ALIGNMENT
id: DIAG-ERD-003
verified_date: 2026-05-24
verified_against: crates/hkask-mcp/src/dispatch.rs; crates/hkask-templates/src/manifest.rs
status: VERIFIED
-->

---

## Manifest Step Grammar

Step execution follows an interpreter pattern where each action type maps to a discrete execution strategy:[^gamma-patterns]

```mermaid
stateDiagram-v2
    [*] --> LoadManifest
    LoadManifest --> ExecuteStep
    ExecuteStep --> Select: action = "select"
    ExecuteStep --> Populate: action = "populate"
    ExecuteStep --> Execute: action = "execute"
    ExecuteStep --> Feedback: action = "feedback"
    ExecuteStep --> Validate: action = "validate"
    ExecuteStep --> Retrieve: action = "retrieve"

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

    Feedback --> InvokeTool: MCP tool (CNS emit)
    Validate --> InvokeTool: MCP tool (validation)
    Retrieve --> InvokeTool: MCP tool (semantic search)

    NextStep --> ExecuteStep: More steps
    NextStep --> [*]: All steps complete

    note right of ExecuteStep
        Matroshka depth enforced
        Default: 7, configurable
    end note
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-ERD-004
verified_date: 2026-06-06
verified_against: crates/hkask-templates/src/executor.rs, crates/hkask-types/src/bundle.rs
status: VERIFIED
-->

---

## CNS Span Hierarchy

CNS spans form a hierarchical observability structure grounded in cybernetic regulation theory:[^ashby-law]

```mermaid
    graph LR
        subgraph "cns.prompt.*"
            CPS[cns.prompt.select]
            CPR[cns.prompt.render]
            CPO[cns.prompt.outcome]
        end
        
        subgraph "cns.tool.*"
            CTI[cns.tool.invoked]
            CTR[cns.tool.completed]
        end
        
        subgraph "cns.inference.*"
            CINF[cns.inference.regulate]
        end
        
        subgraph "cns.agent_pod.*"
            CAP[cns.agent_pod.populated]
            CAR[cns.agent_pod.registered]
            CAA[cns.agent_pod.activated]
        end
        
        subgraph "cns.connector.*"
            CCI[cns.connector.llm]
            CCE[cns.connector.embedding]
        end
        
        subgraph "cns.pipeline.*"
            CPL[cns.pipeline.execute]
        end
        
        subgraph "cns.gas.*"
            CGAS[cns.gas.degradation]
            CGASR[cns.gas.message_rejected]
        end
        
        subgraph "cns.review.*"
            CREV[cns.review.evaluate]
        end
        
        subgraph "cns.template.*"
            CTPL[cns.template.invoke]
            CTCS[cns.template.cascade]
        end
        
        subgraph "cns.curation.*"
            CCUR[cns.curation.decide]
        end
        
        subgraph "cns.variety.*"
            CVAR[cns.variety.counter]
            CALG[cns.variety.algedonic]
        end
        
        subgraph "cns.killzone.*"
            CKZ[cns.killzone.detect]
        end
        
        subgraph "cns.sovereignty.*"
            CSOV[cns.sovereignty.check]
        end
        
        subgraph "cns.goal.*"
            CGOAL[cns.goal.spec]
            CGOALD[cns.goal.capability.denied]
        end
        
        subgraph "cns.spec.*"
            CSPEC[cns.spec.capture]
            CSPECV[cns.spec.validate]
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

<!-- DIAGRAM_ALIGNMENT
id: DIAG-ERD-005
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/cns.rs:122-145; crates/hkask-types/src/event.rs:75-86
status: VERIFIED
-->

---

## Key Invariants

System invariants define the unchanging design contracts that preserve architectural integrity:[^meyer-contract]

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
    
    CYBERNETIC_PHASE: "Sense"
    CYBERNETIC_PHASE: "Compute"
    CYBERNETIC_PHASE: "Compare"
    CYBERNETIC_PHASE: "Act"
    
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
        bool algedonic_alert "Variety deficit >50 (Warning) / >100 (Critical)"
    }
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-ERD-007
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/event.rs:10-22,148-152; crates/hkask-cns/src/
status: VERIFIED
-->

**Cybernetic Flow:**
1. **Sense** — Telemetry capture, pattern recognition, state estimation
2. **Compute** — Contract validation, error signal computation, corrective action
3. **Compare** — Deviation assessment against set-points, variety measurement
4. **Act** — Tool invocation result, confidence scoring, memory write

**Algedonic Alert:** Warning escalation to Curator when variety deficit >50; Critical escalation to human when deficit >100.

---

## References

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model.
[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Law of Requisite Variety.
[^chen-er]: Chen, P. P.-S. (1976). The entity-relationship model—Toward a unified view of data. *ACM Transactions on Database Systems*, 1(1), 9–36. https://doi.org/10.1145/320434.320440
[^fowler-layers]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture*. Addison-Wesley. Layered architecture pattern.
[^hohpe-eip]: Hohpe, G., & Woolf, B. (2003). *Enterprise Integration Patterns: Designing, Building, and Deploying Messaging Solutions*. Addison-Wesley. Message dispatch and routing patterns.
[^gamma-patterns]: Gamma, E., Helm, R., Johnson, R., & Vlissides, J. (1994). *Design Patterns: Elements of Reusable Object-Oriented Software*. Addison-Wesley. Interpreter and command patterns.
[^meyer-contract]: Meyer, B. (1997). *Object-Oriented Software Construction* (2nd ed.). Prentice Hall. Design by contract and class invariants.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.00*
*The Rust is the loom. The YAML/Jinja2 is the thread.*
*MVP in progress.*
