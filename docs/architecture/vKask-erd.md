# νKask Entity Relationship Diagram

## The Cybernetic Triad — Core Flow

```mermaid
graph TB
    subgraph "OBSERVE (ν-observe)"
        O1[Telemetry Capture]
        O2[Pattern Recognition]
        O3[State Estimation]
    end
    
    subgraph "REGULATE (ν-regulate)"
        R1[Contract Validation]
        R2[Error Signal]
        R3[Corrective Action]
    end
    
    subgraph "ACT (ν-act)"
        A1[Tool Invocation]
        A2[Template Render]
        A3[Memory Write]
    end
    
    subgraph "META-OBSERVE (ν-meta)"
        M1[Observe Observation]
        M2[Observer State]
        M3[Blind Spot Detection]
    end
    
    O1 --> R1
    O2 --> R2
    O3 --> R3
    
    R1 --> A1
    R2 --> A2
    R3 --> A3
    
    A1 --> M1
    A2 --> M2
    A3 --> M3
    
    M1 -.->|recursive loop | O1
    M2 -.->|recursive loop | O2
    M3 -.->|recursive loop | O3
    
    style O1 fill:#e1f5fe
    style O2 fill:#e1f5fe
    style O3 fill:#e1f5fe
    style R1 fill:#fff3e0
    style R2 fill:#fff3e0
    style R3 fill:#fff3e0
    style A1 fill:#e8f5e9
    style A2 fill:#e8f5e9
    style A3 fill:#e8f5e9
    style M1 fill:#f3e5f5
    style M2 fill:#f3e5f5
    style M3 fill:#f3e5f5
```

## νKask System Architecture

```mermaid
erDiagram
    %% ===========================================
    %% CYBERNETIC CORE
    %% ===========================================
    
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
    
    OBSERVER_REF ||--|| POD_ID : "identifies"
    OBSERVER_REF ||--|| AGENT_WEB_ID : "sovereign_identity"
    OBSERVER_REF ||--o{ TEMPLATE_REF : "uses"
    OBSERVER_REF ||--|| OBSERVATION_CHANNEL : "senses_via"
    
    OBSERVATION_CHANNEL: "Telemetry"
    OBSERVATION_CHANNEL: "MemoryRecall"
    OBSERVATION_CHANNEL: "ToolOutput"
    OBSERVATION_CHANNEL: "UserInput"
    OBSERVATION_CHANNEL: "MetaCognition"
    
    OBSERVATION: "TelemetryCapture"
    OBSERVATION: "PatternRecognition"
    OBSERVATION: "StateEstimation"
    OBSERVATION: "ContractValidation"
    OBSERVATION: "Outcome"
    
    %% ===========================================
    %% VARIETY ENGINEERING
    %% ===========================================
    
    VARIETY_COUNTER ||--|| POD_ID : "tracks"
    VARIETY_COUNTER ||--|| VARIETY_DISTURBANCE : "measures"
    VARIETY_COUNTER ||--|| VARIETY_REGULATOR : "measures"
    VARIETY_COUNTER ||--|| VARIETY_REQUIRED : "computes"
    VARIETY_COUNTER ||--|| VARIETY_DEFICIT : "computes"
    
    VARIETY_DISTURBANCE: "V(D) — environmental"
    VARIETY_REGULATOR: "V(R) — regulatory"
    VARIETY_REQUIRED: "V(min) — Ashby's law"
    VARIETY_DEFICIT: "V(R) - V(required)"
    
    VARIETY_COUNTER ||--o{ ALGEDONIC_ALERT : "triggers"
    
    ALGEDONIC_ALERT ||--|| ALERT_LEVEL : "has"
    ALGEDONIC_ALERT ||--|| ALERT_CONTEXT : "contains"
    
    ALERT_LEVEL: "Info"
    ALERT_LEVEL: "Warning"
    ALERT_LEVEL: "Critical"
    ALERT_LEVEL: "Emergency"
    
    ALERT_CONTEXT ||--|| SUGGESTED_ACTION : "recommends"
    ALERT_CONTEXT ||--|| ESCALATION_PATH : "defines"
    
    ESCALATION_PATH: "Log → Pod → User → Suspend"
    
    %% ===========================================
    %% CYBERNETIC MONITOR
    %% ===========================================
    
    CYBERNETIC_MONITOR ||--o{ CYBERNETIC_EVENT : "records"
    CYBERNETIC_MONITOR ||--o{ VARIETY_COUNTER : "maintains"
    CYBERNETIC_MONITOR ||--|| ALGEDONIC_HANDLER : "invokes"
    CYBERNETIC_MONITOR ||--|| BITEMPORAL_STORE : "audits_to"
    CYBERNETIC_MONITOR ||--|| KAPPA : "enforces"
    
    KAPPA: "κ — cybernetic constant"
    KAPPA: "minimum cycle time (100ms default)"
    
    CYBERNETIC_MONITOR ||--o{ PASS_RATE_METRIC : "computes"
    CYBERNETIC_MONITOR ||--o{ REGRESSION_ALERT : "detects"
    
    PASS_RATE_METRIC ||--|| TEMPLATE_REF : "for"
    PASS_RATE_METRIC ||--|| TIME_WINDOW : "over"
    
    REGRESSION_ALERT ||--|| THRESHOLD_BREACH : "indicates"
    THRESHOLD_BREACH: "pass_rate drop > 10%"
    
    %% ===========================================
    %% VSM MAPPING (Beer's Viable System Model)
    %% ===========================================
    
    VSM_SYSTEM1 ||--|| MCP_TOOLS : "implements"
    VSM_SYSTEM1 ||--|| AGENT_PODS : "implements"
    VSM_SYSTEM1: "Operations — doing the work"
    
    VSM_SYSTEM2 ||--|| TEMPLATE_REGISTRY : "implements"
    VSM_SYSTEM2: "Coordination — preventing conflicts"
    
    VSM_SYSTEM3 ||--|| CYBERNETIC_MONITOR : "implements"
    VSM_SYSTEM3: "Control — here-and-now regulation"
    
    VSM_SYSTEM4 ||--|| META_COGNITION : "implements"
    VSM_SYSTEM4: "Intelligence — there-and-then adaptation"
    
    VSM_SYSTEM5 ||--|| USER_SOVEREIGNTY : "implements"
    VSM_SYSTEM5: "Policy — identity, ultimate authority"
    
    VSM_SYSTEM3 ||--|| VSM_SYSTEM1 : "controls"
    VSM_SYSTEM4 ||--|| VSM_SYSTEM3 : "advises"
    VSM_SYSTEM5 ||--|| VSM_SYSTEM4 : "directs"
    VSM_SYSTEM5 ||--|| VSM_SYSTEM1 : "policy_to"
    
    %% ===========================================
    %% RECURSION & META-COGNITION
    %% ===========================================
    
    CYBERNETIC_EVENT ||--|| RECURSION_DEPTH : "has"
    
    RECURSION_DEPTH: "0 = first-order"
    RECURSION_DEPTH: "1 = second-order"
    RECURSION_DEPTH: "2+ = higher-order"
    
    META_COGNITION ||--|| CYBERNETIC_EVENT : "observes"
    META_COGNITION ||--|| BLIND_SPOT_DETECTION : "performs"
    META_COGNITION ||--|| OBSERVER_STATE : "tracks"
    
    BLIND_SPOT_DETECTION: "not seeing that we do not see"
    
    OBSERVER_STATE ||--|| CONFIDENCE : "measures"
    OBSERVER_STATE ||--|| ATTENTION : "tracks"
    OBSERVER_STATE ||--|| BIAS : "detects"
    
    %% ===========================================
    %% AUTOPÔIETIC CLOSURE
    %% ===========================================
    
    TEMPLATE ||--o{ CYBERNETIC_EVENT : "generates"
    CYBERNETIC_EVENT ||--o{ OUTCOME : "produces"
    OUTCOME ||--o{ TEMPLATE : "updates_quality_of"
    
    AGENT_POD ||--o{ AGENT_POD : "creates_via_delegation"
    AGENT_POD ||--|| UCAN_TOKEN : "attenuates"
    
    CYBERNETIC_MONITOR ||--o{ CYBERNETIC_MONITOR : "self_configures"
    
    %% ===========================================
    %% BITEMPORAL AUDIT TRAIL
    %% ===========================================
    
    BITEMPORAL_STORE ||--o{ DATOM : "contains"
    DATOM ||--|| CYBERNETIC_EVENT : "audits"
    
    DATOM ||--|| ENTITY_ID : "identifies"
    DATOM ||--|| VALID_TIME : "valid_at"
    DATOM ||--|| TRANSACTION_TIME : "recorded_at"
    
    VALID_TIME: "when event occurred"
    TRANSACTION_TIME: "when recorded"
```

## Variety Engineering — Ashby's Law Visualization

```mermaid
graph LR
    subgraph "Environment (Disturbances)"
        D1[Tool Failures]
        D2[LLM Errors]
        D3[User Requests]
        D4[Memory Misses]
        D5[Contract Violations]
    end
    
    subgraph "Regulator (νKask)"
        R1[Template Selection]
        R2[Agent Routing]
        R3[Capability Gating]
        R4[Meta-Cognition]
        R5[Algedonic Alerts]
    end
    
    subgraph "Outcomes (Essential Variables)"
        E1[Task Success]
        E2[System Viability]
        E3[User Sovereignty]
    end
    
    D1 --> T
    D2 --> T
    D3 --> T
    D4 --> T
    D5 --> T
    
    R1 --> T
    R2 --> T
    R3 --> T
    R4 --> T
    R5 --> T
    
    T --> E1
    T --> E2
    T --> E3
    
    T{{Transformation<br/>Table}}
    
    style D1 fill:#ffccbc
    style D2 fill:#ffccbc
    style D3 fill:#ffccbc
    style D4 fill:#ffccbc
    style D5 fill:#ffccbc
    
    style R1 fill:#c8e6c9
    style R2 fill:#c8e6c9
    style R3 fill:#c8e6c9
    style R4 fill:#c8e6c9
    style R5 fill:#c8e6c9
    
    style E1 fill:#bbdefb
    style E2 fill:#bbdefb
    style E3 fill:#bbdefb
```

## Algedonic Alert Escalation Path

```mermaid
flowchart TD
    A[Variety Deficit Detected] --> B{Deficit > Threshold?}
    
    B -->|No| C[Log to Audit Trail]
    B -->|Yes| D{Deficit Growing?}
    
    D -->|No| E[Info Alert<br/>Monitor]
    D -->|Yes| F{Pod Can<br/>Self-Regulate?}
    
    F -->|Yes| G[Warning Alert<br/>Notify Pod]
    F -->|No| H{User Available?}
    
    H -->|Yes| I[Critical Alert<br/>Escalate to System 5]
    H -->|No| J[Emergency Alert<br/>Suspend Pod]
    
    G --> K[Pod Self-Regulates]
    K --> L{Variety Restored?}
    
    L -->|Yes| M[Alert Cleared]
    L -->|No| H
    
    I --> N[User Intervention]
    N --> O{User Resolves?}
    
    O -->|Yes| M
    O -->|No| J
    
    style A fill:#ffccbc
    style C fill:#c8e6c9
    style E fill:#fff9c4
    style G fill:#ffe0b2
    style I fill:#ffccbc
    style J fill:#ef9a9a
    style M fill:#c8e6c9
```

## Second-Order Observation — Recursive Loop

```mermaid
graph TB
    subgraph "First-Order Observation"
        O1[Observe Environment]
        R1[Regulate Response]
        A1[Act on World]
    end
    
    subgraph "Second-Order Observation"
        O2[Observe O1]
        R2[Regulate R1]
        A2[Act on Observation]
    end
    
    subgraph "Third-Order Observation"
        O3[Observe O2]
        R3[Regulate R2]
        A3[Act on Meta-Observation]
    end
    
    O1 --> R1 --> A1
    A1 --> O2
    O2 --> R2 --> A2
    A2 --> O3
    O3 --> R3 --> A3
    
    O1 -.->|blind spot| BS1[Blind Spot 1]
    O2 -.->|blind spot| BS2[Blind Spot 2]
    O3 -.->|blind spot| BS3[Blind Spot 3]
    
    R2 -.->|corrects| BS1
    R3 -.->|corrects| BS2
    
    style O1 fill:#e1f5fe
    style O2 fill:#f3e5f5
    style O3 fill:#fce4ec
    
    style BS1 fill:#ffcdd2
    style BS2 fill:#ffcdd2
    style BS3 fill:#ffcdd2
```

## νKask Crate Dependency Graph

```mermaid
graph TD
    subgraph "Arsenal (5 crates)"
        AW[arsenal-web]
        AS[arsenal-scholar]
        AF[arsenal-forecast]
        ASP[arsenal-spandrel]
    end
    
    subgraph "Stack - Surface (6 crates)"
        HM[hkask-mcp-memory]
        HL[hkask-mcp-llm]
        HP[hkask-mcp-prompts]
        HF[hkask-mcp-feedback]
        HO[hkask-mcp-observability]
        HC[hkask-cli]
    end
    
    subgraph "Stack - Integration (4 crates)"
        HA[hkask-agents]
        HT[hkask-templates]
        HM2[hkask-memory]
        HK[hkask-keystore]
    end
    
    subgraph "Stack - Domain (3 crates)"
        HB[hkask-bitemporal]
        HS[hkask-store]
        HY[hkask-types]
        HC2[hkask-cybernetics]
    end
    
    AW --> HM
    AS --> HM
    AF --> HT
    ASP --> HM
    
    HM --> HM2
    HL --> HB
    HP --> HT
    HF --> HC2
    HO --> HC2
    
    HA --> HT
    HA --> HK
    HT --> HB
    HT --> HM2
    HM2 --> HS
    HS --> HB
    HB --> HY
    HB --> HC2
    HT --> HC2
    HA --> HC2
```

## Cybernetic Event Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Observing: Start cycle (κ = 100ms)
    
    Observing --> Regulating: Observation captured
    Regulating --> Acting: Error signal computed
    Acting --> MetaObserving: Action completed
    MetaObserving --> Observing: Recursive loop (depth+1)
    
    MetaObserving --> [*]: Cycle complete
    Regulating --> [*]: Regulation failed (algedonic)
    Acting --> [*]: Action blocked (capability)
    
    state Observing {
        [*] --> TelemetryCapture
        TelemetryCapture --> PatternRecognition
        PatternRecognition --> StateEstimation
        StateEstimation --> [*]
    }
    
    state Regulating {
        [*] --> ContractValidation
        ContractValidation --> ErrorComputation
        ErrorComputation --> ActionSelection
        ActionSelection --> [*]
    }
    
    state Acting {
        [*] --> ToolInvocation
        ToolInvocation --> TemplateRender
        TemplateRender --> MemoryWrite
        MemoryWrite --> [*]
    }
    
    state MetaObserving {
        [*] --> ObserveObservation
        ObserveObservation --> BlindSpotCheck
        BlindSpotCheck --> ObserverStateUpdate
        ObserverStateUpdate --> [*]
    }
```
