# Plan — Replicant→UserPod Consolidation (FINAL, Phase 0 resolved)

> **Status:** ADVISORY. No code deleted. Each slice flagged for user accept/reject.
> **Phase 0 — RESOLVED** (user-confirmed):
> 1. **1:1** — one userpod per user; multi-persona removed (`is_primary`, `list_replicants`, switcher deleted).
> 2. **A2A transport KEPT** — userpods present as agents (generic "agent" concept preserved); hKask-specific replicant/bot distinction goes.
> 3. **No crate rename** — `hkask-agents` stays (generically accurate; users act as agents via pod).
> 4. **Persistent userpods** — not spin-up/spin-down; they hold data/preferences/history/artifacts. Lifecycle collapses. Curator = systemd daemon coordinating userpods (k8s arch: `commands/deploy.rs`, Hetzner K3s + Conduit).
>
> **Phase 0b — RESOLVED** (user-confirmed):
> 5. **Delete `AgentKind` entirely** — no place in the new system. A2A registration takes no kind param.
> 6. **Drop persona from userpods (lean)** — the ONLY entity with a persona is the curator. `AgentPersona`/`Charter`/`Identity` deleted from userpods; curator retains a persona (rename → `CuratorPersona`-style).
> 7. **Principles: same twelve, refocused** — P10 not retired; refocused from "bot/replicant taxonomy" to **user agency & user sovereignty**. Same principles, different focus entity.
> 8. **CNS span field** `replicant_host` → **`userpod_host`**.
> 9. **Pod-when-user-offline: UNDEFINED** — the pod persists as long as the account exists. What it does when the user is logged-out or the account is inactive-but-not-cancelled is currently undefined and needs a design decision (see Q-LIFE-DISC discovery task, gates T4.2).

---

## 0. Architecture reframe (confirmed)

- hKask = minimal viable container for AI tools. Actors: **human users** (each → 1 persistent
  userpod, Solid-Pod-modeled), **AI tools** (skills + MCP servers), **curator** (systemd daemon;
  the one surviving "replicant" by name). No bots, no replicant/bot taxonomy.
- Userpods **present as agents** in A2A — the generic agent abstraction stays; the hKask-specific
  `AgentKind::Bot`/`Replicant` distinction goes.
- Principles editing is **in scope** (user: "this includes editing the principles and a lot of
  other things"). P6, P10, P12 and the 5W1H "Who" line all name replicant/bot and must change.

## 1. Direction (Step 1)

- **Challenge:** Delete the agent/replicant/bot abstraction layer; replace with persistent
  `UserPod` (1:1 per user, Solid-Pod-modeled); keep A2A transport with userpods-as-agents;
  extract curator as systemd daemon (already partially done — code calls it "daemon", `init.rs:180`
  generates `hkask.service`); edit principles P5.2/P6/P6.1/P9/P10/P12 + mandate doc.
- **Excellent performance:** zero `AgentKind` variants, zero `PodKind::Team`, zero
  `AgentPersona/Charter/Identity`, zero `is_primary`/`list_replicants`/switcher, P10 retired,
  P12 retitled; persistent userpods; curator = systemd daemon; A2A transport intact; all
  preserved TUI flows (register, login, terminal, skills, MCP) green.
- **Measurement:** `metric_before` (§7) → `metric_after`: AgentKind=0, PodKind=3→2, Team=0,
  Persona-types=0, is_primary=false, P10 retired, TUI compile non-increase, tests green.

## 2. Current Condition (file:line cited)

**Deletion targets:**
- `AgentKind` enum (`hkask-types/src/agent/mod.rs:8-13`) — Bot/Replicant taxonomy = P10. **Delete** (open Q-AK: delete enum entirely vs single `UserPod` variant; lean delete — G3 de-genericize single-variant enum).
- `AgentPod` (`pod/mod.rs:92-110`) — "Runtime container for A2A agents." **Fold into UserPod.**
- `PodDeployment` (`deployment.rs:46-70`) — wrapper. **Fold storage/CNS into UserPod.**
- `PodKind::Team` (`pod/types.rs:39`). **Delete.** `PodKind::Replicant` → `PodKind::UserPod`.
- `AgentPersona`/`AgentCharter`/`AgentIdentity` (`pod/types.rs:111-150`) — **deleted from userpods** (lean; user confirmed the only entity with a persona is the curator). Curator retains a persona → rename `AgentPersona`→`CuratorPersona` (or curator-specific type).
- `ReplicantIdentity.is_primary` + `list_replicants` API (`replicant.rs:63`) + TUI switcher (`terminal.rs:263`). **Delete** (1:1).
- `AgentKind::Bot` A2A registration path (`a2a/mod.rs:807`). **Delete.**
- Lifecycle `Populated→Registered→Activated→Deactivated` (`pod/types.rs:57-66`). **Collapse** — persistent pods; A2A register-on-start; remove `Deactivated`/teardown.
- **P10 — Bot/Replicant Taxonomy** (`PRINCIPLES.md:206-208`). **REFOCUS** (not retire) — Twelve Principles stay twelve; P10 retitled to **user agency & user sovereignty** (same principle role, different focus entity, per user).

**Survivors:**
- `HumanUser` (`hkask-identity/src/lib.rs:15-37`). `hkask-agents` crate name. `hkask-tui`.
- A2A transport (`a2a/mod.rs`) — kept; userpods + curator register as agents.
- Per-pod SQLCipher + CNS + capability/WebID/OCAP (user sovereignty).
- `CuratorSync`/`SemanticIndex` (`semantic_sync.rs:297-362`) — daemon's job.
- `PodContext.curator_index` (`context.rs:63`) — userpod↔curator coordination channel (the k8s-arch coordination you referenced).
- Curator daemon infra (already: `init.rs:180` systemd unit `kask serve`; `deploy.rs` k8s; code comments say "Curator daemon" at `curation_loop.rs:48`, `wallet_manager.rs:3`, `status.rs:88`).

**Principles surface to edit** (file:line):
- P5.2 5W1H "Who" (`PRINCIPLES.md:83`): "agent, replicant, bot, human…" → drop replicant/bot.
- P6 title "Space for Replicants & Bots" (`PRINCIPLES.md:128`) → "Space for UserPods"; body "container for bot and replicant agency" → "container for human users (via userpods) + AI tools".
- P6.1 Per-Pod Deployment (`PRINCIPLES.md:131`): "Each human+replicant pair" → "Each user (1:1)".
- P9 Authority (`PRINCIPLES.md:200`): "replicant or owner WebID" → "userpod or owner WebID".
- P10 Bot/Replicant Taxonomy (`PRINCIPLES.md:206-208`): **retire** (Q-P10).
- P12 Replicant Host Mandate (`PRINCIPLES.md:214-230`): retitle → "Authenticated Host Mandate" (or "UserPod Host Mandate"); rewrite P12.1 surface-host table — drop the "API | 7R7 bots" row; CLI/REPL = "Human user (via userpod) + Curator daemon"; Daemon = Curator; API = Userpods.
- Mandate doc `docs/architecture/mandates/P12-replicant-host-mandate.md` — rewrite.
- Code comments: `deployment.rs:11` (P6 Space for Replicants), `openapi.rs:59` (P12), `identity/lib.rs:222` (P6), `ports/registry.rs:113` ("only human replicants do" → "only human users via userpod do"), `mcp/runtime.rs:246`, `test-harness/lib.rs:17`, `tui/windows/chat.rs:16`.
- Skill docs: `attack-taxonomy-mapper`, `runtime-posture-monitor`, `supply-chain-sentinel` (P12 "replicant_host mandatory" span field → rename to `userpod_host` or `host`).
- `FUNCTIONAL_SPECIFICATION.md` P12 references.

## 3. Target Condition (Step 3)

- **One measurable target (4–8 weeks):** zero `Agent*` persona types on userpods (curator keeps `CuratorPersona`), zero `AgentKind` variants, `PodKind` 3→2 (Curator, UserPod), `is_primary`=gone, P10 refocused to user agency, P12 retitled, persistent userpods, curator as systemd daemon, `cargo build --workspace` green, `userpod_*` tests green. Pod-offline behavior defined via Q-LIFE-DISC.
- **Focus obstacle (ONE):** "Collapsing `AgentPod`+`PodDeployment` into a persistent `UserPod`
  forces the decision: per-pod SQLCipher + CNS live INSIDE UserPod (deep, Solid-Pod-isomorphic)
  vs behind a port. Pick deep — UserPod IS the Solid Pod. Persistent (no Deactivated state)
  changes the lifecycle state machine that `PodLifecycleState::can_transition_to` enforces."
- **Knowledge gap:** consumer trace of `AgentPod`/`PodDeployment` (Q6) not done.

## 4. Adversarial Filter — essentialist G1→G2→G3 (vs confirmed target)

| ID | Change | G1 | G2 | G3 | Verdict | Force |
|---|---|---|---|---|---|---|
| S-1 | Delete `AgentKind` (single-variant after Bot gone → de-genericize) | PASS | enum→0 | single-variant enum | **DELETE** (Q-AK) | Guideline |
| S-2 | Fold `AgentPod`→`UserPod` | PASS | ≤7 pub items | single-impl struct | **FOLD** | Guideline |
| S-3 | Fold `PodDeployment` storage/CNS into `UserPod` | PASS | merge into S-2 | pass-through wrapper | **FOLD** | Guideline |
| S-4 | Delete `PodKind::Team` | PASS | n/a | n/a | **DELETE** | Guardrail (spec) |
| S-5 | Rename `PodKind::Replicant`→`UserPod` | PASS | n/a | rename | **RENAME** | Evidence |
| S-6 | Delete persona from userpods; curator retains (rename→`CuratorPersona`) | PASS — persona mediates a deleted userpod concept | n/a | n/a | **DELETE from userpods; RENAME for curator** | Guardrail (spec: curator-only persona) |
| S-7 | A2A: keep transport; userpods+curator register as agents; drop Bot path | PASS (Bot path) | n/a | n/a | **KEEP transport; DELETE Bot reg** | Evidence |
| S-8 | Collapse lifecycle → persistent + A2A-register-on-start | PASS | ≤2 states | n/a | **SIMPLIFY** | Guardrail (spec: persistent) |
| S-9 | Curator as systemd daemon (first-class, not a PodKind) | PASS — already "daemon" in code + `init.rs:180` unit | n/a | n/a | **EXTRACT/promote** | Evidence |
| S-11 | Rename `ReplicantIdentity`→`UserPod`; drop `is_primary`/switcher (1:1) | PASS | n/a | rename | **RENAME** | Guardrail (spec: 1:1) |
| S-12 | Refocus P10 to user agency (not retire); edit P5.2/P6/P6.1/P9/P12 + mandate doc + comments + skills | PASS — naming a deleted focus entity | n/a | n/a | **EDIT/REFOCUS** | Guardrail (spec: principles in scope) |

**Essentialism:** 11 changes, all PASS G1. ~90% of agent-layer surface removed + principles aligned. Exceeds ≥26%.

## 5. Adversarial Interrogation — grill-me top 5 (S-2, S-3, S-8, S-9, S-12)

- **S-2 Recall:** `AgentPod` runtime container (`pod/mod.rs:92`). **Mechanism:** `PodFactory::deploy` builds it (`deployment.rs:239`), `PodDeployment` wraps it. **Rationale (Hypothesis):** model A2A agents per Solid Pod. **Edge:** folding forces UserPod to own lifecycle+capability; A2A registration now targets UserPod directly. **Rollback:** medium (1 crate). ✅
- **S-3 Recall:** `PodDeployment` bundles storage+CNS+capability+inference (`deployment.rs:46-70`). **Mechanism:** returned by `PodFactory::deploy`, held by `ActivePods`. **Rationale (Hypothesis):** per-pod resource bundling. **Edge:** SQLCipher+CNS must move INTO UserPod (deep) — Solid Pod isomorphism. **Rollback:** medium. ✅
- **S-8 Recall:** `PodLifecycleState` Pop→Reg→Act→Deact (`pod/types.rs:57`), enforced by `can_transition_to`. **Mechanism:** A2A registration gates `Registered`, activation gates `Activated`. **Rationale (Hypothesis):** agent A2A lifecycle. **Edge:** persistent pods → no Deactivated; register-on-start only. **Rollback:** medium. ✅
- **S-9 Recall:** curator is singleton (`active_pods.rs:315` `ensure_curator`); code already says "Curator daemon" (`curation_loop.rs:48`, `status.rs:88`); systemd unit exists (`init.rs:180` `kask serve`). **Mechanism:** today curator is a `PodKind::Curator` inside the pod framework. **Rationale:** system daemon. **Edge:** promote curator OUT of PodKind to a first-class daemon; `CuratorSync` polls UserPods. **Rollback:** medium. ✅
- **S-12 Recall:** P6/P10/P12 name replicant/bot (`PRINCIPLES.md:128,206,214`). **Mechanism:** principles cited as goal/constraining in code comments + FUNCTIONAL_SPECIFICATION. **Rationale (Evidence):** user explicitly said edit principles. **Edge:** retiring P10 reduces Twelve→Eleven (Q-P10: retire vs replace). Span field `replicant_host` rename affects skill YAML. **Rollback:** cheap (docs). ✅

## 6. Phased vertical-slice task list (advisory; accept/reject per slice)

Bottom-up; focus obstacle (S-2/S-3) early; principles editing as its own phase. ≤5 files, ≤3 acceptance bullets, checkpoint every 2–3 slices.

### Phase 1 — Foundation types
- **T1.1** Rename `ReplicantIdentity`→`UserPod`; drop `is_primary`; rename fields `replicant_name`→`userpod_name`, `replicant_webid`→`webid`. Update `hkask-identity` + tests. — *Acc: identity compiles, tests green; ≤3 files; checkpoint: User*
- **T1.2** Delete `AgentKind` enum (Q-AK: delete vs single-variant); delete `PodKind::Team`; rename `PodKind::Replicant`→`PodKind::UserPod`. Update `hkask-types`. — *Acc: hkask-types compiles (consumers broken, expected); ≤2 files*

### Phase 2 — UserPod runtime (focus obstacle, early)
- **T2.0** Trace ALL `AgentPod`/`PodDeployment` consumers repo-wide (Q6) → caller table. — *Acc: table file:line; checkpoint: User*
- **T2.1** Fold `AgentPod`+`PodDeployment`→`UserPod` (deep: owns SQLCipher+CNS+capability); persistent (no Deactivated). Rewrite `agent_pod_integration.rs`→`userpod_integration.rs`, `pod_portability.rs`. — *Acc: UserPod deploys+persists; tests green; ≤5 files; checkpoint: User*
- **T2.2** Delete persona from userpods; rename `AgentPersona`→`CuratorPersona` (curator-only). Remove persona-YAML path from userpod creation; userpod presents in A2A via WebID+name+capabilities only. — *Acc: no persona types on userpods; curator persona intact; ≤4 files*

### Phase 3 — Curator daemon promotion
- **T3.1** Promote curator OUT of `PodKind::Curator` to first-class systemd daemon (`kask serve` already generates unit at `init.rs:180`); `CuratorSync` polls UserPods via `curator_index` channel. Keep test harness. — *Acc: `kask serve` runs curator; CuratorSync green vs UserPod; ≤5 files; checkpoint: User*

### Phase 4 — A2A + lifecycle
- **T4.0 (DISCOVERY, gates T4.2)** Define pod-offline behavior (Q-LIFE-DISC): produce a 1-page design doc with options — (a) pod sleeps: storage-at-rest, no compute, no A2A reachability; (b) pod runs headless: A2A-reachable, no inference; (c) maintenance mode for inactive-not-cancelled accounts. User picks one. — *Acc: design doc with chosen option; checkpoint: UserFeedbackOccurrence*
- **T4.1** A2A: keep transport; delete Bot registration path; userpods + curator register as agents (no `AgentKind`). — *Acc: no Bot path; userpod+curator A2A register; MCP tools unaffected; ≤4 files*
- **T4.2** Collapse `PodLifecycleState` → persistent + register-on-start, per T4.0 decision (≤2–3 states incl. any offline state). Remove `Deactivated`/teardown. — *Acc: states match T4.0 design; tests green; ≤3 files; checkpoint: User*

### Phase 5 — Principles + docs (NEW, user-explicitly-in-scope)
- **T5.1** Edit P6 "Space for Replicants & Bots"→"Space for UserPods" + P6.1 1:1 (`PRINCIPLES.md:128-131`); P5.2 "Who" drop replicant/bot (`:83`); P9 authority `:200`. — *Acc: PRINCIPLES.md consistent; ≤1 file*
- **T5.2** Refocus P10 Bot/Replicant Taxonomy → **P10 User Agency** (`PRINCIPLES.md:206-208`); Twelve Principles stay twelve. — *Acc: P10 retitled+refocused to user agency/sovereignty; ≤1 file; checkpoint: User*
- **T5.3** Retitle P12 → "Authenticated Host Mandate"; rewrite P12.1 surface-host table (drop Bot row; CLI=user+curator, Daemon=curator, API=userpods) (`PRINCIPLES.md:214-230`); rewrite `mandates/P12-replicant-host-mandate.md`. — *Acc: P12 consistent; ≤2 files*
- **T5.4** Sweep code comments: `deployment.rs:11`, `openapi.rs:59`, `identity/lib.rs:222`, `ports/registry.rs:113`, `mcp/runtime.rs:246`, `test-harness/lib.rs:17`, `tui/windows/chat.rs:16`, `FUNCTIONAL_SPECIFICATION.md`. — *Acc: no "replicant/bot" in principle-cited comments; ≤5 files*
- **T5.5** Sweep skill docs: `attack-taxonomy-mapper`, `runtime-posture-monitor`, `supply-chain-sentinel` — rename `replicant_host` span field → `userpod_host` (or `host`) per Q-SPAN. — *Acc: skills consistent; ≤3 files; checkpoint: User*

### Phase 6 — Surface rewiring + verification
- **T6.1** `hkask-api/routes/replicant.rs`→`routes/userpod.rs`; remove `list_replicants` + terminal switcher. — *Acc: API green; ≤4 files*
- **T6.2** `hkask-cli ReplicantAction`→`UserPodAction`. — *Acc: CLI green; ≤3 files*
- **T6.3** `hkask-tui ReplicaDataBridge`→`UserPodDataBridge`. — *Acc: TUI builds + smoke; ≤4 files; checkpoint: User*
- **T6.4** Full verify: `cargo build --workspace`; `cargo test`; TUI flows; record `metric_after`. — *Acc: green; metric_after JSON*

## 7. metric_before / target metric_after (JSON)

```json
{
  "metric_before": {
    "AgentKind_variants": 2,
    "AgentPod_struct": 1,
    "PodDeployment_wrapper": 1,
    "PodKind_variants": 3,
    "AgentPersona_Charter_Identity_types": 3,
    "is_primary_1N": true,
    "list_replicants_api": 1,
    "tui_replicant_switcher": 1,
    "Bot_A2A_registration_path": true,
    "PodLifecycleState_variants": 4,
    "P10_BotReplicantTaxonomy": "active",
    "P12_title": "Replicant Host Mandate",
    "principle_coded_comments_naming_replicant_bot": ">=10",
    "hkask_agents_crate": 1,
    "tui_compile_time_seconds": "Hypothesis — measure T6.4"
  },
  "target_metric_after": {
    "AgentKind_variants": 0,
    "AgentPod_struct": 0,
    "PodDeployment_wrapper": 0,
    "PodKind_variants": 2,
    "AgentPersona_Charter_Identity_types": 0,
    "is_primary_1N": false,
    "list_replicants_api": 0,
    "tui_replicant_switcher": 0,
    "Bot_A2A_registration_path": false,
    "PodLifecycleState_variants": "<=2",
    "P10_BotReplicantTaxonomy": "refocused to User Agency (Twelve Principles stay twelve)",
    "P12_title": "Authenticated Host Mandate",
    "principle_coded_comments_naming_replicant_bot": 0,
    "hkask_agents_crate": 1,
    "tui_compile_time_seconds": "<= metric_before",
    "A2A_transport": "kept (userpods+curator present as agents)"
  }
}
```

## 8. Iteration Engine (gpa-evolution) — 3 iterations, Pareto

(quality = agent-layer surface removed with no TUI regression; cost = files touched + test churn)
- **Iter 1 big-bang delete AgentKind first:** dominated on cost (touches all consumers at once, violates vertical-slice).
- **Iter 2 curator-first:** non-dominated, lower cost.
- **Iter 3 types-first + principles-as-own-phase:** dominates iter 2 — foundation types let later slices compile against new names; principles editing isolated in Phase 5 avoids interleaving doc/code churn. **Non-dominated.**
- Mutations: M1 big-bang (cost FAIL), M2 curator-first (cost win), M3 types-first + principles-phase (cost win + isolation win), M4 fold-lifecycle-into-T2.1 (rejected — lifecycle collapse is a separate verifiable slice).
- **Frontier:** { types-first + principles-phase (iter 3) } size 1. hypervolume delta ≤ 0.10. **Converged iter 3.** Plan §6 = iter 3 ordering.

## 9. Risks

- R1: Q-AK (delete AgentKind vs single-variant) — affects A2A registration signature. **Guardrail**: gate T1.2/T4.1.
- R2: Q-PERSONA — userpods present as agents in A2A; some persona fields may survive. **Guideline**: T2.2 keeps A2A presentation, drops charter/identity YAML.
- R3: S-2/S-3 fold is high blast radius. **Mitigation**: T2.0 caller trace; ≤5 files/slice.
- R4: P10 refocus (not retire) — Twelve Principles stay twelve; only the focus entity changes. Lower risk than retirement. **Guardrail**: T5.2 user-reviewed.
- R5: `replicant_host` span field rename in skills affects CNS span consumers. **Mitigation**: T5.5 isolated; check `CANONICAL_NAMESPACES` CI gate (`scripts/check-cns-canonical.sh`).
- R6: Curator promotion out of PodKind may break `pod_portability.rs` expectations. **Mitigation**: T3.1 keeps test harness mode.

## 10. Open questions (remaining after Phase 0b)

- Q-AK: **RESOLVED** — delete `AgentKind` entirely.
- Q-PERSONA: **RESOLVED** — drop persona from userpods; curator keeps persona (rename `CuratorPersona`).
- Q-P10: **RESOLVED** — refocus P10 to user agency (Twelve Principles stay twelve).
- Q-SPAN: **RESOLVED** — `replicant_host`→`userpod_host`.
- Q-LIFE-DISC: **OPEN (design discovery, T4.0)** — what does a persistent pod do when the user is logged-out, or the account is inactive-but-not-cancelled? Undefined; produce options for user decision.
- Q6: full consumer trace of `AgentPod`/`PodDeployment` (T2.0).

## 11. DC+BIBO metadata

- **Direction:** delete agent/replicant/bot layer; persistent 1:1 UserPod; A2A transport kept (userpods-as-agents); curator = systemd daemon; principles edited.
- **Capability:** graph consolidation + principle realignment.
- **Boundary:** advisory; accept/reject per slice; no autonomous deletion.
- **Input:** user Phase 0 confirmations + read-only codebase (file:line cited).
- **Output:** this plan + todo + §12 table + metric JSON + Pareto.
- **Author:** Zed agent (advisory). **Provenance:** claims cited or labeled Hypothesis.

## 12. Top-5 changes (G1+G2+G3 + grill-me survivors)

| # | file:line | Force | Layers removed | Behavior preserved | Rollback |
|---|---|---|---|---|---|
| S-2 | `pod/mod.rs:92-110`, `deployment.rs:239` | Guideline (FOLD) | 1 (AgentPod→UserPod) | userpod owns lifecycle+capability; A2A registers userpod | medium |
| S-3 | `deployment.rs:46-70` | Guideline (FOLD) | 1 (PodDeployment wrapper) | SQLCipher+CNS move INTO UserPod (deep) | medium |
| S-8 | `pod/types.rs:57-66` | Guardrail (SIMPLIFY) | 1 (lifecycle 4→≤3 per T4.0) | persistent pods; register-on-start; offline state per T4.0 design | medium |
| S-9 | `active_pods.rs:315`, `init.rs:180`, `deploy.rs` | Evidence (EXTRACT/promote) | 1 (curator out of PodKind) | curator = systemd daemon; CuratorSync polls userpods | medium |
| S-12 | `PRINCIPLES.md:128,206,214`, mandate doc | Guardrail (EDIT/REFOCUS) | 1 (P10 refocused to user agency; P6/P12 retitled) | principles focus on user agency/sovereignty, not replicant/bot | cheap |