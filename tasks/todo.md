# todo — Replicant→Userpod Consolidation (ADVISORY, no autonomous deletion)

> Phase 0 is BLOCKING. No code tasks are executable until the user resolves the thesis
> mismatch (§0 of plan.md). Every item below is advisory pending accept/reject.

## Phase 0 — Thesis reconciliation (no code)
- [ ] **T0.1** User answers Q1–Q4: did "replicant→userpod" mean (a) Bot/Replicant AgentKind split, (b) AgentPod/PodDeployment wrapper, (c) CuratorSync, or (d) something else? — *Checkpoint: UserFeedbackOccurrence*
- [ ] **T0.2** If re-scope = S-b: trace `PodDeployment.pod` callers across all crates (Q6) → caller table with file:line → G1 verdict on S-b — *Checkpoint: build green*

## Phase 1 — Conditional on T0.2 confirming S-b pass-through
- [ ] **T1.1** Inline `AgentPod` fields into `PodDeployment` for PodKind::Replicant ONLY; rewrite its callers; run `agent_pod_integration.rs` (≤5 files) — *Checkpoint: build + TUI smoke*
- [ ] **T1.2** Extend inlining to PodKind::Curator + PodKind::Team; run `pod_portability.rs` — *Checkpoint: User*

## Verification gates (advisory)
- [ ] `cargo build -p hkask-tui` compile time does not increase vs metric_before
- [ ] `agent_pod_integration.rs`, `integration_depth.rs`, `pod_portability.rs` green
- [ ] No `todo!()`/`unimplemented!()`/`#[deprecated]` introduced (P5)
- [ ] No `Result<_, String>` introduced (CI gate)
- [ ] Every userpod action retains authenticated author (P12) — audit CNS spans

## Open questions (resolve before any deletion)
- [ ] Q1 Did "replicant vs user" actually mean "Bot vs Replicant" (AgentKind)?
- [ ] Q2 Did "pod-spawn indirection" mean the AgentPod/PodDeployment wrapper?
- [ ] Q3 Did "mediation layer" mean CuratorSync?
- [ ] Q4 Is there a "replicant crate" expectation? (None exists.)
- [ ] Q5 Locate S4/S5 policy components in VSM for the pod subgraph.
- [ ] Q6 Trace ALL callers of `PodDeployment.pod` (grep `\.pod\.` repo-wide).
- [ ] Q7 Read remaining 9 lines of `resolve_replicant_name` (`services-skill/src/skill_impl.rs:302+`).

## G1 verdicts recorded (advisory — all KEEP or Hypothesis)
- [x] S-a Merge HumanUser+ReplicantIdentity → **KEEP** (1:N, G1 FAIL) — Evidence
- [ ] S-b Collapse AgentPod+PodDeployment → **Hypothesis** (needs Q6) — dropped from plan
- [x] S-c Remove AgentKind::Bot/Replicant → **KEEP** (G1 FAIL) — Evidence
- [x] S-d Remove PodKind trichotomy → **KEEP** (G1 FAIL) — Evidence
- [x] S-e Delete CuratorSync → **KEEP** (G1 FAIL, S3 component) — Evidence
- [x] S-f Inline resolve_replicant_name → **KEEP** (G1 FAIL on inline, duplicates) — Evidence