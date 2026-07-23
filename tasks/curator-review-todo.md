# Curator Review — Todo

- [x] S1: dedup GoalExpiredCount arm; remove dead _metric/_target
- [x] S2: typed GoalLifecycle enum for to_state matching
- [x] S3: rewire act_on_throttle/act_on_escalate from HealthSnapshot; delete param_str/param_u64 stubs
- [x] S4: feed HealthSnapshot into escalate template context
- [x] S5: thread new_budget/target through OverrideEnergyBudget (CuratorBudgetOverride variant); externalize 5000
- [x] S6: typed CurationEscalationReason enum for CurationLoop compute/act dispatch
- [x] S7: unify the two EscalationSeverity types (re-applied after revert)
- [x] S8a: remove dead MetacognitionConfig.interval
- [x] S8b: dedup double try_auto_consolidate() call
- [x] L3: deleted unused CuratorHandle::new_test()
- [x] L4: eliminated always-pass OCAP gate; authority by singleton construction
- [x] M1: reg.meta canonical namespace + MetaSpan enum (4 variants) + emission helpers + canonical test
- [x] M2: CuratorContext regulation_sink + SelfQuality counters; emit reg.meta.directive at issue_directive; build/loops.rs wiring
- [x] M3: EscalationPolicy mutable thresholds + self_calibrate meta-cybernetic loop + emit reg.meta.escalation/circuit_breaker + self_calibrate call in act()
- [x] M4: effectiveness-driven LOWERING (bidirectional self-calibration). Pure compute_threshold_adjustment: raise 10pct (ceiling 4x default) + lower 5pct (floor = default) + hysteresis cooldown + min-observations gate (aligned with the existing SetPointCalibrator pattern). 10 unit tests.
- [x] M5: calibration-effectiveness MEASUREMENT (the GEPA precondition). PendingCalibration records eff_before at apply time; closed out on next self_calibrate with eff_after = current effectiveness; reg.meta.self_calibration span now carries eff_before/eff_after/eff_delta. This is the causal signal a future learner needs - and the falsifiability test of whether threshold changes produce a detectable effectiveness signal.
- [x] GEPA learner: DEFERRED (deliberate). GEPA is an offline prompt-artifact skill (v1 evolves prompts, not Rust), there is no runtime GEPA infra, and there is no trajectory data yet. Building a learner before the M5 signal is empirically validated would be gold-plating. The M5 spans become the GEPA trajectory data once the system runs.
- [x] Final: clippy -D warnings (hkask-types, hkask-regulation, hkask-pods, hkask-services-context, hkask-services-chat, hkask-mcp-curator) + tests (types 80, regulation 169, pods 44) + reg-canonical gate OK

## Known unrelated pre-existing issues (NOT curator; user WIP in flux)
- hkask-inference/src/ollama_backend.rs — actively being edited; transient syntax errors
  observed across runs. Not touched (out of curator scope, in flux).
- hkask-services-self-heal — confirmed OK (scoped allow(unsafe_code) + SAFETY comment in HEAD).

## Reusable existing infrastructure consulted
- hkask-regulation/src/set_point_calibrator.rs — the codebase's established self-tuning
  threshold pattern (10pct step, clamp bounds, min_observations gate, Conant-Ashby closure).
  Consulted for M4 discipline; curator self-calibration follows the same pattern rather
  than coupling to it (distinct concern: EscalationPolicy + non-circular SelfQuality channel).
- .agents/skills/gpa-evolution — read for M5/GEPA scoping; confirmed it is an offline
  prompt-artifact evolutionary optimizer, not runtime infrastructure.