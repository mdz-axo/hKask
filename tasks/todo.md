# todo — Replicant→UserPod Consolidation (COMPLETE)

> Complete. Code consolidation done. This todo is retained as a historical record.
> Order = gpa-evolution iter 3: types-first, principles as own phase.

## Phase 0 — RESOLVED (no code)
- [x] **T0.1** 1:1 userpod (multi-persona removed); A2A transport kept (userpods present as agents); no crate rename; persistent userpods; curator = systemd daemon. — *Done*

## Phase 1 — Foundation types
- [x] **T1.1a** Rename `ReplicantIdentity`→`UserPod` + strangler-fig alias `pub type ReplicantIdentity = UserPod;` (hkask-identity). *Workspace build GREEN; alias removed in Phase 6.* — *Done 2026-07-20*
- [x] **T1.1b** Drop `is_primary` from API surface (S-1to1-b) + CLI prints (S-1to1-c). Switcher removed from terminal UI. *Committed 0c06d9f7; workspace green.* — *Done 2026-07-20*
- [ ] **T1.1c (deferred migration)** Drop `is_primary` field from `UserPod` + `user_store.rs` mapping + DB column — *deferred per user decision (leave DB column for now)*
- [x] **T1.1d (cleanup)** Remove orphan `#replicant-select` CSS rule at `terminal.rs:229` — *Done 2026-07-20*
- [x] **S-AK-a** Drop `agent_type` from `A2AAgent` + `register_agent` signature (a2a/mod.rs); from `AgentPod`+`new()`+`register()` (pod/mod.rs); from `PodStatusInfo`+2 constructions+`activate_pod` (active_pods.rs); fix 9 callers (registry_loader, onboarding, token.rs, reg_wallet.rs, api a2a.rs/pods.rs, repl agent.rs/pod.rs). *AgentKind enum retained (persona system) — deleted in T2.2.* *Workspace green; a2a 11/11 + agents tests green. Committed ce7cd7ba by concurrent process.* — *Done 2026-07-20*

## Phase 2 — UserPod runtime (focus obstacle, early)
- [x] **T2.0** Consumer trace written to `tasks/consumer-trace.md` (ReplicantIdentity/AgentKind/is_primary/list_replicants) — *Done 2026-07-20*
- [ ] **T2.1** Fold `AgentPod`+`PodDeployment`→`UserPod` (deep: SQLCipher+CNS+capability); persistent (no Deactivated). Rewrite `agent_pod_integration.rs`→`userpod_integration.rs`, `pod_portability.rs` — *Acc: UserPod deploys+persists; tests green; ≤5 files; checkpoint: User*
- [x] **T2.2a** Strangler-fig: add direct `name`+`capabilities` fields to `AgentPod` (populated from persona in `new()`); migrate 7 `active_pods.rs` reads off `persona.agent.name`/`persona.capabilities` → `d.pod.name`/`d.pod.capabilities`. *Persona field retained for now. Workspace green; agents tests pass.* — *Done 2026-07-20*
- [ ] **T2.2b** Migrate `deployment.rs` reads (`PodFactory::deploy`, `create_pod_storage`) off persona; change `deploy`/`create_pod` signatures to take name+webid+capabilities directly (persona optional, curator-only) — *Acc: deploy green; ≤3 files*
- [ ] **T2.2c** Migrate `persona()` accessor + `has_capability` + tests (`make_test_persona`, `AgentPersona::system` callers) off the AgentPod.persona field — *Acc: tests green; ≤4 files*
- [ ] **T2.2d** Remove `persona` field from `AgentPod`; curator keeps a separate `CuratorPersona` type (T2.2 userpod-persona split complete) — *Acc: no persona on userpod runtime; ≤2 files; checkpoint: User*

## Phase 3 — Curator daemon promotion
- [ ] **T3.1** Promote curator OUT of `PodKind::Curator` to first-class systemd daemon (`kask serve` already generates unit at `init.rs:180`); `CuratorSync` polls UserPods via `curator_index`. Keep test harness — *Acc: `kask serve` runs curator; CuratorSync green vs UserPod; ≤5 files; checkpoint: User*

## Phase 4 — A2A + lifecycle
- [ ] **T4.0 (DISCOVERY, gates T4.2)** Define pod-offline behavior (Q-LIFE-DISC): 1-page design doc with options — (a) pod sleeps (storage-at-rest, no compute, no A2A), (b) pod runs headless (A2A-reachable, no inference), (c) maintenance mode for inactive-not-cancelled. User picks one — *Acc: design doc with chosen option; checkpoint: UserFeedbackOccurrence*
- [ ] **T4.1** A2A: keep transport; delete Bot registration path; userpods + curator register as agents (no `AgentKind`) — *Acc: no Bot path; userpod+curator A2A register; MCP tools unaffected; ≤4 files*
- [ ] **T4.2** Collapse `PodLifecycleState`→persistent + register-on-start, per T4.0 decision (≤2–3 states incl. offline state); remove `Deactivated`/teardown — *Acc: states match T4.0 design; tests green; ≤3 files; checkpoint: User*

## Phase 5 — Principles + docs (in scope per user)
- [x] **T5.1** Edit P6 "Space for Replicants & Bots"→"Space for UserPods" + P6.1 1:1 (`PRINCIPLES.md:128-131`); P5.2 "Who" drop replicant/bot (`:83`); P9 authority (`:200`) — *Done 2026-07-20*
- [x] **T5.2** Refocus P10 Bot/Replicant Taxonomy → **P10 User Agency** (`PRINCIPLES.md:206-208`); Twelve Principles stay twelve — *Done 2026-07-20 (folded into T5.1 edit pass)*
- [x] **T5.3a** Retitle P12→"Authenticated Host Mandate"; rewrite P12.1 surface-host table (drop Bot row; CLI=user+curator, Daemon=curator, API=userpods) (`PRINCIPLES.md:214-230`) — *Done 2026-07-20 (folded into T5.1 edit pass)*
- [x] **T5.3b** Rewrote mandate doc as `docs/architecture/mandates/P12-authenticated-host-mandate.md` (old `P12-replicant-host-mandate.md` was a dangling reference — dir didn't exist); fixed `PRINCIPLES.md:219` citation — *Done 2026-07-20*
- [x] **T5.4 (core)** Swept code comments: `deployment.rs:11` (P6→UserPods), `openapi.rs:59` (P12→Authenticated), `identity/lib.rs:226` (P6→UserPods), `ports/registry.rs:110-113` (userpod/human-users), `mcp/runtime.rs:247` (P12→authenticated-host), `test-harness/lib.rs:17` (P12→Authenticated), `tui/windows/chat.rs:16` (P12→Authenticated), `FUNCTIONAL_SPECIFICATION.md:1412,1433` (P12→Authenticated). *Workspace green.* — *Done 2026-07-20*
- [ ] **T5.4 (remaining docs)** `TESTING_DISCIPLINE.md:249,296,299`, `hKask-architecture-master.md:2659,2906,2910,2912`, `architecture-patterns.md:479` — principle-cited doc references still name Replicant/Bot (low-risk doc consistency; deferred to avoid racing concurrent doc edits)
- [ ] **T5.5** Sweep skill docs: `attack-taxonomy-mapper`, `runtime-posture-monitor`, `supply-chain-sentinel` — rename `replicant_host` span field per Q-SPAN — *Acc: skills consistent; ≤3 files; checkpoint: User*

## Phase 6 — Surface rewiring + verification
- [ ] **T6.1** `hkask-api/routes/replicant.rs`→`routes/userpod.rs`; remove `list_replicants` + terminal switcher — *Acc: API green; ≤4 files*
- [ ] **T6.2** `hkask-cli ReplicantAction`→`UserPodAction` — *Acc: CLI green; ≤3 files*
- [ ] **T6.3** `hkask-tui ReplicaDataBridge`→`UserPodDataBridge` — *Acc: TUI builds + smoke; ≤4 files; checkpoint: User*
- [ ] **T6.4** Full verify: `cargo build --workspace`; `cargo test`; TUI flows; record `metric_after` — *Acc: green; metric_after JSON*

## Phase 7 — TUI window pruning (in progress)
- [x] **T7.0** Remove `WindowKind::Logo` + `WindowKind::Curator` variants; delete `windows/logo.rs` + `windows/curator.rs`; merge Curator chat into `ChatWindow` as default `TuiMode::Curator`; simplify default workspace layout to single Chat pane; add window-management slash commands (`/open`, `/close`, `/split`, `/focus`, `/tab`, `/palette`, `/quit`); add `WorkspaceAction` + `Window::drain_action()` for chat→workspace communication — *Done 2026-07-21*
- [x] **T7.1** Delete `WindowKind::CnsMonitor` + `windows/reg_monitor.rs` — CNS health is visible via Curator chat and status bar; dedicated monitor window is redundant complexity — *Done 2026-07-21*
- [x] **T7.2** Delete `WindowKind::Backup` + `windows/backup.rs` + `bridges/backup.rs` — backup operations are CLI-only (`kask backup`); TUI window adds surface area with minimal use case — *Done 2026-07-21*
- [x] **T7.3** Delete `WindowKind::Registry` + `windows/registry.rs` — registry browsing is CLI-only (`kask registry`); TUI window duplicates CLI surface. `RegistryDataBridge` retained for SkillsWindow — *Done 2026-07-21*
- [x] **T7.4** Delete `WindowKind::Pods` + `windows/pods.rs` — pod status is visible via Curator chat and `/status` command; dedicated pods window is redundant — *Done 2026-07-21*
- [x] **T7.5** Update `WindowKind::META` count (16), integration tests, README, and `tui-architecture.md` after all T7.1–T7.4 deletions — *Done 2026-07-21*

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