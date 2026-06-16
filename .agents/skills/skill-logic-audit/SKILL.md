---
name: skill-logic-audit
visibility: public
description: Audit any .j2 template or manifest.yaml against its annotated goal. Uses a recursive critique loop with bounded depth.
---

# Skill-Logic-Audit

Audit any `.j2` template or `manifest.yaml` against its own stated `goal:` annotation. The skill runs a recursive critique loop: load the goal, critique the template body for logical efficiency and correctness, critique the critique for soundness, compose a revised template, and present it to the user for review.

## When to Use

- Before declaring a new or recalibrated skill "active" (health score ≥ 0.8).
- When a template's body has grown verbose, contradictory, or drifted from its purpose.
- When you suspect a `FlowDef` duplicates logic that should live in a `WordAct` or `KnowAct`.
- As a bootstrap validation: run the skill on its own templates once it exists.

## Goal Annotation Convention

Every auditable `.j2` and `manifest.yaml` must begin with a concise `goal:` comment block:

```jinja2
{# goal: Given a user task and an assessment, produce a constrained implementation plan that touches only necessary files. #}
```

```yaml
# goal: Capture a new specification with inferred MDS category and seeded acceptance criteria.
```

The goal is the auditable contract. The audit answers: "Given this goal, is the body the most logically efficient, correct, and minimal way to achieve it?"

## Audit Flow

1. **Load goal** (`load-goal.j2`, WordAct) — parse the annotated `goal:` block from the target file.
2. **Critique template** (`critique-template.j2`, KnowAct) — invoke near-frontier critique models with the goal and template body. Output concrete flaws, redundancies, ambiguities, missing cases.
3. **Critique critique** (`critique-critique.j2`, KnowAct) — review the critique output for soundness; separate valid concerns from spurious ones.
4. **Compose proposal** (`compose-proposal.j2`, KnowAct) — produce a concrete revised template body and a diff against the original.
5. **User review loop** (`user-review-loop.j2`, FlowDef) — present the proposal and branch on:
   - `accept` → write the revised template and re-run the audit.
   - `reject` → discard the proposal and stop.
   - `counter-proposal` → capture user edits and route back to `compose-proposal.j2`.

The loop is bounded: each counter-proposal iteration increments a depth counter; after a configured maximum, the flow escalates to the user rather than continuing.

## Constraints

- Critique must be anchored to the annotated goal, not general stylistic preferences.
- A `FlowDef` must call constituent templates by id and must never duplicate their logic inline.
- Every iteration must surface at least one concrete, actionable flaw or explicitly report "no material flaws found".
- The depth counter must be checked before any recursive call; on overflow, escalate.
- Do not execute arbitrary Python code in Jinja2 expressions.

## Related Skills

- `essentialist` — for the eliminative 3-gate review of the revised artifact.
- `coding-guidelines` — for constraining any code changes produced by the audit.
- `skill-manager` — for validating the recalibrated skill after audit.
