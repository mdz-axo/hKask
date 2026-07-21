# todo — Replicant→UserPod Consolidation (FINAL, Phase 0 resolved)

> Advisory. Each slice flagged for user accept/reject. No autonomous deletion.
> Order = gpa-evolution iter 3: types-first, principles as own phase.

## Phase 0 — RESOLVED (no code)
- [x] **T0.1** 1:1 userpod (multi-persona removed); A2A transport kept (userpods present as agents); no crate rename; persistent userpods; curator = systemd daemon. — *Done*

## Phase 1 — Foundation types
- [ ] **T1.1** Rename `ReplicantIdentity`→`UserPod`; drop `is_primary`; fields `replicant_name`→`userpod_name`, `replicant_webid`→`webid`. Update `hkask-identity` + tests — *Acc: identity compiles, tests green; ≤3 files; checkpoint: User*
- [ ] **T1.2** Delete `AgentKind` (Q-AK); delete `PodKind::Team`; rename `PodKind::Replicant`→`PodKind::UserPod`. Update `hkask-types` — *Acc: hkask-types compiles (consumers broken, expected); ≤2 files*

## Phase 2 — UserPod runtime (focus obstacle, early)
- [ ] **T2.0** Trace ALL `AgentPod`/`PodDeployment` consumers repo-wide (Q6) → caller table — *Acc: table file:line; checkpoint: User*
- [ ] **T2.1** Fold `AgentPod`+`PodDeployment`→`UserPod` (deep: SQLCipher+CNS+capability); persistent (no Deactivated). Rewrite `agent_pod_integration.rs`→`userpod_integration.rs`, `pod_portability.rs` — *Acc: UserPod deploys+persists; tests green; ≤5 files; checkpoint: User*
- [ ] **T2.2** Delete persona from userpods; rename `AgentPersona`→`CuratorPersona` (curator-only). Remove persona-YAML from userpod creation; userpod presents in A2A via WebID+name+capabilities only — *Acc: no persona on userpods; curator persona intact; ≤4 files*

## Phase 3 — Curator daemon promotion
- [ ] **T3.1** Promote curator OUT of `PodKind::Curator` to first-class systemd daemon (`kask serve` already generates unit at `init.rs:180`); `CuratorSync` polls UserPods via `curator_index`. Keep test harness — *Acc: `kask serve` runs curator; CuratorSync green vs UserPod; ≤5 files; checkpoint: User*

## Phase 4 — A2A + lifecycle
- [ ] **T4.0 (DISCOVERY, gates T4.2)** Define pod-offline behavior (Q-LIFE-DISC): 1-page design doc with options — (a) pod sleeps (storage-at-rest, no compute, no A2A), (b) pod runs headless (A2A-reachable, no inference), (c) maintenance mode for inactive-not-cancelled. User picks one — *Acc: design doc with chosen option; checkpoint: UserFeedbackOccurrence*
- [ ] **T4.1** A2A: keep transport; delete Bot registration path; userpods + curator register as agents (no `AgentKind`) — *Acc: no Bot path; userpod+curator A2A register; MCP tools unaffected; ≤4 files*
- [ ] **T4.2** Collapse `PodLifecycleState`→persistent + register-on-start, per T4.0 decision (≤2–3 states incl. offline state); remove `Deactivated`/teardown — *Acc: states match T4.0 design; tests green; ≤3 files; checkpoint: User*

## Phase 5 — Principles + docs (in scope per user)
- [ ] **T5.1** Edit P6 "Space for Replicants & Bots"→"Space for UserPods" + P6.1 1:1 (`PRINCIPLES.md:128-131`); P5.2 "Who" drop replicant/bot (`:83`); P9 authority (`:200`) — *Acc: PRINCIPLES.md consistent; ≤1 file*
- [ ] **T5.2** Refocus P10 Bot/Replicant Taxonomy → **P10 User Agency** (`PRINCIPLES.md:206-208`); Twelve Principles stay twelve — *Acc: P10 retitled+refocused to user agency/sovereignty; ≤1 file; checkpoint: User*
- [ ] **T5.3** Retitle P12→"Authenticated Host Mandate"; rewrite P12.1 surface-host table (drop Bot row; CLI=user+curator, Daemon=curator, API=userpods) (`PRINCIPLES.md:214-230`); rewrite `mandates/P12-replicant-host-mandate.md` — *Acc: P12 consistent; ≤2 files*
- [ ] **T5.4** Sweep code comments: `deployment.rs:11`, `openapi.rs:59`, `identity/lib.rs:222`, `ports/registry.rs:113`, `mcp/runtime.rs:246`, `test-harness/lib.rs:17`, `tui/windows/chat.rs:16`, `FUNCTIONAL_SPECIFICATION.md` — *Acc: no "replicant/bot" in principle-cited comments; ≤5 files*
- [ ] **T5.5** Sweep skill docs: `attack-taxonomy-mapper`, `runtime-posture-monitor`, `supply-chain-sentinel` — rename `replicant_host` span field per Q-SPAN — *Acc: skills consistent; ≤3 files; checkpoint: User*

## Phase 6 — Surface rewiring + verification
- [ ] **T6.1** `hkask-api/routes/replicant.rs`→`routes/userpod.rs`; remove `list_replicants` + terminal switcher — *Acc: API green; ≤4 files*
- [ ] **T6.2** `hkask-cli ReplicantAction`→`UserPodAction` — *Acc: CLI green; ≤3 files*
- [ ] **T6.3** `hkask-tui ReplicaDataBridge`→`UserPodDataBridge` — *Acc: TUI builds + smoke; ≤4 files; checkpoint: User*
- [ ] **T6.4** Full verify: `cargo build --workspace`; `cargo test`; TUI flows; record `metric_after` — *Acc: green; metric_after JSON*

## Verification gates (every 2–3 slices)
- [ ] Build passes after Phase 1, 2, 3, 5, 6
- [ ] TUI flows (register, login, terminal, skills, MCP) end-to-end after T6.3
- [ ] No `todo!()`/`unimplemented!()`/`#[deprecated]` introduced (P5)
- [ ] No `Result<_, String>` introduced (CI gate)
- [ ] Every userpod action retains authenticated author (P12-as-retitled) — CNS spans audited
- [ ] `scripts/check-cns-canonical.sh` passes after span field rename (T5.5)
- [ ] TUI compile time ≤ metric_before

## Open questions (remaining after Phase 0b)
- [x] Q-AK delete `AgentKind` entirely — **RESOLVED (delete)**
- [x] Q-PERSONA drop persona from userpods; curator keeps (rename `CuratorPersona`) — **RESOLVED**
- [x] Q-P10 refocus P10 to user agency (Twelve stay twelve) — **RESOLVED**
- [x] Q-SPAN `replicant_host`→`userpod_host` — **RESOLVED**
- [ ] Q-LIFE-DISC pod-offline behavior — **OPEN (T4.0 discovery)**: logged-out user? inactive-not-cancelled account?
- [ ] Q6 full consumer trace (T2.0)

## G1 verdicts (vs confirmed target)
- [x] S-1 Delete `AgentKind` — **DELETE** (Guideline, Q-AK)
- [x] S-2 Fold `AgentPod`→`UserPod` — **FOLD** (Guideline)
- [x] S-3 Fold `PodDeployment`→`UserPod` — **FOLD** (Guideline)
- [x] S-4 Delete `PodKind::Team` — **DELETE** (Guardrail)
- [x] S-5 Rename `PodKind::Replicant`→`UserPod` — **RENAME** (Evidence)
- [x] S-6 Delete persona from userpods; curator keeps (rename `CuratorPersona`) — **DELETE+RENAME** (Guardrail)
- [x] S-7 A2A transport kept; Bot path deleted — **KEEP+DELETE** (Evidence)
- [x] S-8 Collapse lifecycle → persistent — **SIMPLIFY** (Guardrail)
- [x] S-9 Curator as systemd daemon — **EXTRACT/promote** (Evidence)
- [x] S-11 Rename `ReplicantIdentity`→`UserPod`, 1:1 — **RENAME** (Guardrail)
- [x] S-12 Refocus P10 to user agency (not retire); edit P5.2/P6/P9/P12 + mandate + comments + skills — **EDIT/REFOCUS** (Guardrail)