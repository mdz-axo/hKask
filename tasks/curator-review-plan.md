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

**Future direction (NOT implemented — user said "not there yet"):** for the
curator to learn to manage itself, add a `reg.meta.*` namespace for
curator-decision-quality metrics (directive acceptance rate, escalation
backlog, consolidation lag, circuit-breaker trips) consumed by a dedicated
meta-observer DISTINCT from `CurationLoop` (so CurationLoop never reads its own
spans). Authority DAG gains a meta-level: Meta -> Curation -> Cybernetics ->
domains. This pairs with the existing `metacognition` + `gpa-evolution` skills.
Defer until the core code is stable and efficient.

## Pre-existing issue (self-heal) — confirmed resolved

`hkask-services-self-heal` had an `unsafe { std::env::set_var(...) }` under
`#![deny(unsafe_code)]`. Examined: committed HEAD correctly scopes
`#[allow(unsafe_code)]` (healer.rs:423) on the single function containing the
unsafe block (healer.rs:460), with a valid SAFETY comment (single-threaded
startup). `git diff --stat` is empty (working tree == HEAD). Compiles clean in
lib + test modes. The earlier `cargo test` failure was a transient working-tree
state mid-edit. No fix needed; the scoped allow is idiomatic for edition-2024
`set_var`.