# todo — Replicant→Userpod Consolidation (RE-SCOPED, advisory)

> Phase 0 BLOCKING. Each slice flagged for user accept/reject. No autonomous deletion.
> Order is bottom-up vertical slices (gpa-evolution iter 3: types-first).

## Phase 0 — Confirmations (no code)
- [ ] **T0.1** User confirms Q-1N (1:1 userpod?), Q-A2A (delete A2A transport?), Q-NAME (crate rename target?), Q-LIFE (lifecycle shape?) — *Checkpoint: UserFeedbackOccurrence*

## Phase 1 — Foundation types
- [ ] **T1.1** Rename `ReplicantIdentity`→`UserPod`; drop `is_primary` (if Q-1N=1:1). Update `hkask-identity` + tests — *Acc: identity compiles, tests green; ≤3 files; checkpoint: User*
- [ ] **T1.2** Delete `AgentKind` enum; delete `PodKind::Team`; rename `PodKind::Replicant`→`PodKind::UserPod`. Update `hkask-types` — *Acc: hkask-types compiles (consumers broken, expected); ≤2 files*

## Phase 2 — UserPod runtime (focus obstacle, high-risk, early)
- [ ] **T2.0** Trace ALL consumers of `AgentPod`/`PodDeployment` repo-wide (Q6) → caller table — *Acc: table with file:line; checkpoint: User*
- [ ] **T2.1** Collapse `AgentPod`+`PodDeployment`→`UserPod` (deep: owns SQLCipher+CNS+capability). Rewrite `agent_pod_integration.rs`→`userpod_integration.rs` — *Acc: UserPod deploys a user; tests green; ≤5 files; checkpoint: User*
- [ ] **T2.2** Delete `AgentPersona`/`AgentCharter`/`AgentIdentity`; remove persona YAML path — *Acc: no Agent* persona types; ≤4 files*

## Phase 3 — Curator daemon extraction
- [ ] **T3.1** Extract curator → `kask curator` subcommand + systemd unit. `CuratorSync` polls `UserPod`s. Keep test harness mode — *Acc: `kask curator` runs; CuratorSync green vs UserPod; ≤5 files; checkpoint: User*

## Phase 4 — A2A + lifecycle + rename
- [ ] **T4.1** Delete A2A agent registration (per Q-A2A: delete or repurpose transport) — *Acc: no agent registration; MCP tool path unaffected; ≤4 files*
- [ ] **T4.2** Simplify lifecycle `Pop→Reg→Act→Deact`→userpod deploy/teardown (per Q-LIFE) — *Acc: ≤3 states; tests green; ≤3 files*
- [ ] **T4.3** Rename `hkask-agents`→target (per Q-NAME) — *Acc: workspace builds; ≤2 files; checkpoint: User*

## Phase 5 — Surface rewiring + verification
- [ ] **T5.1** Rewire `hkask-api/routes/replicant.rs`→`routes/userpod.rs`; remove terminal switcher if 1:1 — *Acc: API green; ≤4 files*
- [ ] **T5.2** Rewire `hkask-cli ReplicantAction`→`UserPodAction` — *Acc: CLI green; ≤3 files*
- [ ] **T5.3** Rewire `hkask-tui ReplicaDataBridge`→`UserPodDataBridge` — *Acc: TUI builds + smoke; ≤4 files; checkpoint: User*
- [ ] **T5.4** Full verification: `cargo build --workspace`; `cargo test -p <renamed>`; TUI flows; record metric_after — *Acc: green; metric_after JSON recorded*

## Verification gates (advisory, every 2–3 slices)
- [ ] Build passes after Phase 1, Phase 2, Phase 3, Phase 5
- [ ] TUI flows (register, login, terminal, skills, MCP) end-to-end after T5.3
- [ ] No `todo!()`/`unimplemented!()`/`#[deprecated]` introduced (P5)
- [ ] No `Result<_, String>` introduced (CI gate)
- [ ] Every userpod action retains authenticated author (P12) — CNS spans audited
- [ ] TUI compile time does not increase vs metric_before

## Open questions (resolve before/deleting)
- [ ] Q-1N 1:1 userpod, or preserve 1:N multi-persona? (lean 1:1 per user msg)
- [ ] Q-A2A delete A2A entirely, or keep transport for MCP tool-to-tool?
- [ ] Q-NAME `hkask-userpods` vs `hkask-services` vs other?
- [ ] Q-LIFE userpod lifecycle: deploy/teardown only, or keep activated/deactivated?
- [ ] Q5 S4/S5 VSM policy components for userpod subgraph (unlocated)
- [ ] Q6 ALL consumers of `AgentPod`/`PodDeployment` (trace repo-wide before T2.1)
- [ ] Q7 `resolve_replicant_name`→`resolve_userpod_name`? read `skill_impl.rs:302+`

## G1 verdicts (vs NEW target: agents/bots/A2A gone)
- [x] S-1 Delete `AgentKind` — **DELETE** (Guideline)
- [x] S-2 Delete `AgentPod`→`UserPod` — **DELETE** (Guideline)
- [x] S-3 Fold `PodDeployment` into `UserPod` — **DELETE/FOLD** (Guideline)
- [x] S-4 Delete `PodKind::Team` — **DELETE** (Guardrail, spec)
- [x] S-5 Rename `PodKind::Replicant`→`UserPod` — **RENAME** (Evidence)
- [x] S-6 Delete `AgentPersona`/`AgentCharter`/`AgentIdentity` — **DELETE** (Guideline)
- [ ] S-7 Delete A2A agent reg; transport? — **DELETE reg; Hypothesis on transport** (Q-A2A)
- [x] S-8 Simplify lifecycle — **SIMPLIFY** (Guideline)
- [x] S-9 Extract curator as systemd daemon — **EXTRACT** (Guideline)
- [x] S-10 Rename `hkask-agents` — **RENAME** (Guideline)
- [ ] S-11 Rename `ReplicantIdentity`→`UserPod`, 1:1 — **RENAME; gated Q-1N** (Hypothesis)