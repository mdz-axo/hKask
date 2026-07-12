---
title: "CNS Homeostatic Loop — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# CNS Homeostatic Loop

The CNS (Cybernetic Nervous System) is the component of hKask that other documentation calls "the monitoring system." That label is wrong, and the misconception matters. The CNS is not a monitoring system — it is a **regulatory system**. A monitoring system observes and alerts. A regulatory system senses, compares, computes, acts, and verifies. The CNS does all five.

## What the CNS Is

The CNS is a closed-loop controller implemented as a cybernetic feedback loop. Its core type is `CyberneticsLoop` (`crates/hkask-cns/src/cybernetics_loop.rs`), which implements the `HkaskLoop` trait (`crates/hkask-cns/src/types/loops/loop_trait.rs`). The `HkaskLoop` trait defines a five-phase cycle:

```
sense → compare → compute → act → verify_impact
```

The `tick()` method chains all five phases:

```rust
async fn tick(&self) {
    let signals = self.sense().await;
    let deviations = self.compare(&signals).await;
    let actions = self.compute(&deviations).await;
    self.act(&actions).await;
    let _ = self.verify_impact(&actions).await;
}
```

Each phase is a separate async method, making the regulatory pipeline explicit rather than buried in reactive callbacks. The cycle is triggered by `Scheduled` ticks, `AlertDriven` events, `Manual` directives, or `EventDriven` ν-events — provenance is tracked via `TriggerOrigin` so the CNS can correlate trigger type with regulatory effectiveness.

## The Sense-Act-Observe Cycle

The regulation cycle begins with **sense**. `CyberneticsLoop::sense()` (`cybernetics_loop.rs:733`) collects signals from multiple sources: per-agent energy ratios from the `GasBudgetManager`, wallet balance ratios, and signals from pluggable `SensorProvider` instances (`EnergyBudgetSensor`, `VarietySensor`, `ToolReliabilitySensor`, `WalletKeyHealthSensor`). The sensor registry pattern means new signal sources can be added without modifying the loop.

Signals are compared against **set-points** during `compare()` — the default implementation in the `HkaskLoop` trait filters signals through `Deviation::from_signal()`, producing a `DeviationDirection` (`AboveSetPoint` or `BelowSetPoint`) for each metric that exceeds its threshold.

**Compute** (`cybernetics_loop.rs:779`) is where the regulatory logic lives. For each deviation, the loop selects an `ActionType`:

| Deviation | Direction | Action |
|-----------|-----------|--------|
| `EnergyRemaining` | `BelowSetPoint` | `Throttle` or `AdjustEnergyBudget` (depending on `InferenceThrottleMode`) |
| `VarietyDeficit` | `AboveSetPoint` | `Escalate` to Curation |
| `ErrorRate` | `AboveSetPoint` | `CircuitBreak` on Inference |
| `ConnectorLatency` | `AboveSetPoint` | `Throttle` |
| `CommunicationQueueDepth` | `AboveSetPoint` | `Throttle` (backpressure) |
| `WalletBalanceRatio` | `BelowSetPoint` | `Escalate` to Curation (critical if zero) |
| `WalletKeyHealth` | `AboveSetPoint` | `Escalate` (informational) |
| `SeamCoverage` | `BelowSetPoint` | `Escalate` (critical if >5pp drop) |
| `SeamCoverage` | `AboveSetPoint` | `Notify` (positive health signal) |
| `ToolReliability` | `BelowSetPoint` | `Escalate` to Curation |

Each action is wrapped in a `LoopAction` struct that carries a `target` (which loop receives the action), an `action_type`, typed `LoopActionParams` with a required `reason` field, and an optional `metric_name` for impact verification.

The **act** phase dispatches these actions to their target loops, and **verify_impact** (`cybernetics_loop.rs:1354`) re-senses the targeted metric and compares pre- and post-action values, producing an `ImpactReport` with an `ActionDecision` — `Accept`, `Stage`, or `Block`. Actions that are repeatedly `Block`ed are prevented from re-use for that metric.

## Set Points as Homeostatic Targets

`SetPoints` (`crates/hkask-cns/src/set_points.rs:139`) is the homeostatic reference model. It defines 25 configurable thresholds across four categories:

**Resource set-points:** `gas_min_remaining` (default 0.2), `variety_max_deficit` (100), `error_rate_max` (0.3), `connector_latency_max_secs` (30.0), `communication_backpressure_threshold`.

**Outcome set-points:** `outcome_warning_threshold` (0.50), `outcome_critical_threshold` (0.25) — when the success rate of regulatory actions drops below these thresholds, the CNS escalates.

**Regulation set-points:** `max_iterations` (100) prevents unbounded cascading, `stagnation_thresholds` detect regulatory plateaus (Fermi-inspired early-stopping pattern), `stage_worsening_ratio` (0.05) and `block_worsening_ratio` (0.20) control the three-tier decision gate.

**Consent and guard set-points:** `inference_throttle_mode` controls how throttling decisions are made — `Off` (user manages manually), `Autonomous` (CNS throttles directly, pre-authorized by P2 consent), or `CuratorMediated` (escalate to Curator with fallback timeout). `guard_violation_rate_max` (0.20) triggers when >20% of requests are blocked by content safety.

Set points are loaded from YAML via `SetPointsConfig::load_from_file()` and merged with defaults in `SetPoints::from_config()`. The `validate()` method (`set_points.rs:398`) enforces invariants: `gas_min_remaining` must be in [0, 1], outcome thresholds must not be inverted (warning > critical), `variety_max_deficit` must be positive.

When actual exceeds target — such as `EnergyRemaining` dropping below `gas_min_remaining` — the CNS does not just log a warning. It produces an efferent action: `Throttle` (reduce inference throughput), `AdjustEnergyBudget` (reallocate within set-point bounds), or `Escalate` (delegate to Curator). The action is not advisory; it changes the system's behavior.

## Variety Engineering — Why the CNS Exists

The CNS exists because of Ashby's Law of Requisite Variety: a controller must have at least as much variety as the system it regulates. An unsupervised agent system generates unbounded variety — different models, different prompts, different tools, different error modes, different resource states. Without a controller with matching variety, the system drifts toward entropy.

The CNS maintains a `VarietyMonitor` (`crates/hkask-cns/src/runtime.rs:219`) that counts distinct agent states across domains using an exponential moving average (EMA) with a 60-second window. Each domain (Cybernetics, Curation, Inference, Episodic) has its own `VarietyTracker` with a configurable window. The `variety_deficit` is computed as the gap between expected variety and observed variety — when it exceeds `variety_max_deficit` (default 100), the CNS escalates.

The runtime's `check_variety()` method (`runtime.rs:703`) compares the current EMA against the `DEFAULT_EXPECTED_VARIETY` and produces an `AboveSetPoint` deviation when the deficit is too large. The Cybernetics loop then fires an `Escalate` action to the Curation loop, which may rebalance agent pools or adjust concurrency limits.

## The Good Regulator Theorem

Conant and Ashby's Good Regulator theorem states that every good regulator of a system must contain a model of that system. The CNS contains explicit models of every subsystem it regulates — this is not an aspirational goal, it is a structural requirement for cybernetic regulation.

The model is embodied in three structures:

- **`SetPoints`** models the desired state of every regulated metric — not just the threshold, but the directional semantics (energy should go up, error rate should go down, variety deficit should stay bounded).

- **`SloManager`** models the service-level objectives: SLOs are registered per-domain and evaluated on each tick. When an SLO breaches, the algedonic pathway activates.

- **`SystemSimulator`** (`cybernetics_loop.rs:101`) models the system's dynamics: `MovingAverageExtrapolator` predicts metric trajectories, enabling **predictive regulation** — if a metric is approaching its set-point within 3 ticks, the CNS emits a `Notify` action *before* the threshold is breached. This is anticipatory regulation, not reactive monitoring.

The `ToolStats` component (`tool_stats`) builds per-tool `LogNormal` cost distributions and `Beta` reliability distributions, enabling the CNS to predict whether a tool call is likely to succeed and how much gas it will consume — a statistical model of the system's behavior under load.

## Action Types and When Each Fires

The `ActionType` enum (`crates/hkask-types/src/loops/actions.rs:195`, re-exported by `hkask-cns`) defines nine action categories:

| ActionType | When it fires | Target |
|-----------|---------------|--------|
| `Throttle` | Energy low, connector latency high, queue depth exceeds backpressure | `Inference` or `Cybernetics` |
| `Escalate` | Variety deficit exceeded, wallet balance low, key unhealthy, seam degraded, tool reliability low | `Curation` |
| `Calibrate` | Thresholds need adjustment based on observed error rates | `Cybernetics` (self-calibration) |
| `CircuitBreak` | Error rate exceeds `error_rate_max` | `Inference` |
| `AdjustEnergyBudget` | Energy low (within set-point bounds, automatic) | `Cybernetics` |
| `OverrideEnergyBudget` | Curation overrides set-point bounds (weaker capability) | `Cybernetics` |
| `ReplenishBudget` | Curator injects gas into an exhausted agent | `Cybernetics` |
| `Notify` | Positive health signals (seam coverage improved, predictive approach warning) | `Curation` |
| `Prune` | Disk space management (StorageGuard Loop 7) | Storage |

The capability hierarchy is deliberate: `AdjustEnergyBudget` and `OverrideEnergyBudget` are distinct because Cybernetics can adjust within its set-point range, but only Curation can override set-points themselves. `ReplenishBudget` is exclusively a Curator capability — the CNS cannot create energy, only redistribute it.

## Algedonic Escalation

"Algedonic" (from Greek *algos*, pain, and *hedone*, pleasure) describes the mechanism by which the CNS communicates threat levels to the Curator. The `AlgedonicManager` (`crates/hkask-cns/src/algedonic.rs`) classifies alerts by severity:

- **Info:** Positive signals (`Notify` actions, seam coverage improvements)
- **Warning:** Deviations that are correctable autonomously (energy below 20% but above 0%, variety deficit elevated but below critical, tool reliability degraded)
- **Critical:** Deviations requiring Curator intervention (energy at 0%, error rate catastrophic, key completely exhausted, seam coverage drops >5pp)
- **Fatal:** System-level failures requiring human intervention (the CNS itself is unstable)

The escalation pathway is not just a log line. `Critical` alerts flow through `alerts_tx` (`cybernetics_loop.rs:79`) as `CurationInput` messages to the `CurationLoop`. The Curator's metacognition layer (`crates/hkask-agents/src/curator_agent/metacognition.rs`) receives these as structured problem statements — not raw metrics, but curated alerts with context, options, and fallback behaviors.

When the CNS cannot self-correct — when `verify_impact` returns `Block` decisions for repeated actions, or when the `StagnationDetector` (`cybernetics_loop.rs:93`) detects a regulatory plateau — the escalation moves upward: from Cybernetic self-regulation to Curator metacognitive override, and from Curator to human operator if the Curator cannot resolve the issue within its authority bounds.

This is the two-level meta-loop stability guarantee: the CNS stabilizes the domain loops (Inference, Episodic, Semantic); the Curation loop stabilizes the CNS. The CNS may NOT regulate the Curation loop — the authority DAG flows downward, never upward.

For a visual reference, see the [CNS Homeostatic Loop Flowchart](../diagrams/flowchart-cns-homeostatic-loop.md), which diagrams the complete sense→compare→compute→act→verify cycle with set-point gates, action dispatch, and algedonic escalation pathways.
