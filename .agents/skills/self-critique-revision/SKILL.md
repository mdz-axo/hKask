---
name: self-critique-revision
visibility: public
description: "Run an iterative self-critique and revision cycle: generate an initial draft, critique it against explicit quality criteria, then revise based on the structured critique."
composes_skills: [pragmatic-semantics]
---

# self-critique-revision — Iterative Improvement Loop

Use this skill to tighten a response through a bounded loop of generation, critique, and revision. Each iteration produces a new draft with a quality score; the loop stops when the critique finds no material issues or when a configured iteration limit is reached.

## When to Use

- The user asks for a high-quality answer to a complex question.
- A first draft is likely to contain unsupported claims, logical gaps, or poor calibration.
- The task has explicit quality criteria (accuracy, clarity, completeness, etc.).

## When NOT to Use

- The answer is short and fully grounded in provided evidence.
- The user wants a quick, provisional answer rather than a polished one.
- No quality criteria can be articulated.

## Quality Criteria Framework

The critique loop requires measurable criteria to converge. Without them, "no material issues" and "quality score" are undefined and the PDCA convergence threshold of 0.15 is unverifiable.

### Required Structure

Every invocation must define quality criteria using this template:

| Criterion | Weight | Target | Description |
|-----------|--------|--------|-------------|
| Accuracy | 1.0 | ≥ 4 | Factual correctness, no unsupported claims |
| Clarity | 1.0 | ≥ 4 | Unambiguous, well-structured, appropriate level of detail |
| Completeness | 1.0 | ≥ 4 | All parts of the question addressed, no gaps |
| Calibration | 0.5 | ≥ 3 | Certainty appropriately expressed, uncertainty acknowledged |

- **Score per criterion:** 1 (severe gap) to 5 (no issues)
- **Weighted quality score:** `sum(weight × score) / sum(weights)` — normalized to 1–5 scale
- **Convergence threshold:** Weighted quality score ≥ target AND no single criterion < 3
- **"No material issues":** All criteria at or above their individual targets
- Weights and targets may be user-adjusted. The default framework above applies when none are specified.

### The Quality Score Maps to PDCA Convergence

The PDCA convergence threshold of 0.15 is operationalized as:
- `convergence_metric = (5 - weighted_quality_score) / 4`
- At quality score 4.4: metric = 0.15 → converged
- At quality score 5.0: metric = 0.0 → fully converged

## Instructions

1. Capture the task prompt from the user.
2. Establish quality criteria using the framework above. If the user provides criteria, map them to the 1–5 scoring template. If not, apply the default framework.
3. Render `registry/templates/self-critique-revision/generate.j2` to produce draft 1 with an initial quality self-assessment.
4. Render `registry/templates/self-critique-revision/critique.j2` with the draft and criteria. The critique must produce:
   - A score (1–5) for each criterion
   - A weighted quality score
   - Each finding classified by constraint force:
     - **Prohibition:** factual errors, unsafe claims — must fix
     - **Guardrail:** ambiguous or misleading statements — should fix unless overridden
     - **Guideline:** style, structure, calibration improvements — preferred but optional
     - **Evidence:** informational observations — no action required
5. If the critique returns no material issues (all criteria at or above target, no Prohibition or Guardrail findings), return the draft.
6. Before accepting revision N+1, verify no regression: compare each criterion score against the previous iteration. Any criterion that dropped by ≥ 1 point must be flagged and addressed explicitly in the revision.
7. Render `registry/templates/self-critique-revision/revise.j2` to produce draft N+1, addressing every Prohibition and Guardrail finding from the critique.
8. Repeat steps 4–7 up to the configured iteration limit (max 3).
9. Return the final draft, the final quality scores, and a summary of what changed across iterations.

## Constraints

- Do not invent evidence to satisfy a critique point.
- Do not ignore any critique point; if one cannot be addressed, explain why and classify it as an accepted Guardrail override.
- Preserve all technical substance (code, errors, URLs) across revisions.
- Stop iterating if the quality score stops improving (no improvement across two consecutive iterations).
- All critique findings must carry a constraint-force classification (Prohibition/Guardrail/Guideline/Evidence).
- Regression on any criterion by ≥ 1 point must be flagged and explicitly addressed before accepting a revision.

## Related Skills

- `review` — lightweight self-critique before finalizing (Evidence: alternative, not a dependency).
- `skill-logic-audit` — audit the critique/revision templates themselves (Evidence: reflexive quality check).
- `pragmatic-semantics` — **composed** into the critique output format; every finding carries a force classification.

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `generate.j2` | WordAct | Produce an initial draft and quality score |
| `critique.j2` | KnowAct | Evaluate a draft against quality criteria |
| `revise.j2` | KnowAct | Produce a revised draft addressing each critique |


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/self-critique-revision.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = all major critique issues resolved, incremental gains < improvement target

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 18000 rJ
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
