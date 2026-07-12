---
title: "Skill PDCA Model — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Skill PDCA Model

## Two Layers That Other Systems Conflate

Most agent platforms have one concept: a "skill" is a prompt that runs once. hKask distinguishes two layers:

- **Templates** (`.j2` Jinja2 files, 273 total) — One-shot prompt executions. A template runs once, returns output, and exits. This is what Claude, ChatGPT, and most platforms call a "skill." In hKask, it's raw material.
- **Skills** (39 total) — Iterative PDCA (Plan-Do-Check-Act) loops that compose multiple templates into autonomous cycles. A Skill has a FlowDef manifest with a convergence threshold, a gas budget, and a loop action. It runs until it converges on a quality threshold, exhausts its energy budget, or escalates.

Where other systems give you a prompt, hKask gives you a process.

## The FlowDef Manifest

Every skill is defined by a `manifest.yaml` file located at `registry/templates/{skill-id}/manifest.yaml`. The `SkillLoader` (`crates/hkask-templates/src/skill_loader.rs`) discovers skills from two zones: the private zone (`.agents/skills/`, the author's source of truth) and the public zone (`skills/`, the export surface). It parses YAML front matter from each `SKILL.md`, validates zone-vs-visibility consistency, and infers the skill's domain from its registry manifest template types.

The manifest is deserialized into a `BundleManifest` (`crates/hkask-templates/src/bundle/manifest.rs` and `manifest_loader.rs`):

```rust
pub struct BundleManifest {
    pub id: String,
    pub skills: Vec<BundleSkill>,
    pub steps: Vec<BundleManifestStep>,
    pub convergence: ConvergenceConfig,
    pub gas: BundleGasConfig,
    pub rjoule: RjouleConfig,
    pub error_handling: ErrorHandlingConfig,
    pub ocap: OcapConfig,
    // ...
}
```

Each `BundleManifestStep` declares its `ordinal`, `action` (render, tool_invoke, choice, populate, select, abort, escalate), `template_ref`, `mcp` server target, per-step `gas_cap`, `timeout_seconds`, and optional `condition` gating and `dual_model` flag.

## The Convergence Contract

Convergence is how a skill knows it's done. `ConvergenceConfig` (`crates/hkask-templates/src/bundle/config.rs`) defines the exit conditions:

```rust
pub struct ConvergenceConfig {
    pub threshold: f64,              // Absolute quality threshold
    pub improvement_ratio: f64,      // Minimum proportional improvement
    pub improvement_gate: String,    // "threshold_only" | "both" | "either"
    pub max_iterations: u32,         // Hard iteration cap
    pub min_iterations: u32,         // Minimum before exit allowed
    pub convergence_field: String,   // Context field to read (e.g. "composite")
    pub on_not_reached: String,      // "abort" | "escalate" on max iterations
    pub aggregation: String,         // "none" | "min" | "weighted_avg" | "all_converged"
}
```

Defaults: `threshold = 0.1`, `max_iterations = 3`, `improvement_gate = "threshold_only"`. For a skill to converge, the quality score in `convergence_field` must drop below `threshold` — lower is better, measuring deviation from the target condition. When `improvement_ratio > 0`, the improvement kata engages: `(baseline - current) / baseline >= improvement_ratio`. The `improvement_gate` determines whether both conditions must be met or either suffices.

`min_iterations` prevents premature exit — the improvement kata needs at least a few cycles to produce measurable change.

## The PDCA Loop in Practice

The `ManifestExecutor` (`crates/hkask-templates/src/executor.rs`) implements the loop. Its `execute_manifest()` method:

1. **Plan** — sorts steps by ordinal, reads convergence config, initializes context with `_convergence` metadata (threshold, gas_remaining, rjoule_remaining, iterations_completed).
2. **Do** — enters a `'cascade` loop. Each iteration walks the step list sequentially: render templates (Jinja2 via minijinja), invoke MCP tools, evaluate choice branches, populate context from step outputs, run optional select/evaluate steps.
3. **Check** — after each iteration, `check_convergence()` reads the quality field from context (resolving dot-paths like `"_convergence.quality_at_exit"`), checks against threshold and improvement ratio, and determines whether all convergence conditions are met. For compound skills with multiple sources, `compute_compound_quality()` aggregates across sources using the configured aggregation method.
4. **Act** — if converged, exits with success via `finalize_convergence_report()`. If max iterations reached without convergence, acts per `on_not_reached` — either `abort` (exit with available result) or `escalate` (exit with error for Curator review).

The loop respects gas budgets: `gas_cap` is the total compute cycle allocation, and `gas_cost_per_iteration` is deducted each pass. When gas reaches the `alert_threshold`, a CNS alert fires; when exhausted under `hard_limit`, the loop terminates. Dual currency tracking extends to rJoules: `rjoule_cap` bounds inference energy, and `rjoule_alert_threshold` governs warnings. Tokens consumed by inference calls are tracked via the provider's per-token cost.

## Templates vs. Skills

The distinction is structural, not just semantic. A **Template** (Jinja2 file) is invoked once via `execute_knowact()` or `execute_knowact_dual()` (dual-model path). It renders, queries an LLM, returns output. No convergence, no iteration, no gas tracking.

A **Skill** wraps templates in `BundleManifestStep` entries with `action: "render"`. Multiple render steps, possibly interleaved with `tool_invoke` steps, form a cascade. The skill executor tracks `iteration`, `baseline_quality`, and `recursion_depth` (capped at `hkask_capability::SYSTEM_MAX_RECURSION`). Step conditions (`"NOT var_name"`, `"a AND b"`, `"a OR b"`) gate execution. Choice steps branch based on threshold comparisons against context values.

## LoopAction: The CNS Analog

While skills execute PDCA at the template level, CNS loops execute PDCA at the regulatory level. The `Loop` trait (`crates/hkask-cns/src/types/loops/loop_trait.rs`) defines the same sense→compare→compute→act→verify cycle, but for system regulation rather than skill execution:

```rust
pub trait Loop: Send + Sync {
    fn id(&self) -> LoopId;
    async fn sense(&self) -> Vec<Signal>;
    async fn compare(&self, signals: &[Signal]) -> Vec<Deviation>;
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction>;
    async fn act(&self, actions: &[LoopAction]);
    async fn verify_impact(&self, previous_actions: &[LoopAction]) -> Vec<ImpactReport>;
}
```

Nine `ActionType` variants (`actions.rs`) define what a regulatory loop can do: `Throttle`, `Escalate`, `Calibrate`, `CircuitBreak`, `AdjustEnergyBudget`, `OverrideEnergyBudget`, `ReplenishBudget`, `Notify`, `Prune`. The distinction between `AdjustEnergyBudget` (Cybernetics can adjust within set-point bounds) and `OverrideEnergyBudget` (only Curation can exceed set-points) reflects the authority hierarchy in the CNS DAG.

`LoopQuality` measures the loop's own performance — `delay_ms`, `gain` (actions per deviation), `fidelity_score` (deviations matched to actions), and `effectiveness_score` (ratio of Accepted impact reports). These telemetry metrics close the meta-feedback loop: is the regulator itself regulating effectively?

## Bundle Composition

A Bundle composes multiple Skills but is not itself a PDCA loop. The `BundleManifest` validates composition constraints: at least 2 skills, cascade depth ≤ 7 (the matroshka limit), no divergent and convergent skills in the same cascade phase (P1 violation), and conflict/complementarity declarations that reference valid skill IDs. The bundle orchestrates — it delegates deployment decisions to sub-skills — while skills iterate toward convergence.
