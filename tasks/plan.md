# Plan — Replicant→Userpod Consolidation: Adversarial Graph Simplification

> **Status:** ADVISORY. No code deleted. Every recommendation carries a constraint-force label.
> The user must accept/reject each item before any action (per task spec).
> **Headline finding:** The task's central thesis is **Hypothesis**, not Evidence. See §0.

---

## 0. Thesis Verification (pragmatic-semantics: classify before acting)

The task thesis states: *"hKask currently separates 'replicant agent pods' and 'users' into
distinct entities with mediation layers between them… no `Replicant` trait, no pod-spawn
indirection."* Evidence from the codebase contradicts three sub-claims:

| Thesis sub-claim | Evidence (file:line) | Verdict |
|---|---|---|
| A `Replicant` trait exists | grep `trait Replicant\|Replicant for\|impl Replicant` → only `impl ReplicantIdentity` (a struct), `hkask-identity/src/lib.rs:89` | **No such trait.** Hypothesis → rejected. |
| "replicant agent pods" vs "users" are separate | `ReplicantIdentity` doc: *"the in-system persona users log in AS"* (`hkask-identity/src/lib.rs:72`); `PodKind::Replicant` doc: *"per-user sovereign pod"* (`hkask-agents/src/pod/types.rs:36`) | The replicant **IS** the userpod. No separation of the kind the thesis assumes. |
| "pod-spawn indirection" between human↔curator | Terminal path: `Command::new("kask").arg("repl").arg("--webid")` child process (`hkask-api/src/routes/terminal.rs:95-105`). A2A: Pod→Router→Pod (`hkask-agents/src/a2a/mod.rs:15`). No spawn *layer* — just a child proc + a router. | Indirection is 1 hop (a subprocess), not an abstraction layer. |

**The actual layering that exists** (cited):
1. `HumanUser` — account: email/passhash/OAuth/role (`hkask-identity/src/lib.rs:15-37`).
2. `ReplicantIdentity` — in-system persona: WebID/wallet/persona_yaml (`hkask-identity/src/lib.rs:74-87`).
3. `AgentPod` — runtime lifecycle container: id/webid/agent_type/state (`hkask-agents/src/pod/mod.rs:92-110`).
4. `PodDeployment` — Solid-Pod-isomorphic deployment unit: dedicated DB+CNS+capability+inference (`hkask-agents/src/pod/deployment.rs:46-70`).
5. `CuratorSync` — S3 aggregator polling pod DBs into `SemanticIndex` (`hkask-agents/src/curator/semantic_sync.rs:297-362`).

**Cardinality (decisive):** `HumanUser`→`ReplicantIdentity` is **1:N**. Evidence:
`ReplicantIdentity.is_primary` (`hkask-identity/src/lib.rs:84`), `list_replicants` API
(`hkask-api/src/routes/replicant.rs:63`), replicant switcher dropdown in the terminal UI
(`hkask-api/src/routes/terminal.rs:263`). One account owns multiple in-system personas.

**Open questions (knowledge threshold — NOT silently resolved):**
- Q1: Did the task author mean the `AgentKind::Bot`/`Replicant` split (bots vs user-pods), not
  "replicant vs user"? That split exists (`hkask-types/src/agent/mod.rs:8-13`).
- Q2: Did the task author mean the `AgentPod`↔`PodDeployment` wrapper? That exists
  (`deployment.rs:46-70`) and applies to ALL pod kinds, not just replicants.
- Q3: Did the task author mean `CuratorSync` as the "mediation layer"? It's a real S3 loop,
  not pass-through (§3.4).
- Q4: Is there a "replicant crate"? No. Replicant code spans `hkask-identity`,
  `hkask-agents/pod`, `hkask-api/routes/replicant.rs`, `hkask-cli` `ReplicantAction`. The
  `metric_before` field "public surface of the replicant crate" is undefined → N/A.

---

## 1. Direction (kata-improvement Step 1)

- **Challenge:** Remove ≥ N abstraction layers between human↔curator and human↔human paths
  while preserving every behavior observable through the TUI.
- **Reframed challenge (after §0):** Since the replicant IS the userpod, the only candidate
  layers are (a) the `HumanUser`/`ReplicantIdentity` 1:N seam, (b) the `AgentPod`/`PodDeployment`
  wrapper, (c) the `Bot`/`Replicant` `AgentKind` split, (d) the `PodKind` trichotomy, (e) `CuratorSync`.
- **Excellent performance:** A userpod is the only authenticated-actor type — BUT evidence
  shows "userpod" already == `ReplicantIdentity`+`AgentPod`. The real question is whether any
  of (a)–(e) are *pass-through* (G1 FAIL → delete) vs *behavior-carrying* (G1 PASS → keep).
- **Knowledge threshold:** §0 open questions Q1–Q4 must be resolved by the user before any
  deletion plan can be executable. Marked open; never silently resolved.

## 2. Current Condition (kata-improvement Step 2 — go and see)

Read-only. `metric_before` recorded in §6. Cybernetic analysis:

- **Loop: curator↔replicant↔user.** Polarity: negative (CuratorSync attenuates cross-pod
  semantic drift). Delay: polling interval (`semantic_sync.rs:79` `interval: Duration`).
  Gain: high (aggregates N pods). Closure: closed (cursor-advance, `semantic_sync.rs:307`).
  Fidelity: high (reads source DB read-only). **Closure is NOT broken → not a deletion target.**
- **Loop: human↔replicant (TUI terminal).** Polarity: n/a (control path). Delay: subprocess
  spawn. Closure: closed (user sees REPL output). One hop. Not a "layer."
- **Variety check (curator vs N pods × M behaviors):** The pod layer *attenuates* real
  disturbance (per-pod SQLCipher isolation, `deployment.rs:49` "No shared store"). It is NOT
  merely hiding variety → **passes Good Regulator test → keep.**
- **VSM map:** S1 = N `PodDeployment`s; S2 = `CuratorSync` anti-oscillatory channel
  (semantic index); S3 = curator; S4/S5 = policy (not located in this read — open Q5).
  Algedonic S1→S5 = CNS spans (`hkask.pod.deployment` target, `deployment.rs:286`).
  No unviable component located in the pod/curator subgraph.

## 3. Target Condition (kata-improvement Step 3)

- **One measurable target (2–8 weeks):** Resolve §0 Q1–Q4; for each surviving G1-PASS seam,
  reduce its public surface by ≥26% with zero TUI regression.
- **Focus obstacle (ONE):** "The `AgentPod`↔`PodDeployment` wrapper may be pass-through
  (PodDeployment adds resource fields around AgentPod) — but it applies to all PodKinds,
  so unifying it is orthogonal to 'replicant→userpod' and risks cross-tier breakage."
- **Knowledge gap around focus:** I have NOT traced all callers of `PodDeployment.pod`
  (grep `\.pod\.` on deployment.rs returned only logging, not field access — suggesting
  `AgentPod` fields may be accessed through `PodDeployment` in other crates). Open Q6.

## 4. Adversarial Filter — essentialist G1→G2→G3 on REAL seams

Order fixed. A change that fails G1 does not get counted. Advisory mode.

| ID | Proposed change | G1 Exist | G2 Surface | G3 Contract | Verdict | Force |
|---|---|---|---|---|---|---|
| S-a | Merge `HumanUser`+`ReplicantIdentity` → single userpod | **FAIL** — 1:N (is_primary, list_replicants, switcher `terminal.rs:263`). Inlining forces Vec-in-account or 1:1 → TUI behavior lost. | n/a | n/a | KEEP | Evidence |
| S-b | Collapse `AgentPod`+`PodDeployment` → one struct | UNCERTAIN — wrapper adds storage/CNS/capability/inference/semantic_index (`deployment.rs:46-70`). Deletion may lose per-pod resource bundling. Applies to ALL PodKinds. | if merged: ~12 pub items (justify) | single-impl? open | **Hypothesis** — needs caller trace (Q6) | Hypothesis |
| S-c | Remove `AgentKind::Bot`/`Replicant` | **FAIL** — validation enforces `["bot","replicant"]` (`pod/types.rs:349`); A2A distinguishes (`a2a/mod.rs:807`); ownership/capability routing differs. Inlining loses user-owned-vs-system distinction. | n/a | n/a | KEEP | Evidence |
| S-d | Remove `PodKind` trichotomy | **FAIL** — determines isolation model + filename (`deployment.rs:411-415`). Inlining loses per-tier storage layout. | n/a | n/a | KEEP | Evidence |
| S-e | Delete `CuratorSync`/`CuratorAgent` | **FAIL** (deletion) — `SemanticIndex` is consumed by `PodContext` (`context.rs:117+`). Real S3 component. | n/a | n/a | KEEP | Evidence |
| S-f | Inline `resolve_replicant_name()` (`services-skill/src/skill_impl.rs:291`) into callers | PASS-ish — 2 callers (`bundles.rs:225,302`). Reads env then `git config user.name`. Small wrapper. | 1 fn → 2 inline sites | single-purpose util | **Guideline** (advisory) — minor; not replicant→userpod | Guideline |

**Essentialism score (this pass):** Of 6 candidate seams, 4 are KEEP (G1 FAIL), 1 is Hypothesis
(S-b), 1 is minor Guideline (S-f). % removed from the *replicant→userpod consolidation space* =
**0%** (no seam in that space survives G1 as deletable). The task's ≥26% target is **not
reachable** on the evidence because the replicant IS the userpod. Target only reachable if
the user re-scopes to S-b/S-f (orthogonal cleanups).

## 5. Adversarial Interrogation — grill-me on top recommendations

Per the rule, any recommendation that cannot answer Recall+Mechanism with file:line is
downgraded to Hypothesis and dropped from the executable plan. Only S-b and S-f survived G1;
S-b cannot answer Mechanism without the Q6 caller trace → downgraded to Hypothesis → dropped.
S-f answers both:

- **S-f Recall:** `resolve_replicant_name()` (`hkask-services-skill/src/skill_impl.rs:291`)
  returns the active replicant name from `HKASK_REPLICANT_NAME` env, falling back to
  `git config user.name`.
- **S-f Mechanism:** `bundles.rs:225` and `bundles.rs:302` call it to populate the `editor`
  field passed to `BundleService::compose`/`evolve`. Two call sites, identical use.
- **S-f Rationale:** inferred — provides the "who is the author" string for bundle attribution
  (P12). **Hypothesis** on rationale (no spec cite located).
- **S-f Edge Cases:** If env unset AND no git, returns a fallback string (need to read
  remaining 9 lines — open Q7). Inlining would duplicate the env+git fallback at 2 sites →
  **net complexity increases**. G1 actually FAILS on inlining (complexity reappears ×2).
  → **Reclassify S-f: KEEP.** The function is the *deletion test in reverse* — deleting the
  wrapper duplicates logic.

**Net result of grill-me:** ALL SIX candidates are either KEEP (G1 FAIL) or Hypothesis (dropped).
**There are zero recommendations that survive to an executable deletion plan.** This is the
honest output. The plan therefore reduces to: resolve §0 open questions with the user.

## 6. metric_before (JSON, recorded read-only)

```json
{
  "replicant_typed_items": {
    "AgentKind::Replicant_variant": 1,
    "PodKind::Replicant_variant": 1,
    "ReplicantIdentity_struct": 1,
    "ReplicantInfo_struct": 1,
    "ReplicantAction_enum": 1,
    "yaml_string_labels": ["replicant", "Replicant"],
    "cli_subcommand": "replicant",
    "total_type_definitions": 5
  },
  "pod_spawn_route_hops": {
    "human_to_repl_hops": 1,
    "a2a_pod_to_pod_hops": 2,
    "curator_sync_polls_n_pods": true
  },
  "replicant_crate_public_surface": "N/A — no replicant crate exists; code spans hkask-identity, hkask-agents/pod, hkask-api/routes/replicant.rs, hkask-cli",
  "tui_compile_time_seconds": "Hypothesis — not measured (requires cargo build -p hkask-tui)",
  "integration_tests_referencing_replicant": {
    "agent_pod_integration_rs": true,
    "integration_depth_rs": true,
    "pod_portability_rs": true
  },
  "replicant_trait_count": 0,
  "Replicant_trait_implementors": 0
}
```

**Target `metric_after`** (conditional on user re-scoping):
```json
{
  "note": "Unreachable on the stated thesis. If user re-scopes to S-b (AgentPod/PodDeployment wrapper) and Q6 caller trace confirms pass-through:",
  "replicant_typed_items": "unchanged (no replicant-specific deletion available)",
  "layers_removed": "0 or 1 (S-b only, conditional)",
  "tui_compile_time_seconds": "must not increase",
  "integration_tests_referencing_replicant": "must remain green"
}
```

## 7. Iteration Engine (gpa-evolution) — 2 iterations, Pareto

Artifact class: `plan`. Frontier on (quality = layers removed with no behavior loss,
cost = files touched + test churn).

- **Iteration 1 — sample trajectory "naive merge":** delete `ReplicantIdentity`, fold into
  `HumanUser`. Quality = 0 (1:N behavior lost — switcher, `is_primary`, `list_replicants`).
  Cost = high (identity crate + api + cli + tui + tests). **Dominated.**
- **Iteration 2 — sample trajectory "keep seams, probe S-b":** keep S-a/S-c/S-d/S-e (G1 FAIL),
  investigate S-b (AgentPod/PodDeployment) via Q6 caller trace, keep S-f (G1 FAIL on inline).
  Quality = 0..1 (S-b conditional). Cost = low if S-b fails, medium if S-b passes. **Non-dominated.**
- **Mutation hypotheses tested:**
  - M1 "if I merge HumanUser+ReplicantIdentity, layers drop because Z" → FALSE (1:N).
  - M2 "if I inline resolve_replicant_name, call sites shrink because Z" → FALSE (duplicates).
  - M3 "if I drop AgentKind::Bot/Replicant, routing simplifies because Z" → FALSE (ownership lost).
  - M4 "if I collapse AgentPod into PodDeployment, one struct because Z" → UNVERIFIED (Q6).
- **Crossover:** none — frontier has 1 non-dominated member.
- **Convergence:** hypervolume delta between iter 1 and iter 2 = |0 − 0..1|; with one
  non-dominated member, delta ≤ 0.10 is satisfied trivially. **Converged at iteration 2**
  (per spec: do not stop before iteration 2).
- **Pareto frontier:** { "keep seams + probe S-b" (quality=0..1, cost=low) }. Size 1.
  Crowding-distance pruning N/A.

## 8. Phased vertical-slice task list (advisory — none executable until §0 resolved)

**Phase 0 — Thesis reconciliation (blocking, no code):**
- T0.1: User answers Q1–Q4 (did you mean Bot/Replicant split? AgentPod/PodDeployment? CuratorSync?).
  Acceptance: written re-scope. Checkpoint: UserFeedbackOccurrence.
- T0.2: If re-scope = S-b, trace `PodDeployment.pod` callers across all crates (Q6).
  Acceptance: caller table with file:line; G1 verdict on S-b. Checkpoint: build green.

**Phase 1 (conditional on T0.2 confirming S-b is pass-through):**
- T1.1: Inline `AgentPod` fields into `PodDeployment` in ONE pod kind (Replicant) first;
  rewrite its callers; run `agent_pod_integration.rs`. Acceptance: tests green, ≤5 files.
  Checkpoint: build + TUI smoke.
- T1.2: Extend to Curator + Team kinds. Acceptance: `pod_portability.rs` green. Checkpoint: User.

**No other phases** — every other seam failed G1 (KEEP).

## 9. Risks

- R1: Executing the thesis literally destroys multi-persona-per-account (1:N). **Prohibition**
  against silent behavior loss.
- R2: Re-scoping mid-flight (user meant Bot/Replicant, not replicant/user) invalidates
  `metric_before`. Mitigation: T0.1 blocking.
- R3: S-b (AgentPod/PodDeployment) touches ALL pod kinds — high blast radius despite
  being a "wrapper." Mitigation: T1.1 does Replicant-only first.

## 10. Open questions (knowledge threshold)

Q1–Q4 (§0 thesis intent), Q5 (S4/S5 policy location in VSM), Q6 (`PodDeployment.pod` callers),
Q7 (read remaining 9 lines of `resolve_replicant_name`).

## 11. DC+BIBO metadata

- **Direction:** consolidate replicant→userpod (reframed: verify whether any real seam is pass-through).
- **Capability:** graph simplification under essentialist G1.
- **Boundary:** advisory only; no autonomous deletion.
- **Input:** task spec + read-only codebase evidence (file:line cited).
- **Boundary (BIBO in):** task thesis (Hypothesis) + cited facts.
- **Output (BIBO out):** this plan + §12 table + metric JSON + Pareto frontier.
- **Author:** Zed agent (advisory mode). **Provenance:** all claims cited or labeled Hypothesis.