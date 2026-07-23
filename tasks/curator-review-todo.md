# Curator Review — Todo

- [x] S1: dedup `GoalExpiredCount` arm (curation_loop.rs); remove dead `_metric`/`_target` (hloop_impl.rs)
- [x] S2: typed `GoalLifecycle` enum for `to_state` matching (channels.rs + curation_loop.rs sense)
- [x] S3: rewire act_on_throttle/act_on_escalate from HealthSnapshot + typed RegulationData; delete param_str/param_u64 stubs
- [x] S4: feed HealthSnapshot into escalate template context (hloop_impl.rs act batch branch)
- [x] S5: thread new_budget/target through OverrideEnergyBudget (new RegulationData::CuratorBudgetOverride variant); externalize 5000 constant
- [x] S6: typed CurationEscalationReason enum for CurationLoop compute/act dispatch
- [x] S7: unify the two EscalationSeverity types (metacognition now uses hkask_types::curator::EscalationSeverity)
- [x] S8a: remove dead MetacognitionConfig.interval + DEFAULT_METACOGNITION_INTERVAL_SECS
- [x] S8b: dedup double try_auto_consolidate() call in CurationLoop::act()
- [x] L3: deleted unused CuratorHandle::new_test() (zero callers)
- [x] L4: eliminated always-pass OCAP gate in issue_directive; authority now enforced by singleton construction; removed dead DataCategory import
- [x] Open Q (reg.* spans): examined — see plan §"Open Q resolution". Deliberate non-circularity confirmed; future self-management direction documented, not implemented.
- [x] Pre-existing (self-heal unsafe): examined + confirmed resolved. Committed HEAD correctly scopes `#[allow(unsafe_code)]` on the single set_var function with a valid SAFETY comment; earlier failure was a transient working-tree state. No fix needed.
- [x] Final: cargo clippy -D warnings (hkask-types, hkask-regulation, hkask-pods, hkask-services-context, hkask-services-chat, hkask-mcp-curator) + cargo test (types 80, regulation 167, pods 31)