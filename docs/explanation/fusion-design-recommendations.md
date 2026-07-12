---
title: "Fusion System Design Recommendations"
audience: [developers, agents]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition]
---

# Fusion System Design Recommendations

**Composed using:** pragmatic-laziness, essentialist, grill-me, coding-guidelines
**Date:** 2026-07-12
**Status:** Recommendations — items marked ADD/DON'T/DEFER

---

## Design Question 1: Should per-manifest FusionConfig support partial inheritance?

**Question:** Should `judge: null` mean "inherit global judge" while overriding only the panel?

### Pragmatic-laziness (path of least action)
The current all-or-nothing config is simpler. Partial inheritance adds a new concept (field-level null = inherit) that increases total system action. If a skill wants to inherit the global judge but customize the panel, the manifest author can read the global env vars and copy the values. Deletion test: delete partial inheritance → nothing breaks → don't add it.

### Essentialist (3-gate)
- **G1 Exist:** Does partial inheritance earn its existence? No — callers can copy values from the global config. The abstraction is a convenience, not a necessity. **G1 FAIL.**
- **Conclusion:** Don't add.

### Recommendation: **DON'T** — The current all-or-nothing replacement is simpler and sufficient. Partial inheritance adds a concept without adding capability. If measured need arises (e.g., many skills wanting to override only the panel), revisit.

---

## Design Question 2: Should "fusion mode" be a first-class manifest concept?

**Question:** Should there be a top-level `fusion_mode: synthesis | critique | deliberation | best-of-n | pi | disabled` shorthand?

### Pragmatic-laziness
The `fusion:` block already IS the concept. Adding a shorthand `fusion_mode: synthesis` would duplicate what `fusion: { mode: synthesis }` already does. Two ways to say the same thing is more total action, not less. Deletion test: delete the shorthand → nothing breaks → don't add it.

### Essentialist (3-gate)
- **G1 Exist:** Does a separate fusion_mode shorthand earn its existence? No — `fusion: { mode: X }` already exists. The shorthand is a pass-through to the full config. **G1 FAIL.**
- **Conclusion:** Don't add.

### Coding-guidelines (Simplicity First)
Two config mechanisms for the same thing violates simplicity. The full `FusionConfig` block is already minimal (5 fields). A shorthand would add a second parsing path and a precedence rule (which wins when both are present?).

### Recommendation: **DON'T** — The `fusion:` block is the concept. Adding a shorthand duplicates it. The full config is already minimal (5 fields, YAML-clean).

---

## Design Question 3: Should fusion and dual-model be unified under a single "multi-model strategy" abstraction?

**Question:** Should there be a `multi_model: { strategy: fusion | dual | single, ... }` field?

### Pragmatic-laziness
They solve different problems: fusion = deliberation quality (N perspectives → judge synthesis), dual-model = epistemic integrity (2 jurisdictions → merge). Unifying them creates a wider interface with more parameters, increasing total system action. Deletion test: delete the unified abstraction → two separate concepts are simpler and never used together → don't unify.

### Essentialist (3-gate)
- **G1 Exist:** Delete the unified abstraction → two concepts reappear in callers → but the two concepts are used independently and never composed → the unified abstraction is a pass-through that adds a layer without adding behavior. **G1 FAIL.**
- **G2 Surface:** The unified interface would need to accept all parameters from both systems (judge, panel, mode, skills, max_rounds, model_a, model_b, drift_threshold) — far exceeding the 7-function rule.
- **Conclusion:** Don't unify.

### Grill-me (Edge cases)
What happens when both `strategy: fusion` and `strategy: dual` are needed on the same step? Currently, `dual_model: true` bypasses fusion — they're mutually exclusive by design. A unified abstraction would need to express this mutual exclusion, adding complexity. The current design makes the exclusion structural (two separate fields) rather than logical (a strategy enum with a constraint).

### Recommendation: **DON'T** — Fusion and dual-model are intentionally orthogonal. They solve different problems with different mechanisms. Unifying them would create a wider interface without adding capability. The mutual exclusion (`dual_model` always bypasses fusion) is simpler as a structural rule than as a strategy enum constraint.

---

## Design Question 4: Should the scenario-builder quality gate skip implications when it fails?

**Question:** Should the implications step (step 6) be skipped when the quality gate (step 5) fails, to save ~5000 gas?

### Pragmatic-laziness
The `condition:` field already exists on `BundleManifestStep`. Adding `condition: "step_5_result.gate_pass"` to the implications step is zero new code — it uses an existing primitive. This is the path of least action: no new mechanism, just wiring an existing one. **This is the least-action configuration.**

### Essentialist (3-gate)
- **G1 Exist:** Delete the conditional skip → implications step runs even when gate fails → 5000 gas wasted per failed iteration → complexity reappears as wasted energy. **G1 PASS.**
- **G2 Surface:** One field (`condition:`) on one step. Minimal. **G2 PASS.**
- **G3 Contract:** The condition expression is explicit and already verified by the flow engine's `evaluate_step_condition()`. **G3 PASS.**
- **Conclusion:** Add it.

### Coding-guidelines (Surgical Changes)
One-line change to one manifest. No new code. No new concepts. Existing mechanism (`condition:`) applied to an existing problem (wasted gas on doomed iterations). Verifiable: the implications step should not appear in the context when `gate_pass` is false.

### Recommendation: **ADD** — Implemented. Added `condition: "step_5_result.gate_pass"` to the implications step in `scenario-builder.yaml`. This uses the existing `condition:` mechanism — zero new code, ~5000 gas saved per failed gate iteration.

---

## Design Question 5: Should the superforecasting loop target be conditional?

**Question:** Should the loop restart at different steps depending on what the convergence check diagnoses (e.g., restart at evidence update if evidence is weak, at calibration if calibration drifted)?

### Pragmatic-laziness
The current fixed `loop_target: 2` (Fermi) is simpler. Conditional loop targets require the flow engine to evaluate conditions on the loop step, which is a new mechanism. The PDCA loop + `max_iterations` + `on_not_reached: escalate` handles the edge cases. Deletion test: delete conditional loop → edge cases handled by max_iterations + escalate → don't add it.

### Essentialist (3-gate)
- **G1 Exist:** Delete conditional loop → some iterations target the wrong step → but `max_iterations` + escalate catches it → complexity doesn't reappear, just suboptimal iteration. **G1 marginal FAIL** — the waste is real but bounded.
- **Conclusion:** Don't add now. Revisit if iteration waste is measured.

### Grill-me (Edge cases)
What if the convergence check says "evidence is weak" but the Fermi decomposition was also flawed? The conditional logic would need a priority order, which is domain-specific and hard to generalize. The current fixed target (Fermi) is a reasonable default because decomposition is the foundation — if the foundation is wrong, everything downstream is wrong too.

### Recommendation: **DEFER** — The fixed loop_target is simpler and the edge cases are bounded by `max_iterations: 3` + escalate. Conditional targets would add domain-specific routing logic to the flow engine. Revisit if measured iteration waste justifies the complexity.

---

## Follow-up A: Does the per-manifest fusion config work end-to-end?

### Grill-me (Recall → Mechanism → Rationale → Edge Cases → Synthesis)

**Recall:** We have 3 router-level tests (`per_call_fusion_config_*`) and 3 executor-level tests (`fusion_config_wiring`).

**Mechanism:** Executor sets `params.fusion_config` from `manifest.fusion` → Router checks `params.fusion_config` before global config → Orchestrator dispatches to panel + judge.

**Rationale:** The tests verify the wiring at both levels. The routing logic (the part we test) is identical whether using mock or real inference.

**Edge case:** The mock returns canned JSON without going through the panel + judge. Real inference would dispatch to N panel models in parallel, then the judge synthesizes. However, the fusion routing decision (whether to use fusion at all, and which config to use) is what we're testing — the actual panel dispatch is the `fusion_orchestrator`'s responsibility, which is unchanged.

**Synthesis:** Yes, the per-manifest fusion config works end-to-end at the routing level. Full E2E verification (panel models actually respond, judge actually synthesizes) requires live API keys. The routing tests cover the contract: manifest → executor → router → orchestrator.

### Recommendation: **VERIFIED** — The routing path is tested at both the executor and router levels. Full live E2E is a future task that requires API keys or a mock `InferenceRouter` (not just `InferencePort`).

---

## Follow-up B: What happens when a custom-judge model isn't available?

### Grill-me (Mechanism → Edge Cases → Synthesis)

**Mechanism:** `call_judge()` calls `router.resolve(judge_model)` → if the model's provider isn't configured, returns `Err(InferenceError::Connection)` → the orchestrator propagates this → the executor receives `TemplateError::Inference(e)` → the flow engine treats it as a step failure.

**Edge case:** If panel models succeed but the judge fails, the panel results are lost. This is correct — a fusion run without a judge is meaningless. If ALL panel models fail, `dispatch_panel` returns an empty vec and the orchestrator returns an error before reaching the judge.

**Synthesis:** Custom-judge unavailability causes a graceful error that aborts the skill. This is correct behavior. The error is logged with `target: "cns.inference"` and the specific model name. Recommendation: document this in the architecture doc (done — the per-manifest fusion section mentions that panel/judge models must be available on configured providers).

### Recommendation: **VERIFIED** — Graceful error handling is already in place. The error propagates through `TemplateError::Inference` and aborts the skill. Documented in the architecture doc.

---

## Follow-up C: Should other skills in the catalog get fusion configs?

### Pragmatic-laziness (effort hotspot analysis)
Skills with generative analysis (diagnosis, design, critique) are effort hotspots where fusion's multi-model deliberation adds the most value per unit of cost. Skills with deterministic rubric evaluation are NOT hotspots — fusion adds noise without adding signal.

### Grill-me (classification)
| Skill type | Fusion benefit | Examples |
|------------|---------------|----------|
| Generative analysis | High — diverse perspectives improve quality | diagnose, review, self-critique-revision, metacognition, improve-codebase-architecture, bug-hunt, idiomatic-rust, deep-module, refactor-service-layer |
| Deterministic rubric | Low — rubric evaluation doesn't benefit from multiple opinions | goal-analysis, skill-logic-audit, semantic-graph-audit, magna-carta-verifier |
| Interactive/dialogue | Medium — depends on use case | kata-coaching, grill-me, improv, essentialist, pragmatic-laziness |
| Infrastructure | None — not agent-facing | qa-*, cns-gas-tracking, bootstrap-sequence, dispatch |

### Recommendation: **ADD GRADUALLY** — Add fusion configs to the 9 generative-analysis skills that don't have it yet. Use per-step `fusion: false` on convergence checks. Use mode-appropriate deliberation:
- `diagnose` → `critique` (draft → panel critiques → revise)
- `review` → `critique` (same pattern)
- `self-citique-revision` → `deliberation` (multi-round refinement)
- `metacognition` → `synthesis` (compose perspectives)
- `improve-codebase-architecture` → `deliberation` (multi-round exploration)
- `bug-hunt` → `best-of-n` (pick the best diagnosis)
- `idiomatic-rust` → `critique` (draft → challenge → refine)
- `deep-module` → `synthesis` (compose module assessments)
- `refactor-service-layer` → `pi` (plan → implement)

Priority: `diagnose` and `review` first (highest value, simplest mode). Others as demand arises.

---

## Follow-up D: Is improvement_ratio a dead field in other skills?

### Essentialist (deletion test)
Found: `improvement_ratio` exists in **49 manifests**. ALL use `improvement_gate: threshold_only`, which means the field is never consulted by the executor's `check_convergence()` function. This is a **Prohibition #3 violation** (hidden parameter — the field is declared but unused).

### Pragmatic-laziness (deletion test)
Delete `improvement_ratio` from all 49 manifests → nothing breaks (the executor ignores it when `improvement_gate: threshold_only`) → total system action decreases (49 fewer lines of dead config) → delete it.

### Recommendation: **DELETE** — Remove `improvement_ratio` from all 49 remaining manifests. The field is dead in every case (all use `threshold_only`). This is a Prohibition #3 cleanup. The 5 skills we already cleaned (superforecasting, scenario-builder, essentialist, grill-me, pragmatic-laziness) demonstrated the pattern. A workspace-wide `sed` deletion is the path of least action.

---

## Summary

| # | Question | Recommendation | Rationale |
|---|----------|---------------|-----------|
| Q1 | Partial inheritance in FusionConfig | **DON'T** | All-or-nothing is simpler; copy values if needed |
| Q2 | Fusion mode as first-class concept | **DON'T** | `fusion:` block IS the concept; shorthand duplicates it |
| Q3 | Unify fusion + dual-model | **DON'T** | Orthogonal by design; unifying widens interface without adding capability |
| Q4 | Quality gate conditional skip | **ADD** (implemented) | One-line change using existing `condition:` field; saves ~5000 gas |
| Q5 | Conditional loop target | **DEFER** | Fixed target is simpler; edge cases bounded by max_iterations + escalate |
| FA | End-to-end fusion verification | **VERIFIED** | 6 tests cover executor → router → orchestrator routing path |
| FB | Custom-judge unavailability | **VERIFIED** | Graceful error propagation already in place |
| FC | Fusion for other skills | **ADD GRADUALLY** | 9 generative-analysis skills would benefit; prioritize diagnose + review |
| FD | improvement_ratio dead field | **DELETE** | 49 manifests have this dead field (Prohibition #3); workspace-wide cleanup |