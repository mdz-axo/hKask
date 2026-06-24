---
name: review
visibility: public
description: "Self-critique a reasoning output for contradictions, unsupported claims, logical gaps, and confidence calibration before finalizing an answer."
---

# review — Self-Critique

A companion skill for any reasoning or synthesis task. It inspects derived knowledge against the original goal and flags missing information, contradictions, unsupported claims, logical gaps, and poorly calibrated confidence.

## When to Use

- Before finalizing a complex answer or plan.
- When the user asks "are you sure?" or challenges a claim.
- Before entering an iterative refinement loop with `self-critique-revision`.
- When confidence scores seem out of line with the evidence base.

## When NOT to Use

- The response is trivial or already fully grounded in provided evidence.
- The user explicitly wants creative, unverified speculation.
- There is no clear goal or set of derived facts to evaluate.

## Instructions

1. Identify the goal query, agent identity, and active project from context.
2. Collect the derived facts, constraints, and any evidence provenance.
3. Render `registry/templates/review/self_critique.j2` with the gathered inputs.
4. Examine the returned critique for blocking issues.
5. If issues are found, route to the appropriate revision skill or back to the user.
6. If the critique returns "NONE", finalize the answer.

## Constraints

- Do not weaken a correct answer just to satisfy a generic checklist.
- Flag only concrete issues anchored to the goal or evidence.
- Do not invent evidence; mark claims as `[UNVERIFIED]` when appropriate.

## Related Skills

- `self-critique-revision` — iterative revision after critique.
- `skill-logic-audit` — audit the critique template itself for logical flaws.
- `constraint-forces` — classify findings by Prohibition/Guardrail/Guideline/Evidence/Hypothesis.

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `self_critique.j2` | WordAct | Self-critique reasoning output against quality criteria |


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/review.yaml`

### PDCA Convergence
- **Threshold:** 0.25 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = no unresolved high-severity critique blockers

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 14000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
