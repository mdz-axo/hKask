---
title: "Curator Metacognition Loop — Flowchart"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, trust]
last-verified-against: "3d1a876f"
diataxis: reference
---

```mermaid
flowchart TD
    START([MetacognitionCycle.begin]) --> SENSE

    subgraph SENSE["1. Sense"]
        CNS[Read CNS health<br/>health + variety + alerts]
        REG[Read regulation effectiveness<br/>CnsRuntime::regulation_health]
        BOTS[Count bot failures<br/>escalation policy check]
        SNAPSHOT[Build HealthSnapshot<br/>store via watch channel]
    end

    SENSE --> CLASSIFY

    subgraph CLASSIFY["2. Classify"]
        POLICY[EscalationPolicy::check_conditions]
        WARN{Warning?<br/>deficit > threshold/2}
        CRIT{Critical?<br/>deficit > threshold}
        ALERT_CRIT{Critical alerts<br/>>= threshold?}
        BOT_FAIL{Bot failures<br/>>= threshold?}
        CAT[CAT posture evaluation<br/>convergence_bias decision]
    end

    CLASSIFY --> DECIDE

    subgraph DECIDE["3. Decide"]
        TEMPLATE{ManifestExecutor<br/>available?}
        LLM[compute_with_templates<br/>KnowAct manifest execution]
        RUST[compute_with_thresholds<br/>Rust fallback logic]
        ACTIONS[Produce LoopActions<br/>Calibrate | Escalate | NoAction]
    end

    DECIDE --> ACT

    subgraph ACT["4. Act"]
        CALIBRATE[act_on_throttle<br/>energy budget adjustment]
        ESCALATE[act_on_escalate<br/>direct bots + post escalation]
        NOOP[act_on_no_action<br/>log and continue]
        DIRECT[direct_bot<br/>A2A restart / rebalance]
        BUDGET[OverrideEnergyBudget<br/>LLM-computed new budget]
        PERSIST[Persist escalations<br/>batch or individual<br/>with exponential-backoff retry]
    end

    ACT --> OBSERVE

    subgraph OBSERVE["5. Observe"]
        FEEDBACK[Regulation effectiveness tracking]
        UPDATE[Update HealthSnapshot<br/>for next cycle]
        SEMANTIC[Update semantic index<br/>consolidation if enabled]
    end

    OBSERVE --> START

    style START fill:#4a9,stroke:#333,color:#fff
    style SENSE fill:#26a,stroke:#333,color:#fff
    style CLASSIFY fill:#a4a,stroke:#333,color:#fff
    style DECIDE fill:#c80,stroke:#333,color:#fff
    style ACT fill:#a22,stroke:#333,color:#fff
    style OBSERVE fill:#2a6,stroke:#333,color:#fff
```

## Phase Details

### 1. Sense

Reads CNS health snapshots (`CnsHealth`), variety counters per namespace, all alerts, and critical alerts. Computes total variety deficit against `expected_variety_per_domain` config. Delegates escalation condition checking to `EscalationPolicy::check_conditions`. Builds a `HealthSnapshot` struct and publishes it via `tokio::sync::watch::Sender` for downstream consumers. Produces two afferent `Signal`s: `MetacognitionVarietyDeficit` and `MetacognitionCriticalAlerts`.

### 2. Classify

- **EscalationPolicy** — pure-data module implementing the algedonic signal model. Checks three conditions: VarietyDeficit (Warning at threshold/2, Critical at threshold), CriticalAlerts (≥ threshold), BotFailures (≥ threshold). Returns `Vec<EscalationAlert>`.
- **CAT (Communication Accommodation Theory)** — evaluates whether the Curator should engage with Matrix communication events. `convergence_bias ≥ 0.7`: speak to any message. `> 0.0`: speak only when addressed by name. `= 0.0`: always silent.

### 3. Decide

Two code paths gated by `ManifestExecutor` availability:
- **Template path** (`compute_with_templates`): KnowAct manifest execution via LLM. Produces calibrated regulatory actions with confidence scores from `manifest_executor.execute_knowact`.
- **Rust fallback** (`compute_with_thresholds`): Threshold comparison producing `LoopAction` with `Calibrate`/`Escalate`/`NoAction` types. Used in standalone CLI mode when no executor is configured.

### 4. Act

Dispatches `LoopAction`s:
- `Calibrate` → `act_on_throttle`: generates throttle escalation entries
- `Escalate` → `act_on_escalate`: may issue `CuratorDirective` (OverrideEnergyBudget) or direct bots via `direct_bot` (A2A restart/rebalance)
- Template-driven bot direction: LLM-computed restart/rebalance directives sent before escalation posting
- Escalation persistence with exponential-backoff retry (max 3 retries, base 100ms delay). Batched when concurrent count ≥ `max_concurrent_escalations` threshold.

### 5. Observe

Regulation effectiveness (accepted/blocked/staged ratio from `CnsRuntime`) feeds into next cycle's snapshot. Semantic index updated via consolidation bridge if `auto_consolidation_enabled`. HealthSnapshot watch channel notifies downstream observers. Cycle repeats at configured `interval` (default: 1 hour).

## Node-to-Code Mapping

| Node | Crate | Source File |
|------|-------|-------------|
| `MetacognitionLoop` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `HkaskLoop::sense` | `hkask-agents` | `src/curator_agent/metacognition/hloop_impl.rs` |
| `HkaskLoop::compute` | `hkask-agents` | `src/curator_agent/metacognition/hloop_impl.rs` |
| `HkaskLoop::act` | `hkask-agents` | `src/curator_agent/metacognition/hloop_impl.rs` |
| `compute_with_templates` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `compute_with_thresholds` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `act_on_throttle` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `act_on_escalate` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `act_on_no_action` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `direct_bot` | `hkask-agents` | `src/curator_agent/metacognition/loop_body.rs` |
| `EscalationPolicy` | `hkask-agents` | `src/curator_agent/metacognition/escalation.rs` |
| `HealthSnapshot` | `hkask-agents` | `src/curator_agent/metacognition/config.rs` |
| `MetacognitionConfig` | `hkask-agents` | `src/curator_agent/metacognition/config.rs` |
| `CAT evaluate` | `hkask-agents` | `src/curator_agent/cat.rs` |
| `CuratorAgent` (composition) | `hkask-agents` | `src/curator_agent/mod.rs` |
| `persist_escalation_with_retry` | `hkask-agents` | `src/curator_agent/metacognition/persistence.rs` |
| `format_health_status` | `hkask-agents` | `src/curator_agent/metacognition/format.rs` |
