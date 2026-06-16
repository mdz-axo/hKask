---
name: skill-logic-audit
visibility: public
description: "Audit any .j2 template or manifest.yaml against its own stated goal. Adversarial critique, soundness filter, concrete revision proposal, and bounded user-review loop. Use when a template's logic may be redundant, ambiguous, or inefficient, or as a self-application gate before committing skill changes."
---

# skill-logic-audit — Self-Applied Logic Audit

A meta-skill that audits hKask templates and manifests for logical correctness, minimality, and alignment with their own stated goal. It does not check style or formatting; it checks whether the artifact is the most efficient and correct way to achieve the goal it claims.

## When to Use

- Before committing a new or revised `.j2` template or `manifest.yaml`.
- When a template body and its `contract` appear to disagree.
- When a `FlowDef` duplicates logic that should live in a `WordAct` or `KnowAct`.
- When you suspect a template contains speculative features or impossible error handling.
- As a CI gate: fail the build if any skill health score is below threshold or any template type is invalid.

## Goal Annotation Convention

Every auditable artifact must include a concise `goal:` comment block near the top:

```jinja2
{# goal: Given a module path and source, enumerate its public interface,
         evaluate its behavior complexity, compute a depth score, and classify
         the module as Deep/Adequate/Shallow/VeryShallow. #}
```

For manifests:

```yaml
# goal: Describe this skill's runtime templates and link them to the Zed companion.
```

The `goal:` block is the single source of truth for the audit. Everything else in the artifact is judged against it.

## Audit Cascade

The runtime executes a `FlowDef` (`registry/templates/skill-logic-audit/user-review-loop.j2`) that orchestrates:

1. **load-goal** (`WordAct`, `registry/templates/skill-logic-audit/load-goal.j2`) — parse the `goal:` block from the target artifact.
2. **critique-template** (`KnowAct`, `registry/templates/skill-logic-audit/critique-template.j2`) — adversarial critique anchored to the goal.
3. **critique-critique** (`KnowAct`, `registry/templates/skill-logic-audit/critique-critique.j2`) — soundness filter; separate valid concerns from spurious ones.
4. **compose-proposal** (`KnowAct`, `registry/templates/skill-logic-audit/compose-proposal.j2`) — produce a concrete revised artifact and a diff.
5. **user-choice** (`WordAct`, `registry/templates/skill-logic-audit/user-choice.j2`) — present the proposal and branch on:
   - `accept`: write the revised artifact and re-run the audit.
   - `reject`: discard the proposal and stop.
   - `counter-proposal`: capture user edits and route back to `compose-proposal`.

The loop increments a depth counter; after the configured maximum it escalates to the user rather than continuing.

## Constraints

- Critiques must be anchored to the `goal:` block, not general stylistic preferences.
- Do not execute arbitrary code from template bodies.
- The loop is bounded; never recurse infinitely.
- A proposal must be rejected if it weakens the artifact's contract or introduces stubs.

## Related Skills

- `constraint-forces` — classify findings by Prohibition/Guardrail/Guideline/Evidence/Hypothesis.
- `pragmatic-semantics` — distinguish IS from OUGHT in audit reports.
- `essentialist` — challenge whether every line survives the deletion test.
- `coding-guidelines` — constrain implementation to surgical, simple, goal-driven changes.
