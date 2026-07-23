# Curator Review — Todo

- [ ] S1: dedup `GoalExpiredCount` arm (curation_loop.rs); remove dead `_metric`/`_target` (hloop_impl.rs)
- [ ] S2: typed `GoalState` enum for `to_state` matching (curation_loop.rs sense)
- [ ] S3: rewire act_on_throttle/act_on_escalate from snapshot + typed RegulationData; delete param_str/param_u64 stubs
- [ ] S4: feed HealthSnapshot into escalate template context (hloop_impl.rs act batch branch)
- [ ] S5: thread new_budget/target through OverrideEnergyBudget; externalize 5000 constant
- [ ] S6: typed CurationReason enum for CurationLoop::act dispatch
- [ ] S7: unify the two EscalationSeverity types
- [ ] S8: essentialist — OCAP gate, dead interval, dedup consolidation
- [ ] Final: cargo clippy -D warnings + cargo test -p hkask-pods