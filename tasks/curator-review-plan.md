# Curator Review — Plan

**Scope:** Evidence-grounded cleanup of the curator (regulatory + persona halves).
**Status:** Discovery complete. Executing vertical slices bottom-up.

## Architecture (verified)

The curator is NOT a god-object — it was already decomposed in "Curation/Agent
Separation (Task 6)". Two `RegulationLoop` impls share `LoopId::Curation`:
`CurationLoop` (pure regulatory) and `MetacognitionLoop` (persona), ticked by
`LoopScheduler` at 10s. Matrix is a thin seam (events via channel, not a god-object).

## Findings (consolidated, evidence-backed)

### CRITICAL
- **C1** — Metacognition `act()` is effectively dead. `param_str`/`param_u64`
  (`loop_body.rs` L266-274) are stubs that always return defaults (refactor
  residue from `RegulationData` typing). `act_on_throttle` always returns None
  (domain always `""`); `act_on_escalate` always returns None (metric always
  `""`). No escalations or directives issued from `MetacognitionLoop::act()`.
  No tests cover these helpers.

### HIGH
- **H1** — `hloop_impl.rs` `act()` batch branch builds the escalate template
  context with empty/hardcoded defaults (`critical_issues: []`, `variety_deficit:
  0`) instead of the actual `HealthSnapshot` (available via `last_snapshot_tx`,
  proven by `compute_with_templates` L397). LLM escalation notification is
  generated with fake context.
- **H2** — `compute_with_templates` `adjust_budget` branch reads `new_budget` +
  `target` from the LLM plan but discards them — the action carries only
  `reason: "adjust_budget"`. Connects to M1's hardcoded 5000.

### MEDIUM
- **M1** — Hardcoded `new_budget: 5000` in `CurationLoop::act()` L572.
- **M2** — Stringly-typed action dispatch (`reason == "..."`) in `CurationLoop::act()`.
- **M3** — Stringly-typed state dispatch (`to_state == "stale"`) in `CurationLoop::sense()`.
- **M4** — `EscalationSeverity` defined twice: `hkask-types::curator` (Info/Warning/Critical)
  vs `metacognition::escalation` (Warning/Critical). Naming collision, drift risk.

### LOW
- **L1** — Duplicate `GoalExpiredCount` match arm (`curation_loop.rs` L502-519). Escapes clippy (identical guards).
- **L2** — Dead locals `_metric`/`_target` in `hloop_impl.rs` act() L144-145.
- **L3** — `CuratorHandle::new_test()` is `pub` — weakens singleton.
- **L4** — OCAP gate in `issue_directive` structurally always-pass. Essentialist target.
- **L5** — `MetacognitionConfig.interval` (3600s) dead — loop driven at 10s.
- **L6** — Double `try_auto_consolidate()` call risk in one `act()` pass.
- **L7** — `reg_health` shadowed in `hloop_impl.rs` sense() L29/L35.

## Vertical slices (bottom-up, each ≤5 files)

| Slice | Fixes | Risk | Deps |
|-------|-------|------|------|
| **S1** | L1, L2 — dedup match arm, remove dead locals | Trivial | none |
| **S2** | M3 — typed `GoalState` enum for `to_state` matching | Low | none |
| **S3** | C1 — rewire `act_on_throttle`/`act_on_escalate` to source data from `HealthSnapshot` + typed `RegulationData` instead of stub `param_str`/`param_u64`; delete the stubs | Medium | S2 |
| **S4** | H1 — feed actual `HealthSnapshot` into escalate template context | Low | S3 |
| **S5** | H2+M1 — thread `new_budget`/`target` through `OverrideEnergyBudget`; externalize the 5000 constant | Medium | S3 |
| **S6** | M2 — typed `CurationReason` enum replacing string dispatch in `CurationLoop::act()` | Medium | S2 |
| **S7** | M4 — unify the two `EscalationSeverity` types | Low | none |
| **S8** | L4,L5,L6 — essentialist: remove always-pass OCAP gate OR make it real; remove dead `interval`; dedup consolidation call | Low | S3 |

## Open questions (unresolved)
1. Should the curator emit `reg.curator.*` spans, or is the tracing-target-only approach intentional?
2. L4: remove the always-pass OCAP gate, or make it enforce a real capability check?
3. Public/private memory boundary: keep runtime `DataCategory` gate or move to type-enforcement?

## Validation
`cargo clippy -D warnings` + `cargo test -p hkask-pods` after each slice.