---
title: "The Good Regulator Theorem — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# The Good Regulator Theorem

## Conant-Ashby: The Regulator Must Model the System

The Conant-Ashby theorem (1970) states: "Every good regulator of a system must be a model of that system." The regulator can only control what it can represent. If the regulator's internal model diverges from the system's actual behavior — if it doesn't know what "healthy" looks like, or can't detect when "healthy" becomes "unhealthy" — regulation fails.

This design exists because hKask's CNS is not a passive observer. It is an active cybernetic regulator. It must have a model of the system it regulates, and that model must stay synchronized with reality. The four components that comprise the CNS's internal model — `SetPoints`, `SloManager`, `SeamWatcher`, and `ToolStats` — each model a different dimension of system health, and together they satisfy the Conant-Ashby requirement.

## SetPoints: The Regulator's Internal Model

`SetPoints` at `crates/hkask-cns/src/set_points.rs:139` is the regulator's reference model. It defines 25 configurable fields that establish "what healthy looks like" for every observable dimension:

- **Energy health**: `gas_min_remaining` (default 0.2 — alert when less than 20% of budget remains)
- **Variety health**: `variety_max_deficit` (default 100 — alert when observed variety falls short of expected by more than 100)
- **Error health**: `error_rate_max` (default 0.3 — alert when >30% of operations fail)
- **Latency health**: `connector_latency_max_secs` (default 30s)
- **Communication health**: `communication_backpressure_threshold` (default: `QueueDepth::DEFAULT_BACKPRESSURE`)
- **Seam health**: `seam_coverage_min` (default 0.0 — alert on ANY coverage regression)
- **Federation health**: 8 fields covering sync latency (warning 5s, critical 30s), CRDT divergence (2× baseline), link downtime (warning 1h, critical 24h), pause duration (24h), invitation rate (5/hr), and registry divergence (10 entries/sync)
- **Regulation health**: `max_iterations` (100), `stagnation_thresholds` (per-metric, default 5), `stage_worsening_ratio` (0.05), `block_worsening_ratio` (0.20), `substitution_after` (2)
- **Dampener**: `dampen_window_secs` (60s), `metacognitive_window_secs` (300s), `override_cooldown_secs` (120s)
- **Outcome**: `outcome_warning_threshold` (0.50), `outcome_critical_threshold` (0.25)
- **Guard**: `guard_violation_rate_max` (0.20 — per OWASP LLM Top 10)

These set points are loaded from YAML via `HKASK_CNS_CONFIG` environment variable, falling back to validated defaults. The `SetPointsConfig` type (line 232) allows partial configuration — any omitted field uses its default, making the model self-healing against misconfiguration.

The regulator's model is validated on load: `validate()` at line 398 enforces 13 invariants — ratio fields must be in `[0.0, 1.0]`, warning thresholds must exceed critical thresholds, federation latencies must be ordered warning < critical, `stage_worsening_ratio` < `block_worsening_ratio`, and `variety_max_deficit` must be positive.

## SloManager: Service Level Objectives vs Actual Performance

`SloManager` at `crates/hkask-cns/src/slo_manager.rs:82` models the system's service level contracts. It holds `Vec<SloDefinition>` — explicit, measurable service level objectives — and evaluates them against ν-event data via the `SloDataProvider` trait.

Each SLO has a target compliance rate and a time window. `SloDataProvider::query()` retrieves `SloDataPoint { total_operations, successful_operations }` for a given span namespace within the window. The manager computes `SloEvaluation` — compliance rate, error budget remaining, and breach status. Breaches emit `cns.slo.evaluated` spans and feed the algedonic pathway.

This is the regulator modeling the system's contractual obligations. An SLO breach isn't just "things are slow" — it's "the system promised 99% availability on this span and is delivering 94%." The gap between SLO target and actual performance is a Conant-Ashby deviation: the model says "should be X," reality says "is Y," and the regulator must close that gap.

## SeamWatcher: Detecting Model-Reality Drift

`SeamWatcher` at `crates/hkask-cns/src/seam_watcher.rs:94` models the system's API contracts. It loads the public seam inventory — a machine-readable JSON catalog of every public type, function, and trait, each tagged with its REQ test coverage status — and compares snapshots over time.

The inventory is embedded at compile time via `include_str!("../../../docs/status/public-seam-inventory.json")` (line 34), ensuring seam watching works in deployed binaries. The `HKASK_SEAM_INVENTORY_PATH` env var provides a development override.

When `SeamWatcher` detects drift between snapshots — coverage degradation, new items without tests, or removed coverage — it produces `SeamDrift` records with per-crate `delta_pct`. These drift signals are registered as CNS variety dimensions (`seam:{crate_name}`) with `SEAM_EXPECTED_VARIETY` set to 10. When coverage degrades, the variety deficit triggers algedonic alerts.

This is the regulator detecting model-reality divergence. The seam inventory IS the model of "what APIs exist and are tested." When that model drifts — when a developer adds a public function without a REQ test — the regulator knows. Conant-Ashby is satisfied: the regulator's model of the codebase is kept synchronized through continuous observation.

## ToolStats: Statistical Learning

`ToolStats` at `crates/hkask-cns/src/tool_stats.rs:71` is the regulator's statistical model of tool behavior. It implements a three-layer learning architecture:

**Layer 1 (cost distribution)**: Each tool accumulates up to 200 cost observations (`MAX_COST_OBSERVATIONS`) in a `VecDeque<f64>`. At `reserve_estimate()` time (line 109), if ≥10 observations exist (`MIN_OBSERVATIONS_FOR_FIT`), a LogNormal distribution is fitted via method of moments on log-transformed observations. The reserve estimate is the 90th percentile (`p90`), tightening with more data. If fewer observations exist, the raw mean is used. If none exist, the caller falls back to the `EnergyEstimator` point estimate.

**Layer 2 (reliability tracking)**: `ToolState` tracks `successes: u64` and `failures: u64`. `reliability_alerts()` (line 130) computes Beta posterior success probability: `P(success) = (successes + 1) / (successes + failures + 2)` — a Beta(α = successes+1, β = failures+1) conjugate prior with Laplace smoothing. When `P(success) < reliability_threshold` (default 0.80), a `ToolReliabilityAlert` is emitted, pre-escalating before the tool actually fails.

**Layer 3 (auto-calibration)**: When `GovernedTool` reserves gas, it queries `ToolStats::reserve_estimate()` first. If the distribution's p90 is consistently lower than the point estimate from `EnergyEstimator`, reserves tighten automatically — the statistical model overrides the static model. This closes the feedback loop: tool behavior feeds the model, the model improves the reserve, better reserves prevent gas waste.

The LogNormal choice for cost is deliberate — tool costs are positive and right-skewed (most invocations are cheap, a few are expensive). The Beta choice for reliability is the standard Bayesian conjugate prior for Bernoulli trials, enabling probabilistic reasoning about tool health without storing raw success/failure streams.

`ToolStats` is wired into `GovernedTool` at construction time via `with_tool_stats()`. At settle time, `stats.record(tool, actual_cost, success)` updates the model. The `ToolReliabilitySensor` feeds reliability alerts into the `SensorRegistry`, making tool degradation visible to the CNS regulation pipeline. This completes the Conant-Ashby contract: the regulator models tool behavior statistically, detects degradation probabilistically, and intervenes before the user experiences a failure.
