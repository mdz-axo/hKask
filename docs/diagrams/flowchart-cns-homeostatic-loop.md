---
title: "CNS Homeostatic Loop — Mermaid Flowchart"
diataxis: reference
---

# CNS Homeostatic Loop — Mermaid Flowchart

**Diataxis quadrant:** Explanation  
**Domain ontology tier:** Core  
**Purpose:** Visualize the CNS (Cybernetic Nervous System) homeostatic self-regulation loop — the feedback mechanism by which hKask monitors its own health and takes corrective action.  
**Verified against:** `crates/hkask-cns/src/cybernetics_loop.rs`, `crates/hkask-cns/src/runtime.rs`  
last-verified-against: "3d1a876f45e3ce64864c3453f1e71d75b2f14376"

> **v0.32.0 update:** Added `SetPointCalibrator` (self-tuning regulation thresholds via NuEventStore replay) and contract violation path to CurationLoop.

```mermaid
flowchart TD
    S[Sensors collect data\nSetPoints check thresholds\nSloManager evaluates SLOs]
    C[CyberneticsLoop::sense\nCompares actual vs target\nComputes variety deficit]
    D{Deviation detected?}
    A[CyberneticsLoop::act\nSelects LoopAction\nApplies corrective action]
    R[GovernedTool / GovernedInference\nExecutes action through\nOCAP membrane]
    O[Observe outcome\nImpactReport generated\nLoopQuality assessed]
    E[Emit CNS span\ncns.regulation.* NuEvent persisted\nAlgedonic alert if critical]
    STORE[(NuEventStore)]
    CAL[SetPointCalibrator\nQueries regulation events\nAdjusts thresholds within bounds]
    CUR[CurationLoop\nReads algedonic events\nContract violations included]
    CTV[Contract Violations\nemit_contract_violated\n→ NuEventStore]

    S --> C
    C --> D
    D -->|Yes, deviation exceeds threshold| A
    D -->|No, within tolerance| S
    A --> R
    R --> O
    O --> E
    E --> S
    E -->|persist| STORE
    STORE -->|query every 60 min| CAL
    CAL -->|adjust| SP1
    CAL -->|adjust| SP2
    STORE -->|read via cursor| CUR
    CTV -->|persist| STORE

    subgraph "Set Points"
        SP1[guard_violation_rate_max]
        SP2[energy_budget]
        SP3[convergence_threshold]
        SP4[variety_ceiling]
    end

    subgraph "Sensors"
        SN1[SloManager::evaluate]
        SN2[SeamWatcher::detect_drift]
        SN3[StorageGuardLoop::check]
        SN4[ApiMeter::sample]
    end

    SP1 --> C
    SP2 --> C
    SN1 --> S
    SN2 --> S
    SN3 --> S
    SN4 --> S

    subgraph "Action Types"
        AT1[AdjustEnergyBudget]
        AT2[EscalateToCurator]
        AT3[TriggerCircuitBreaker]
        AT4[RequestConsolidation]
        AT5[EmitAlgedonicAlert]
    end

    A --> AT1
    A --> AT2
    A --> AT3
    A --> AT4
    A --> AT5

    style S fill:#1a1a2e,stroke:#e94560,color:#fff
    style C fill:#16213e,stroke:#0f3460,color:#fff
    style A fill:#0f3460,stroke:#e94560,color:#fff
    style E fill:#533483,stroke:#e94560,color:#fff
```

**Node-to-code mapping:**

| Diagram Node | Source Location |
|-------------|----------------|
| CyberneticsLoop::sense | `crates/hkask-cns/src/cybernetics_loop.rs` |
| CyberneticsLoop::act | `crates/hkask-cns/src/cybernetics_loop.rs` |
| GovernedTool membrane | `crates/hkask-cns/src/governed_tool.rs` |
| SloManager::evaluate | `crates/hkask-cns/src/slo_manager.rs` |
| SeamWatcher | `crates/hkask-cns/src/seam_watcher.rs` |
| StorageGuardLoop | `crates/hkask-storage-guard/src/lib.rs` |
| SetPoints | `crates/hkask-cns/src/set_points.rs` |
| SetPointCalibrator | `crates/hkask-cns/src/set_point_calibrator.rs` |
| ObservableSpan trait | `crates/hkask-types/src/observable_span.rs` |
| LoopAction enum | `crates/hkask-cns/src/types/loops/actions.rs` |
| ImpactReport | `crates/hkask-cns/src/types/loops/core.rs` |
| Algedonic escalation | `crates/hkask-cns/src/runtime.rs` |
| CurationLoop | `crates/hkask-agents/src/curator/curation_loop.rs` |
| Contract events | `crates/hkask-cns/src/contract_events.rs` |
| NuEventStore | `crates/hkask-storage/src/nu_event_store.rs` |

**Cardinality:** 1 CyberneticsLoop runs per AgentService instance. N SetPoints are configured (4 shown). M Sensors feed into the loop. 5 LoopAction types exist in the current codebase (verified against `ActionType` enum).
