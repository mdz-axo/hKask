---
title: "LoopAction Lifecycle — State Diagram"
diataxis: reference
---

# LoopAction Lifecycle — State Diagram

**Diataxis quadrant:** Reference  
**Domain ontology tier:** Core  
**Purpose:** State diagram for the `LoopAction` enum lifecycle — the finite state machine that governs corrective actions taken by the CNS homeostatic loop.  
**Verified against:** `crates/hkask-cns/src/types/loops/actions.rs`, `crates/hkask-cns/src/types/loops/core.rs`  
last-verified-against: "3d1a876f45e3ce64864c3453f1e71d75b2f14376"

```mermaid
stateDiagram-v2
    [*] --> Pending: CyberneticsLoop::sense\n detects deviation
    
    state Pending {
        [*] --> Evaluating
        Evaluating --> Selected: ActionDecision\n selects action type
    }

    Pending --> Active: CyberneticsLoop::act\n initiates action
    
    state Active {
        [*] --> Executing
        Executing --> GovernedTool: OCAP membrane\n check + reserve
        GovernedTool --> Delegating: Pass OCAP
        GovernedTool --> Denied: Fail OCAP
        Delegating --> Settling: Tool returns result
        Settling --> Completed: Energy settled
    }

    Active --> Completed: Action succeeded
    Active --> Failed: Action failed\n (error/timeout/budget)
    Denied --> Failed: OCAP denial

    state Completed {
        [*] --> ImpactReported
        ImpactReported --> QualityAssessed: LoopQuality scored
        QualityAssessed --> SpanEmitted: CNS span published
    }

    Completed --> [*]: Loop iteration complete
    Failed --> [*]: Escalated to Curator\n (if critical)

    note right of Pending
        ActionType variants:
        - AdjustEnergyBudget
        - EscalateToCurator
        - TriggerCircuitBreaker
        - RequestConsolidation
        - EmitAlgedonicAlert
    end note

    note right of Active
        LoopActionParams carries:
        - action_type: ActionType
        - target: String
        - reason: String
        - priority: u8
    end note

    note right of Completed
        ImpactReport carries:
        - action: LoopAction
        - outcome: ActionOutcome
        - energy_consumed: RJoule
        - duration: Duration
    end note
```

**Node-to-code mapping:**

| State | Type/Enum | Source |
|-------|-----------|--------|
| `LoopAction` | struct | `crates/hkask-cns/src/types/loops/actions.rs` |
| `LoopActionParams` | struct | `crates/hkask-cns/src/types/loops/actions.rs` |
| `ActionType` | enum (5 variants) | `crates/hkask-cns/src/types/loops/actions.rs` |
| `ActionDecision` | struct | `crates/hkask-cns/src/types/loops/core.rs` |
| `ImpactReport` | struct | `crates/hkask-cns/src/types/loops/core.rs` |
| `LoopQuality` | enum | `crates/hkask-cns/src/types/loops/core.rs` |
| `CyberneticsLoop` | struct | `crates/hkask-cns/src/cybernetics_loop.rs` |

**Cardinality:** Exactly 5 `ActionType` variants (verified from source). `LoopAction` is created once per CNS loop iteration. Each action produces exactly 1 `ImpactReport`.
