# Plan — Replicant→Userpod Consolidation (RE-SCOPED)

> **Status:** ADVISORY. No code deleted. Each slice flagged for user accept/reject.
> **Re-scope (user-confirmed):** hKask = minimal viable container for AI tools. No agents,
> no bots, no agent-pod lifecycle. Only: human users (each → 1 userpod, Solid-Pod-modeled),
> AI tools (skills + MCP servers), and the curator (systemd daemon; the one surviving
> "replicant" by name). All intermediate agent/replicant/bot abstractions are DELETION TARGETS.
> User sovereignty preserved (WebID, OCAP, per-pod SQLCipher).

---

## 0. Correction of prior framing

My v1 plan ran the essentialist G1 against "preserve current TUI behavior." That was wrong.
The user has decided the agent/bot/A2A behavior is being *removed* — it is not a preservation
constraint. Re-running G1 against the correct target condition inverts the verdicts: the
agent abstraction layer IS pass-through *given the new target* (it mediates a concept —
"agents" — that no longer exists in the system). G1 now PASSES (delete) for the agent layer.

## 1. Direction (kata-improvement Step 1, re-stated)

- **Challenge:** Delete the entire agent abstraction layer (AgentKind, AgentPod, PodDeployment
  as agent-container, AgentPersona/Charter/Identity, A2A agent registration, PodKind::Team)
  and replace the user-facing container with a single `UserPod` (Solid-Pod-modeled).
  The curator is extracted as a standalone systemd daemon.
- **Excellent performance:** No `Agent*` types, no `AgentKind` enum, no A2A agent registration,
  no `PodKind::Team`, no `hkask-agents` crate (renamed). One `UserPod` type per user.
  Curator runs as `kask curator` under systemd. All preserved TUI flows (register, login,
  terminal, skills, MCP) build and pass.
- **Measurement:** `metric_before` (§7) vs `metric_after`: AgentKind=0, AgentPod=0,
  PodKind variants 3→2 (Curator, UserPod), A2A agent-registration calls=0,
  `hkask-agents` crate=0 (renamed), TUI compile time non-increase, integration tests green.

## 2. Current Condition (read-only, file:line cited)

**Deletion targets (the agent layer):**
- `AgentKind` enum (`hkask-types/src/agent/mod.rs:8-13`) — 2 variants Bot/Replicant. **Delete.**
- `AgentPod` struct (`hkask-agents/src/pod/mod.rs:92-110`) — "Runtime container for A2A agents." **Delete → fold into UserPod.**
- `PodDeployment` (`hkask-agents/src/pod/deployment.rs:46-70`) — wraps AgentPod + storage/CNS/capability/inference. **Fold storage/CNS into UserPod; delete the agent-wrapper.**
- `PodKind::Team` (`pod/types.rs:39`) — no teams in new model. **Delete.**
- `PodKind::Replicant` (`pod/types.rs:42`) — rename → `PodKind::UserPod`.
- `AgentPersona`/`AgentCharter`/`AgentIdentity` (`pod/types.rs:111-150`) — agent abstractions. **Delete** (userpod has no "charter"; it has services).
- A2A agent registration (`hkask-agents/src/a2a/mod.rs:807` `register_agent`) — no agents to register. **Delete** (open Q-A2A: keep A2A *transport* for tool-to-tool MCP? lean delete).
- Agent lifecycle `Populated→Registered→Activated→Deactivated` (`pod/types.rs:57-66`) — agent A2A lifecycle. **Simplify** for userpods (open Q-LIFE).
- `hkask-agents` crate name — **rename** (open Q-NAME: `hkask-userpods` vs `hkask-services`).
- `ReplicantIdentity` (`hkask-identity/src/lib.rs:74-87`) — rename → `UserPod` identity; drop `is_primary` (open Q-1N: confirm 1:1).

**Survivors:**
- `HumanUser` (`hkask-identity/src/lib.rs:15-37`) — account auth. Keep.
- `WebID`, `CapabilityChecker`, OCAP tokens (`hkask-capability`) — user sovereignty. Keep.
- Per-pod SQLCipher storage (`deployment.rs:49` "No shared store") — Solid Pod model. Keep, move into UserPod.
- `CuratorSync` + `SemanticIndex` (`curator/semantic_sync.rs:297-362`) — the daemon's job. Keep.
- CNS, keystore, MCP servers, skills subsystem. Keep.
- TUI (`hkask-tui`) — keep; rewire to UserPod.

## 3. Target Condition (Step 3)

- **One measurable target (4–8 weeks):** zero `Agent*` types; one `UserPod` type;
  curator runs as `kask curator` systemd service; `cargo build --workspace` green;
  `agent_pod_integration.rs`/`pod_portability.rs` rewritten as `userpod_*` and green.
- **Focus obstacle (ONE):** "Collapsing `AgentPod`+`PodDeployment` into `UserPod` forces a
  single decision on whether the per-pod SQLCipher + CNS runtime lives *inside* UserPod
  (deep) or behind a port (hexagonal). Pick deep — UserPod IS the Solid Pod."
- **Knowledge gap:** I have NOT traced all `AgentPod`/`PodDeployment` consumers (open Q6).
  The collapse touches identity, agents, api/routes, cli, services-*, tui bridges. Blast
  radius is the whole graph — must slice vertically, not horizontally.

## 4. Adversarial Filter — essentialist G1→G2→G3 (re-run vs NEW target)

| ID | Change | G1 Exist (vs new target) | G2 Surface | G3 Contract | Verdict | Force |
|---|---|---|---|---|---|---|
| S-1 | Delete `AgentKind` enum | PASS — no agents exist; tag is pass-through to nothing | enum gone (0 items) | single-use tag → delete | **DELETE** | Guideline |
| S-2 | Delete `AgentPod`, fold into `UserPod` | PASS — "container for A2A agents" mediates a deleted concept | UserPod ≤7 pub items (TBD) | single-impl struct → inline | **DELETE** | Guideline |
| S-3 | Fold `PodDeployment` storage/CNS into `UserPod`; delete agent wrapper | PASS — wrapper adds resources around a deleted agent concept | merge into S-2 surface | pass-through wrapper | **DELETE** | Guideline |
| S-4 | Delete `PodKind::Team` | PASS — no teams in new model | n/a | n/a | **DELETE** | Guardrail (spec) |
| S-5 | Rename `PodKind::Replicant` → `PodKind::UserPod` | PASS — replicant IS the userpod (already) | n/a | rename, not behavior | **RENAME** | Evidence |
| S-6 | Delete `AgentPersona`/`AgentCharter`/`AgentIdentity` | PASS — agent YAML abstractions | n/a | n/a | **DELETE** | Guideline |
| S-7 | Delete A2A agent registration (keep transport? open) | PASS (agent reg) / UNVERIFIED (transport) | n/a | n/a | **DELETE reg; Q-A2A transport** | Hypothesis (transport) |
| S-8 | Simplify lifecycle `Pop→Reg→Act→Deact` for userpods | PASS — agent A2A lifecycle; userpods have deploy/teardown | ≤4 states | n/a | **SIMPLIFY** | Guideline |
| S-9 | Extract curator as `kask curator` systemd daemon | PASS — curator already a singleton (`active_pods.rs:315`) | daemon binary + unit file | n/a | **EXTRACT** | Guideline |
| S-10 | Rename `hkask-agents` → `hkask-userpods` (or `hkask-services`) | PASS — no agents | n/a | n/a | **RENAME** | Guideline |
| S-11 | Rename `ReplicantIdentity` → `UserPod`; drop `is_primary` (1:1) | PASS if 1:1 (open Q-1N) | n/a | n/a | **RENAME; Q-1N** | Hypothesis (1:1) |

**Essentialism score:** 11 changes, 9 PASS G1 (delete/rename/extract), 2 Hypothesis (S-7
transport, S-11 1:1). % removed from agent layer ≈ **~90%** of the agent abstraction surface.
Exceeds ≥26% target — *if* the user accepts each slice.

## 5. Adversarial Interrogation — grill-me on top 5 (S-2, S-3, S-4, S-9, S-11)

- **S-2 Recall:** `AgentPod` = runtime container for A2A agents (`pod/mod.rs:92`). **Mechanism:** created by `PodFactory::deploy` (`deployment.rs:239`), wrapped by `PodDeployment`. Used by A2A registration + context. **Rationale (Hypothesis):** model A2A agents per Solid Pod. **Edge:** deleting it forces UserPod to own lifecycle state + capability token. **Rollback:** cheap (git revert; one crate). ✅ survives (file:line cited).
- **S-3 Recall:** `PodDeployment` wraps `AgentPod` + storage + CNS + capability + inference (`deployment.rs:46-70`). **Mechanism:** built by `PodFactory::deploy`, returned to `ActivePods`. **Rationale (Hypothesis):** bundle per-pod resources. **Edge:** storage/CNS must move into UserPod (deep) or behind a port. **Rollback:** medium. ✅ survives.
- **S-4 Recall:** `PodKind::Team` (`pod/types.rs:39`), shared bot workspace. **Mechanism:** `deployment.rs:411` routes team→"team" filename. **Rationale (Hypothesis):** multi-bot shared episodic. **Edge:** no bots → no teams. **Rollback:** cheap. ✅ survives.
- **S-9 Recall:** curator is a singleton (`active_pods.rs:315` `ensure_curator`), runs `CuratorSync` (`semantic_sync.rs`). **Mechanism:** spawned via `tokio::spawn` inside the pod framework today; target = `kask curator` systemd service. **Rationale:** curator is the system daemon, not a userpod. **Edge:** systemd unit file + socket activation; loses in-process curator for tests. **Rollback:** medium. ✅ survives.
- **S-11 Recall:** `ReplicantIdentity` (`hkask-identity/src/lib.rs:74-87`), 1:N via `is_primary` + `list_replicants`. **Mechanism:** OAuth callback creates it (`auth.rs:363`); TUI switcher (`terminal.rs:263`). **Rationale:** was multi-persona. **Edge:** 1:1 loses the switcher — CONFIRM with user (Q-1N). **Rollback:** cheap. ⚠️ Hypothesis pending Q-1N — kept in plan but gated.

## 6. Phased vertical-slice task list (advisory; accept/reject per slice)

Bottom-up foundations first; high-risk (S-2/S-3 collapse) scheduled early. Each slice is a
complete end-to-end path, ≤5 files, ≤3 acceptance bullets, checkpoint every 2–3 slices.

### Phase 0 — Confirmations (blocking, no code)
- **T0.1** User confirms: 1:1 userpod-per-user (Q-1N)? A2A transport delete or keep for MCP tool-to-tool (Q-A2A)? crate rename target (Q-NAME)? lifecycle simplification shape (Q-LIFE)? — *Checkpoint: UserFeedbackOccurrence*

### Phase 1 — Foundation types (hkask-types + hkask-identity)
- **T1.1** Rename `ReplicantIdentity` → `UserPod`; drop `is_primary` (if Q-1N=1:1). Update `hkask-identity` + its tests. — *Acc: identity crate compiles, tests green; ≤3 files*
- **T1.2** Delete `AgentKind` enum; delete `PodKind::Team`; rename `PodKind::Replicant`→`PodKind::UserPod`. Update `hkask-types`. — *Acc: hkask-types compiles; consumers broken (expected); ≤2 files*

### Phase 2 — UserPod runtime (the focus obstacle, high-risk, early)
- **T2.1** Collapse `AgentPod`+`PodDeployment` → `UserPod` (deep: owns SQLCipher + CNS + capability). Trace ALL consumers first (Q6). — *Acc: UserPod deploys a user, `agent_pod_integration.rs` rewritten `userpod_integration.rs` green; ≤5 files; checkpoint: User*
- **T2.2** Delete `AgentPersona`/`AgentCharter`/`AgentIdentity`; userpod has services not charters. — *Acc: no `Agent*` persona types; persona YAML path removed; ≤4 files*

### Phase 3 — Curator daemon extraction
- **T3.1** Extract curator from pod framework → `kask curator` subcommand + systemd unit. `CuratorSync` polls `UserPod`s. — *Acc: `kask curator` runs; `CuratorSync` green against UserPod; ≤5 files; checkpoint: User*

### Phase 4 — A2A + lifecycle + rename
- **T4.1** Delete A2A agent registration (per Q-A2A: delete or repurpose transport). — *Acc: no agent registration; MCP tool path unaffected; ≤4 files*
- **T4.2** Simplify lifecycle `Pop→Reg→Act→Deact` → userpod deploy/teardown (per Q-LIFE). — *Acc: ≤3 states; tests green; ≤3 files*
- **T4.3** Rename `hkask-agents` → target (per Q-NAME). — *Acc: workspace builds; ≤2 files (Cargo.toml + lib.rs); checkpoint: User*

### Phase 5 — Surface rewiring + verification
- **T5.1** Rewire `hkask-api/routes/replicant.rs` → `routes/userpod.rs`; terminal.rs switcher removed if 1:1. — *Acc: API green; ≤4 files*
- **T5.2** Rewire `hkask-cli ReplicantAction` → `UserPodAction`. — *Acc: CLI green; ≤3 files*
- **T5.3** Rewire `hkask-tui ReplicaDataBridge` → `UserPodDataBridge`. — *Acc: TUI builds + smoke; ≤4 files; checkpoint: User*
- **T5.4** Full verification: `cargo build --workspace`; `cargo test -p hkask-userpods`; TUI flows. — *Acc: green; metric_after recorded*

## 7. metric_before (JSON, read-only) and target metric_after

```json
{
  "metric_before": {
    "AgentKind_variants": 2,
    "AgentPod_struct": 1,
    "PodDeployment_wrapper": 1,
    "PodKind_variants": 3,
    "AgentPersona_Charter_Identity_types": 3,
    "a2a_register_agent_callsites": ">=1 (a2a/mod.rs:807)",
    "hkask_agents_crate": 1,
    "ReplicantIdentity_is_primary_1N": true,
    "replicant_typed_items_total": 5,
    "tui_compile_time_seconds": "Hypothesis — measure in T5.4",
    "integration_tests_referencing_replicant": 3
  },
  "target_metric_after": {
    "AgentKind_variants": 0,
    "AgentPod_struct": 0,
    "PodDeployment_wrapper": 0,
    "PodKind_variants": 2,
    "AgentPersona_Charter_Identity_types": 0,
    "a2a_register_agent_callsites": 0,
    "hkask_agents_crate": 0,
    "ReplicantIdentity_is_primary_1N": "false (1:1, if Q-1N confirmed)",
    "replicant_typed_items_total": "0 (renamed to userpod; curator keeps 'replicant' name only as daemon label)",
    "tui_compile_time_seconds": "<= metric_before",
    "integration_tests_referencing_replicant": "0 (renamed userpod_*)"
  }
}
```

## 8. Iteration Engine (gpa-evolution) — 2 iterations, Pareto

(quality = agent-layer surface removed with no TUI regression; cost = files touched + test churn)

- **Iter 1 "big-bang delete AgentKind":** delete enum first, fix all consumers. quality=high, cost=very high (touches every consumer at once). Violates vertical-slice rule. **Dominated** on cost.
- **Iter 2 "vertical slices, curator-first":** T3 (extract curator) before T2 (collapse UserPod). quality=same, cost=lower (curator extraction isolates the daemon first, shrinking T2 blast radius). **Non-dominated.**
- **Iter 3 "vertical slices, types-first":** T1 (types) before T3. quality=same, cost=lower still (foundation types first lets T2/T3 compile against new names). **Non-dominated, dominates iter 2.**
- Mutations tested: M1 big-bang (cost FAIL), M2 curator-first (cost win), M3 types-first (cost win + natural bottom-up).
- **Frontier:** { types-first vertical slices (iter 3) } size 1. hypervolume delta iter2→iter3 ≤ 0.10. **Converged at iteration 2–3.** Plan §6 reflects iter 3 ordering.

## 9. Risks

- R1: Q-1N (1:1 vs 1:N) — if user wants 1:N preserved, T1.1 keeps `is_primary` and the switcher; userpod ≠ single-per-user. **Guardrail**: gate T1.1 on T0.1.
- R2: Q-A2A transport — MCP tool-to-tool may need *some* transport; deleting all A2A could break tool composition. **Hypothesis**: gate T4.1 on T0.1.
- R3: S-2/S-3 collapse is the focus obstacle — high blast radius across identity/agents/api/cli/tui. **Mitigation**: T2.1 traces consumers first (Q6), ≤5 files per slice.
- R4: Curator systemd extraction (T3.1) loses in-process curator for integration tests. **Mitigation**: keep a test harness mode.

## 10. Open questions (knowledge threshold)

- Q-1N: 1 userpod per user, or preserve 1:N multi-persona? (user said "each will get their userpod" → lean 1:1, confirm)
- Q-A2A: delete A2A entirely, or keep transport for MCP tool-to-tool coordination?
- Q-NAME: `hkask-userpods` vs `hkask-services` vs other?
- Q-LIFE: userpod lifecycle shape (deploy/teardown only? or keep activated/deactivated for session gating?)
- Q5: S4/S5 VSM policy components for the userpod subgraph (unlocated).
- Q6: ALL consumers of `AgentPod`/`PodDeployment` (trace repo-wide before T2.1).
- Q7: remaining 9 lines of `resolve_replicant_name` (`services-skill/src/skill_impl.rs:302+`) — does it become `resolve_userpod_name`?

## 11. DC+BIBO metadata

- **Direction:** delete agent layer; userpod = Solid Pod per user; curator = systemd daemon; tools = skills+MCP.
- **Capability:** graph consolidation (removal, not refactor).
- **Boundary:** advisory; no autonomous deletion; accept/reject per slice.
- **Input:** user re-scope + read-only codebase (file:line cited).
- **Output:** this plan + todo + §12 table + metric JSON + Pareto.
- **Author:** Zed agent (advisory). **Provenance:** claims cited or labeled Hypothesis.

## 12. Top-5 changes (G1+G2+G3 + grill-me survivors)

| # | file:line | Force | Layers removed | Behavior preserved | Rollback cost |
|---|---|---|---|---|---|
| S-2 | `pod/mod.rs:92-110`, `deployment.rs:239` | Guideline (DELETE) | 1 (AgentPod) | userpod owns lifecycle+capability | medium (1 crate) |
| S-3 | `deployment.rs:46-70` | Guideline (DELETE/FOLD) | 1 (PodDeployment wrapper) | storage/CNS move into UserPod | medium |
| S-4 | `pod/types.rs:39`, `deployment.rs:411` | Guardrail (DELETE) | 1 (PodKind::Team) | no teams in new model | cheap |
| S-9 | `active_pods.rs:315`, `semantic_sync.rs:297` | Guideline (EXTRACT) | 1 (curator out of pod framework) | curator = systemd daemon | medium |
| S-11 | `hkask-identity/src/lib.rs:74-87`, `terminal.rs:263` | Hypothesis (RENAME; gated Q-1N) | 1 (ReplicantIdentity→UserPod) | 1:1 if Q-1N confirmed; switcher removed | cheap |