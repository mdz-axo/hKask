# Curator Review — Plan

**Scope:** Evidence-grounded cleanup of the curator (regulatory + persona halves).
**Status:** Complete. All slices S1-S8 + L3/L4 executed and validated (clippy clean, tests green). Open Q examined (see bottom). Pre-existing self-heal issue confirmed resolved (see bottom).

## Architecture (verified)

The curator is NOT a god-object — it was already decomposed in "Curation/Agent
Separation (Task 6)". Two `RegulationLoop` impls share `LoopId::Curation`:
`CurationLoop` (pure regulatory) and `MetacognitionLoop` (persona), ticked by
`LoopScheduler` at 10s. Matrix is a thin seam (events via channel, not a god-object).

## Findings (consolidated, evidence-backed)

### CRITICAL
- **C1** [FIXED S3] — Metacognition `act()` was effectively dead. `param_str`/`param_u64`
  were stubs that always returned defaults. `act_on_throttle`/`act_on_escalate` always
  returned None. No escalations or directives issued. No tests covered it.

### HIGH
- **H1** [FIXED S4] — `act()` batch branch built escalate template context with empty
  defaults instead of the actual `HealthSnapshot`.
- **H2** [FIXED S5] — `compute_with_templates` `adjust_budget` discarded `new_budget`/`target`;
  the `OverrideEnergyBudget` action was never handled in `act()` (silently dropped).

### MEDIUM
- **M1** [FIXED S5] — Hardcoded `new_budget: 5000` externalized to a named const.
- **M2** [FIXED S6] — Stringly-typed action dispatch replaced with `CurationEscalationReason` enum.
- **M3** [FIXED S2] — Stringly-typed state dispatch replaced with `GoalLifecycle` enum.
- **M4** [FIXED S7] — Unified the two `EscalationSeverity` types.

### LOW
- **L1** [FIXED S1] — Duplicate `GoalExpiredCount` match arm removed.
- **L2** [FIXED S1] — Dead `_metric`/`_target` locals removed.
- **L3** [FIXED] — Deleted unused `CuratorHandle::new_test()` (zero callers).
- **L4** [FIXED] — Eliminated always-pass OCAP gate; authority enforced by singleton construction.
- **L5** [FIXED S8a] — Removed dead `MetacognitionConfig.interval`.
- **L6** [FIXED S8b] — Deduped double `try_auto_consolidate()` call.
- **L7** — `reg_health` shadowed in sense() (cosmetic; left as-is).

## Vertical slices executed

| Slice | Files | Fix |
|-------|-------|-----|
| S1 | curation_loop.rs, hloop_impl.rs | dedup match arm + dead locals |
| S2 | channels.rs, loops/mod.rs, curation_loop.rs | `GoalLifecycle` enum |
| S3 | loop_body.rs | CRITICAL: rewire act helpers from snapshot; delete stubs |
| S4 | hloop_impl.rs | feed HealthSnapshot into escalate template context |
| S5 | actions.rs, loop_body.rs, hloop_impl.rs, curation_loop.rs | `CuratorBudgetOverride` variant + handle it + externalize 5000 |
| S6 | curation_loop.rs | `CurationEscalationReason` typed dispatch |
| S7 | escalation.rs, hloop_impl.rs, two mod.rs | unify EscalationSeverity |
| S8 | config.rs, curation_loop.rs | remove dead interval + dedup consolidation |
| L3 | curator.rs | delete new_test() |
| L4 | context.rs | eliminate always-pass OCAP gate + dead import |

## Validation
`cargo clippy -D warnings` clean on hkask-types, hkask-regulation, hkask-pods,
hkask-services-context, hkask-services-chat, hkask-mcp-curator.
`cargo test`: hkask-types 80/80, hkask-regulation 167/167, hkask-pods 31/31.

---

## Open Q resolution (curator self-regulation / reg.* spans)

**Current state (evidence):** The curator deliberately keeps most of its
observability as **tracing log targets** (`reg.curation.escalation`,
`reg.curation.matrix`, `curation.loop`, `curator.metacognition`) rather than
persistent `reg.*` spans. The ONE persistent canonical span it emits is
`reg.curator.consolidation` (`InfraSpan::CuratorConsolidation`). This avoids
circularity: `CurationLoop::sense()` reads ALL algedonic events via
`store.query_algedonic(since, ...)` with no curator-namespace filter, so any
persistent `reg.curator.*` span it emitted would be read back by itself.

**Why the user's instinct is right:** the curator manages the regulator
(Curation -> Cybernetics), so persistent self-observation spans would create a
self-referential loop (the regulator regulating its own regulatory signals).

**What already exists for self-management (half-built):**
- `MetacognitionLoop::sense()` reads `regulation_effectiveness` (ratio of
  accepted regulatory actions) — this IS a self-quality signal.
- `EscalationPolicy` thresholds + `MetacognitionConfig` are configurable.
- `reg.curator.consolidation` proves the canonical-namespace path works for
  curator action outputs.

**IMPLEMENTED (this turn) — the reg.meta self-management namespace:**
A `reg.meta.*` canonical namespace + `MetaSpan` type now lets the Curator
observe its own decision quality and adjust itself, non-circularly:
- `reg.meta.directive` — emitted at `issue_directive` (the single chokepoint).
- `reg.meta.escalation` — emitted on each escalation persist outcome.
- `reg.meta.circuit_breaker` — emitted when the template circuit breaker trips.
- `reg.meta.self_calibration` — emitted when the Curator adjusts its own threshold.
Non-circularity is enforced structurally: these spans are NOT in
`ALGEDONIC_SPAN_CATEGORIES`, so `CurationLoop::sense()` (which uses
`query_algedonic`) never reads them back. Self-quality is tracked in-process
(`SelfQuality` counters) and fed to `MetacognitionLoop::self_calibrate()`,
which adjusts `EscalationPolicy` thresholds (interior-mutable `Arc<RwLock>`)
and emits `reg.meta.self_calibration`. Authority DAG: Meta -> Curation
-> Cybernetics -> domains.

**Final policy (M4 + M6):** generative-first. `self_calibrate` is async and
asks the `curator/metacognition-self-calibrate.j2` template to generate its
own threshold adjustment from self-quality + effectiveness + the last
calibration's effectiveness delta (the Curator as its own generative entity).
The Rust `compute_threshold_adjustment` is the safety-rail fallback (no
executor / template failure). Both paths are clamped to a bounded band
[floor, 4x ceiling] with hysteresis cooldown + min-observations gate; the
`reg.meta.self_calibration` span records the decision source (generative /
fallback) + before/after effectiveness (eff_delta) — the causal signal. Pairs
with the `metacognition` + `gpa-evolution` skills: the M5 spans become the
GEPA trajectory data for offline evolution of the self-calibration template.

**Deferred:** GEPA offline evolution of the template itself — waits on real
`reg.meta.self_calibration` trajectory data accumulating from runtime.

## Pre-existing issue (self-heal) — confirmed resolved

`hkask-services-self-heal` had an `unsafe { std::env::set_var(...) }` under
`#![deny(unsafe_code)]`. Examined: committed HEAD correctly scopes
`#[allow(unsafe_code)]` (healer.rs:423) on the single function containing the
unsafe block (healer.rs:460), with a valid SAFETY comment (single-threaded
startup). `git diff --stat` is empty (working tree == HEAD). Compiles clean in
lib + test modes. The earlier `cargo test` failure was a transient working-tree
state mid-edit. No fix needed; the scoped allow is idiomatic for edition-2024
`set_var`.