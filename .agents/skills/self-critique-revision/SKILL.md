---
name: self-critique-revision
visibility: public
description: "Run an iterative self-critique and revision cycle: generate an initial draft, critique it against explicit quality criteria, then revise based on the structured critique."
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

## Instructions

1. Capture the task prompt and the quality criteria from the user.
2. Render `registry/templates/self-critique-revision/generate.j2` to produce draft 1.
3. Render `registry/templates/self-critique-revision/critique.j2` with the draft and criteria.
4. If the critique returns no material issues, return the draft.
5. Otherwise, render `registry/templates/self-critique-revision/revise.j2` to produce draft N+1.
6. Repeat steps 3–5 up to the configured iteration limit.
7. Return the final draft and a summary of what changed.

## Constraints

- Do not invent evidence to satisfy a critique point.
- Do not ignore any critique point; if one cannot be addressed, explain why.
- Preserve all technical substance (code, errors, URLs) across revisions.
- Stop iterating if the quality score stops improving.

## Related Skills

- `review` — lightweight self-critique before finalizing.
- `skill-logic-audit` — audit the critique/revision templates themselves.
- `constraint-forces` — classify revision findings by force type.
