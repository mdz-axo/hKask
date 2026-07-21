---
title: "ADR-056: Ashby-Aligned Loop Regulation Focus"
audience: [architects, developers, agents]
last_updated: 2026-07-21
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, curation]
---

# ADR-056: Ashby-Aligned Loop Regulation Focus

**Date:** 2026-07-21  
**Status:** Active

## Context

The shift from a three-tier pod model (Curator/Team/Replicant) to a two-kind model (Curator/UserPod) in v0.31.0 collapsed the taxonomic distinction between agent kinds. The loops and CNS now regulate *behavioral roles* (VSM S1 Implementation vs. S4 Intelligence) rather than *agent identity tiers*. This shift changes the disturbance space that the cybernetic regulator must absorb.

**Problem Statement:** The `RegulationPolicy` covers only 8 of 31 `SignalMetric` variants, creating a variety deficit (Ashby's Law violation) where the regulator cannot respond to 23 distinct disturbance classes the system can produce.

**Stakeholders:** Curator (regulator), UserPods (regulated actors), Cybernetics Loop (homeostatic controller), Curation Loop (meta-observer).

**Constraints:**
- P9 (Homeostatic Self-Regulation) requires the regulator to have sufficient variety to absorb system disturbances.
- P5 (Essentialism) forbids adding regulation rules without a concrete disturbance they address.
- The `Loop` trait's `verify_impact()` closure must produce actionable `ImpactReport` data, not just telemetry.

## Decision

**Chosen Approach:** Focus loop regulation on closing the variety deficit through three concrete actions, each grounded in a specific unregulated disturbance class:

### Action 1: Extend `RegulationPolicy` to cover the 23 unregulated metrics

The `RegulationPolicy::default()` currently has 12 rules covering 8 metrics. The 23 unregulated metrics fall into three categories:

**Category A — Metrics that should produce `Notify` (informational, no regulation needed):**
- `StorageUsage`, `TripleCount`, `LowConfidenceCount`, `SnapshotInterval`, `ConsolidationCandidates`, `PendingEscalations`

These are observational metrics. They don't represent deviations requiring regulatory action — they're state snapshots. Adding `Notify` rules for them closes the variety gap without introducing false-positive regulation. The regulator *acknowledges* the signal without *acting* on it.

**Category B — Metrics that should produce `Escalate` (require Curation intervention):**
- `GoalStaleCount`, `GoalExpiredCount`, `MetacognitionVarietyDeficit`, `MetacognitionCriticalAlerts`, `ActionIneffective`, `RegulatoryPlateau`, `ActionDecisionBlocked`

These are meta-regulatory signals — they indicate the regulator itself is failing. Per Conant-Ashby (Good Regulator), when the regulator's model diverges from reality, the only valid response is escalation to a meta-regulator (Curation). These metrics already have `SignalMetric` variants but no `RegulationRule` entries.

**Category C — Metrics that should produce `Calibrate` or `Throttle` (domain-specific regulation):**
- `MemoryLife` (Episodic Loop 2a) → `Calibrate` (adjust memory retention set-point)
- `CircuitBreakerState`, `InferenceAvailable`, `InferenceGasRemaining`, `InferenceModelAvailable` (Inference Loop 1) → `Calibrate` (signal model selection needed) or `Throttle` (gas depletion)
- `AlgedonicEvents` (Cybernetics Loop 6) → `Escalate` (algedonic cascade detection)
- `McpServerHealth` (McpServerGuard Loop 8) → `CircuitBreak` (unhealthy server isolation)
- `DiskUsagePct` (StorageGuard Loop 7) → `Prune` (autonomous disk space management)

### Action 2: Extend `default_substitution_ladder` to cover all regulated metrics

The `default_substitution_ladder()` function currently covers 5 metrics. The remaining regulated metrics (from Action 1 Category B and C) need substitution ladders so that when a primary action is ineffective, the regulator has a fallback chain rather than cycling in place.

Proposed ladders:
- `GoalStaleCount` / `GoalExpiredCount` → `[Escalate, Calibrate]`
- `MetacognitionVarietyDeficit` / `MetacognitionCriticalAlerts` → `[Escalate, Calibrate, OverrideEnergyBudget]`
- `ActionIneffective` / `RegulatoryPlateau` / `ActionDecisionBlocked` → `[Escalate, Calibrate]` (meta-regulatory — only Curation can break the plateau)
- `MemoryLife` → `[Calibrate, Escalate]`
- `CircuitBreakerState` → `[CircuitBreak, Calibrate, Escalate]`
- `InferenceAvailable` / `InferenceModelAvailable` → `[Calibrate, Escalate]`
- `InferenceGasRemaining` → `[Throttle, AdjustEnergyBudget, Escalate]`
- `AlgedonicEvents` → `[Escalate, Calibrate]`
- `McpServerHealth` → `[CircuitBreak, Calibrate, Escalate]`
- `DiskUsagePct` → `[Prune, Escalate]`

### Action 3: Ensure `verify_impact()` closure for all new rules

The `Loop` trait's `verify_impact()` method is the Conant-Ashby closure — the regulator observes its own action's effect. Currently, only `CyberneticsLoop` implements `verify_impact()` non-trivially. The new regulation rules must be paired with impact verification so that `ActionDecision::Accept/Stage/Block` is computed for every regulated metric.

This means:
- `StorageGuard` loop must implement `verify_impact()` for `DiskUsagePct` → `Prune` actions (re-sense disk usage after pruning).
- `McpServerGuard` loop must implement `verify_impact()` for `McpServerHealth` → `CircuitBreak` actions (re-sense server health after isolation).
- `InferenceLoop` must implement `verify_impact()` for `InferenceGasRemaining` → `Throttle` actions (re-sense gas remaining after throttling).

**Alternatives Considered:**
1. **Do nothing — let unregulated metrics accumulate without response.** Rejected: violates Ashby's Law (regulator variety < system variety) and P9 (Homeostatic Self-Regulation). Unregulated disturbances will cascade unmitigated.
2. **Add a generic "catch-all" rule that escalates any unregulated metric.** Rejected: violates P5 (Essentialism — no pass-through abstractions). A catch-all rule is a shallow module that doesn't encode the specific regulatory response each metric requires.
3. **Remove the unregulated metrics from `SignalMetric` instead of regulating them.** Rejected: the metrics are real disturbances the system produces. Removing them from the enum would hide the disturbance, not eliminate it. This is the "suppress the alert" anti-pattern.

**Rationale:** Ashby's Law of Requisite Variety states that the regulator's variety must be at least as large as the system's disturbance variety. The current 8-of-31 coverage is a 74% variety deficit. The three actions close this deficit by:
- Action 1: Expanding regulator variety (more rules = more response classes)
- Action 2: Expanding regulator variety further (substitution ladders = ordered response chains)
- Action 3: Closing the feedback loop (verify_impact = the regulator observes its own effectiveness)

The distinction between Category A (Notify), B (Escalate), and C (Calibrate/Throttle/CircuitBreak/Prune) is grounded in the cybernetic role of each metric, not in ad-hoc severity assignment. This is the difference between *taxonomic regulation* (the old tier model) and *cybernetic regulation* (the post-shift model).

## Consequences

### Positive

- **Ashby's Law satisfied.** Regulator variety (rules × substitution ladders × impact verification) meets or exceeds system disturbance variety (31 SignalMetric variants).
- **Conant-Ashby closure.** Every regulated metric has a `verify_impact()` path, so the regulator observes its own effectiveness and can self-tune via `SetPointCalibrator`.
- **Meta-regulatory escalation.** Metrics like `RegulatoryPlateau` and `ActionDecisionBlocked` now have explicit escalation paths, preventing the regulator from cycling in place when its model has converged to a wrong attractor.
- **Per-loop impact verification.** `StorageGuard` and `McpServerGuard` loops gain `verify_impact()` implementations, closing their local feedback loops rather than relying on Cybernetics to detect their failures.

### Negative

- **Larger `RegulationPolicy` surface.** 12 rules → ~35 rules. More rules means more configuration surface and more test cases.
- **More `verify_impact()` implementations.** Each loop that gains impact verification must re-sense its target metric after acting, adding latency to the regulation cycle.
- **Substitution ladder complexity.** More ladders means more stagnation keys to track and more `ActionDecision::Block` states to manage.

### Neutral

- The `RegulationPolicy` remains a pure-data type (no behavior change). The `compute()` method in `CyberneticsLoop` already iterates `policy.decide(dev)` — adding rules doesn't change the control flow.
- The `SetPointCalibrator` already self-tunes thresholds; the new rules just give it more signals to tune.

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P5** (Essentialism) | ✅ | Each new rule addresses a specific disturbance class — no catch-all, no pass-through. Category A (Notify) is the minimal response for observational metrics. |
| **P7** (Prefer deletion over deprecation) | ✅ | No existing rules are deprecated. The new rules extend coverage without removing existing regulation. |
| **P9** (Homeostatic Self-Regulation) | ✅ | Closes the Ashby variety deficit. Every `SignalMetric` has a regulatory response. |
| **P8** (Semantic Grounding) | ✅ | Each rule is grounded in a specific `SignalMetric` variant with a documented cybernetic role. |

### Constraints

| Constraint | Compliance | Evidence |
|-----------|-----------|----------|
| **C4** (Repetition is missing primitive) | ✅ | The substitution ladders are the missing primitive for ordered response chains. |
| **C5** (Every error variant is unique recovery path) | ✅ | Each `SignalMetric` variant maps to a distinct regulatory response, not a shared catch-all. |
| **C7** (Divergence must yield) | ✅ | `RegulatoryPlateau` and `ActionDecisionBlocked` metrics now have explicit escalation paths — divergence yields to Curation. |

## Verification

```bash
# Verify RegulationPolicy covers all SignalMetric variants
cargo test -p hkask-cns regulation_policy

# Verify substitution ladders are non-empty for all regulated metrics
cargo test -p hkask-cns default_substitution_ladders

# Verify verify_impact is implemented for StorageGuard and McpServerGuard
cargo test -p hkask-storage-guard verify_impact
cargo test -p hkask-cns verify_impact

# Verify no SignalMetric is unregulated
grep -c "SignalMetric::" crates/hkask-types/src/loops/signals.rs
grep -c "RegulationRule" crates/hkask-cns/src/regulation_policy.rs
```

**Expected Results:**
- All `regulation_policy` tests pass, including new tests for previously-unregulated metrics.
- All `default_substitution_ladders` tests pass, including new ladders.
- `verify_impact` tests pass for `StorageGuard` and `McpServerGuard`.
- The count of `RegulationRule` entries in `regulation_policy.rs` is ≥ 31 (one per `SignalMetric` variant, minus the `NoData`-equivalent metrics that don't need regulation).

## Implementation Phases

### Phase 1: Category A (Notify) rules — low risk, no behavior change
Add `Notify` rules for the 6 observational metrics. These produce informational signals without regulatory action. No `verify_impact()` needed — `Notify` is non-regulatory.

### Phase 2: Category B (Escalate) rules — medium risk, adds meta-regulatory paths
Add `Escalate` rules for the 7 meta-regulatory metrics. These route to Curation, which already handles escalation. Add substitution ladders for these metrics.

### Phase 3: Category C (Calibrate/Throttle/CircuitBreak/Prune) rules — higher risk, adds domain-specific regulation
Add domain-specific rules for the 10 domain metrics. Implement `verify_impact()` for `StorageGuard`, `McpServerGuard`, and `InferenceLoop`. Add substitution ladders.

Each phase is independently verifiable and can be merged separately.

## References

[^ashby]: Ashby, W. R. (1956). *An Introduction to Cybernetics.* Chapman & Hall. — Ashby's Law of Requisite Variety: the regulator's variety must be ≥ the system's disturbance variety.
[^conant-ashby]: Conant, R. C., & Ashby, W. R. (1970). "Every Good Regulator of a System Must Be a Model of That System." *International Journal of Systems Science*, 1(2), 89-97. — the regulator must model the system it regulates.
[^beer-vsm]: Beer, S. (1979). *The Heart of Enterprise.* Wiley. — Viable System Model (S1–S5 subsystems).
[^ousterhout]: Ousterhout, J. (2018). *A Philosophy of Software Design.* Yaknyam Press. — deep modules, interface minimalism.

---

*ℏKask - A Minimal Viable Container for UserPods — ADR-056 — v0.31.0*
