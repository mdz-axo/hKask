# hLexicon Alignment Validation Report

**Date:** 2026-05-23
**Version:** v0.21.4

## Summary

| Metric | Value |
|--------|-------|
| Total Templates/Manifests | 70 |
| With functional_role | 66 |
| Missing functional_role | 4 |
| Compliance Rate | 94% |

## Functional Distribution

| Category | Count | Percentage |
|----------|-------|------------|
| WordAct | 8 | 11% |
| FlowDef | 41 | 61% |
| KnowAct | 17 | 25% |

## Validation Results

❌ 4 templates missing functional_role
⚠️ Distribution skewed

## Templates Missing functional_role

- `/home/mdz-axolotl/Clones/hKask/registry/templates/goal_create.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/goal_judge_command.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/goal_judge_simple.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/goal_judge.j2`

## Orthogonal Mapping

Functional logic and implementation logic are orthogonal surfaces.
See `docs/architecture/hlexicon-functional-logic-note.md` for design rationale.
