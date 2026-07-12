---
title: "CNS Regulation Pipeline — 5-Phase Cybernetic Cycle"
diataxis: reference
verified_against:
  - crates/hkask-cns/src/cybernetics_loop.rs
  - crates/hkask-types/src/loops/core.rs
  - crates/hkask-types/src/loops/actions.rs
  - crates/hkask-cns/src/types/loops/signals.rs
  - crates/hkask-cns/src/dampener.rs
last_verified_commit: "3d1a876f"
---

# CNS Regulation Pipeline — 5-Phase Cybernetic Cycle

**Diataxis quadrant:** Reference
**Domain ontology tier:** Core
**Purpose:** Visualize the closed-loop regulation pipeline of the CNS Cybernetics
Loop: Sense → Compare → Compute → Act → Verify. Covers all 9 signal-metric
decision paths in `compute()`, the substitution ladder for repeatedly
ineffective actions, the 3-tier ActionDecision gate, and stagnation detection
leading to RegulatoryPlateau escalation.

Cross-linked from: [`hKask-architecture-master.md`](../architecture/hKask-architecture-master.md)

The diagram below shows the full regulation cycle. Each phase maps to a
`Loop` trait method (or `CyberneticsLoop` override). Signal metrics flow
through deviation detection, per-metric compute rules, action substitution,
dispatch, and impact verification with plateau detection.

```mermaid
flowchart TD
    %% ═══════════════════════════════════════════════════
    %% Phase 1: SENSE
    %% ═══════════════════════════════════════════════════
    subgraph "Phase 1 — Sense"
        S1[Process directive inbox]
        S2[Read wallet balance ratios]
        S3[Poll pluggable sensors via registry]
        S4[Feed values to predictive simulator]
        S5([Cycle start: tick() triggered])
        SIGS[(Signal buffer)]
    end

    %% ═══════════════════════════════════════════════════
    %% Phase 2: COMPARE
    %% ═══════════════════════════════════════════════════
    subgraph "Phase 2 — Compare"
        C1[Match each signal against set-point]
        C2{Deviation detected?<br/>AboveSetPoint or BelowSetPoint}
    end

    %% ═══════════════════════════════════════════════════
    %% Phase 3: COMPUTE
    %% ═══════════════════════════════════════════════════
    subgraph "Phase 3 — Compute (per-metric decision)"

        %% Predictive regulation (pre-deviation)
        subgraph "Predictive Gate"
            PR1{Approaching set-point<br/>in ≤ 3 ticks?}
            PR2[Emit Notify to Curation<br/>with trend projection]
        end

        %% Per-metric decision matrix
        subgraph "Per-Metric Decision Paths"
            ER{EnergyRemaining<br/>below set-point?}
            ER_MODE{InferenceThrottleMode?}
            ER_OFF[Log: throttle disabled]
            ER_AUTO[Pre-authorized autonomous<br/>Throttle → Inference loop]
            ER_CUR[Escalate to Curation<br/>with budget options<br/>fallback: gentle throttle]
            ER_ADJ[AdjustEnergyBudget<br/>unless mode is Off]

            VD{VarietyDeficit<br/>above set-point?}
            VD_ACT[Escalate to Curation]

            ERR{ErrorRate<br/>above set-point?}
            ERR_ACT[CircuitBreak → Inference loop]

            CL{ConnectorLatency<br/>above set-point?}
            CL_ACT[Throttle → Cybernetics loop]

            CQD{CommunicationQueueDepth<br/>above set-point?}
            CQD_ACT[Throttle → Cybernetics loop]

            WBR{WalletBalanceRatio<br/>below set-point?}
            WBR_SEV{Balance = 0?}
            WBR_CRIT[Escalate: critical severity]
            WBR_WARN[Escalate: warning severity]

            WKH{WalletKeyHealth<br/>above set-point?}
            WKH_ACT[Escalate to Curation<br/>informational]

            SC{SeamCoverage?}
            SC_DIR{Direction?}
            SC_DOWN{Drop magnitude > 5pp?}
            SC_CRIT[Escalate: critical severity]
            SC_WARN[Escalate: warning severity]
            SC_UP[Notify: positive health signal]

            TR{ToolReliability<br/>below set-point?}
            TR_ACT[Escalate to Curation]
        end

        %% Substitution Ladder
        subgraph "Substitution Ladder"
            SL{ineffective_count ≥<br/>substitution_after?}
            SL_WALK[Walk metric's substitution ladder<br/>custom → default]
            SL_FOUND{Alternate action<br/>with count = 0?}
            SL_EMIT[Emit ActionSubstituted span<br/>return alternate ActionType]
            SL_FALLBACK[All alternatives exhausted<br/>return proposed action]
        end
    end

    %% ═══════════════════════════════════════════════════
    %% Phase 4: ACT
    %% ═══════════════════════════════════════════════════
    subgraph "Phase 4 — Act"
        A1[Route LoopAction to target loop]
        A2[Persist regulation span<br/>to NuEventStore]
        A3[Emit algedonic alerts<br/>to Curation inbox]
    end

    %% ═══════════════════════════════════════════════════
    %% Phase 5: VERIFY
    %% ═══════════════════════════════════════════════════
    subgraph "Phase 5 — Verify"
        V1[Re-sense metric post-action<br/>gas budget / variety deficit]
        V2[Compute delta: after − before]
        V3{Improved?}
        V4[classify_decision: worsening<br/>vs stage/block ratios]
        V5{Decision?}
        V_ACCEPT[Accept: within noise tolerance<br/>counter reset]
        V_STAGE[Stage: moderate worsening<br/>5%–20%, escalate as Warning]
        V_BLOCK[Block: severe worsening ≥ 20%<br/>emit ActionBlocked span<br/>+ Critical alert to Curation]

        subgraph "Stagnation Detection"
            SD1[record_and_check:<br/>metric + action_type pair]
            SD2{Threshold reached?<br/>default 5 cycles}
            SD3[Emit RegulatoryPlateau span<br/>+ Warning alert to Curation]
        end

        V6[Emit ImpactVerified span<br/>update LoopQuality]
        V7[Compute effectiveness_score<br/>from accept/stage/block ratio]
    end

    %% ═══════════════════════════════════════════════════
    %% Edges: SENSE → COMPARE
    %% ═══════════════════════════════════════════════════
    S5 --> S1
    S1 --> S2
    S2 --> S3
    S3 --> S4
    S4 --> SIGS
    SIGS --> C1

    %% ═══════════════════════════════════════════════════
    %% Edges: COMPARE → COMPUTE
    %% ═══════════════════════════════════════════════════
    C1 --> C2
    C2 -->|"Yes"| PR1
    C2 -->|"No"| S5

    %% Predictive gate
    PR1 -->|"Yes"| PR2
    PR1 -->|"No"| ER
    PR2 --> A1

    %% EnergyRemaining decision chain
    ER -->|"Yes"| ER_MODE
    ER -->|"No"| VD
    ER_MODE -->|"Off"| ER_OFF
    ER_MODE -->|"Autonomous"| ER_AUTO
    ER_MODE -->|"CuratorMediated"| ER_CUR
    ER_OFF --> A1
    ER_AUTO --> SL
    ER_CUR --> A1
    ER_AUTO --> ER_ADJ
    ER_CUR --> ER_ADJ
    ER_ADJ --> SL

    %% VarietyDeficit
    VD -->|"Yes"| VD_ACT
    VD -->|"No"| ERR
    VD_ACT --> SL

    %% ErrorRate
    ERR -->|"Yes"| ERR_ACT
    ERR -->|"No"| CL
    ERR_ACT --> SL

    %% ConnectorLatency
    CL -->|"Yes"| CL_ACT
    CL -->|"No"| CQD
    CL_ACT --> SL

    %% CommunicationQueueDepth
    CQD -->|"Yes"| CQD_ACT
    CQD -->|"No"| WBR
    CQD_ACT --> SL

    %% WalletBalanceRatio
    WBR -->|"Yes"| WBR_SEV
    WBR -->|"No"| WKH
    WBR_SEV -->|"Yes"| WBR_CRIT
    WBR_SEV -->|"No"| WBR_WARN
    WBR_CRIT --> SL
    WBR_WARN --> SL

    %% WalletKeyHealth
    WKH -->|"Yes"| WKH_ACT
    WKH -->|"No"| SC
    WKH_ACT --> A1

    %% SeamCoverage (two directions)
    SC -->|"BelowSetPoint"| SC_DIR
    SC -->|"AboveSetPoint"| SC_UP
    SC_DIR --> SC_DOWN
    SC_DOWN -->|"Yes"| SC_CRIT
    SC_DOWN -->|"No"| SC_WARN
    SC_CRIT --> A1
    SC_WARN --> A1
    SC_UP --> A1

    %% ToolReliability
    TR -->|"Yes"| TR_ACT
    TR -->|"No"| A1
    TR_ACT --> SL

    %% Substitution ladder
    SL -->|"Yes"| SL_WALK
    SL -->|"No (return proposed)"| A1
    SL_WALK --> SL_FOUND
    SL_FOUND -->|"Yes"| SL_EMIT
    SL_FOUND -->|"No"| SL_FALLBACK
    SL_EMIT --> A1
    SL_FALLBACK --> A1

    %% ═══════════════════════════════════════════════════
    %% Edges: COMPUTE → ACT
    %% ═══════════════════════════════════════════════════
    A1 --> A2
    A2 --> A3
    A3 --> V1

    %% ═══════════════════════════════════════════════════
    %% Edges: ACT → VERIFY
    %% ═══════════════════════════════════════════════════
    V1 --> V2
    V2 --> V3
    V3 -->|"Yes"| V4
    V3 -->|"No"| V4
    V4 --> V5
    V5 -->|"Accept"| V_ACCEPT
    V5 -->|"Stage"| V_STAGE
    V5 -->|"Block"| V_BLOCK
    V_ACCEPT --> SD1
    V_STAGE --> SD1
    V_BLOCK --> SD1

    %% Stagnation detection
    SD1 --> SD2
    SD2 -->|"Yes"| SD3
    SD2 -->|"No"| V6
    SD3 --> V6

    %% Impact span + LoopQuality
    V6 --> V7
    V7 --> S5

    %% ═══════════════════════════════════════════════════
    %% Data stores
    %% ═══════════════════════════════════════════════════
    STORE[(NuEventStore)]
    A2 -->|"persist"| STORE
    SD3 -->|"persist"| STORE
    V_BLOCK -->|"persist"| STORE
    A3 -->|"live channel"| CUR[(Curation inbox)]
```

**Node-to-code mapping:**

| Diagram Node | Source Location |
|---|---|
| Sense phase | `cybernetics_loop.rs:734-778` (`sense()`) |
| SignalMetric enum (all 22 variants) | `types/loops/signals.rs:12-96` |
| Compare phase | `crates/hkask-types/src/loops/core.rs:53-56` (`compare()`) |
| Predictive gate | `cybernetics_loop.rs:784-821` |
| EnergyRemaining / InferenceThrottleMode | `cybernetics_loop.rs:825-902` |
| VarietyDeficit / Escalate | `cybernetics_loop.rs:905-919` |
| ErrorRate / CircuitBreak | `cybernetics_loop.rs:920-932` |
| ConnectorLatency / Throttle | `cybernetics_loop.rs:933-947` |
| CommunicationQueueDepth / Throttle | `cybernetics_loop.rs:948-963` |
| WalletBalanceRatio / Escalate | `cybernetics_loop.rs:964-985` |
| WalletKeyHealth / Escalate | `cybernetics_loop.rs:986-999` |
| SeamCoverage (both directions) | `cybernetics_loop.rs:1000-1059` |
| ToolReliability / Escalate | `cybernetics_loop.rs:1061-1082` |
| try_substitute (substitution ladder) | `cybernetics_loop.rs:261-327` |
| default_substitution_ladder | `cybernetics_loop.rs:1762-1777` |
| Act phase | `crates/hkask-types/src/loops/core.rs:62` (`act()`) |
| Verify phase / verify_impact | `cybernetics_loop.rs:1348-1550` |
| classify_decision (Accept/Stage/Block) | `cybernetics_loop.rs:1742-1760` |
| ActionDecision enum | `crates/hkask-types/src/loops/core.rs:159-168` |
| ImpactReport struct | `crates/hkask-types/src/loops/core.rs:98-147` |
| StagnationDetector | `dampener.rs:200-289` |
| RegulatoryPlateau alert emission | `cybernetics_loop.rs:1465-1501` |
| ActionType enum (all 9 variants) | `crates/hkask-types/src/loops/actions.rs:195-227` |
| LoopQuality / effectiveness_score | `crates/hkask-types/src/loops/core.rs:278-286` |

**Substitution ladders (per metric):**

| Metric | Ladder (ordered) |
|---|---|
| EnergyRemaining | Throttle → AdjustEnergyBudget → Escalate |
| VarietyDeficit | Escalate → Calibrate → OverrideEnergyBudget |
| ErrorRate | CircuitBreak → Calibrate → Escalate |
| ConnectorLatency | Throttle → Calibrate → Escalate |
| CommunicationQueueDepth | Throttle → Escalate |
| All others | Empty — escalate on plateau |

**ActionDecision thresholds (classify_decision, line 1748):**

| Gate | Condition | Effect |
|---|---|---|
| Accept | worsening < `stage_worsening_ratio` (5%) | OK, counter reset |
| Stage | 5% ≤ worsening < `block_worsening_ratio` (20%) | Warning, counter increments |
| Block | worsening ≥ 20% | Critical alert, action blocked from re-use |

**Cardinality:** 1 CyberneticsLoop per AgentService. 9 signal metrics have
explicit decision paths in `compute()`. 5 metrics have non-empty substitution
ladders. Default stagnation threshold: 5 consecutive ineffective cycles.
