# hLexicon Alignment Validation Report

**Date:** 2026-05-21
**Version:** v0.21.4

## Summary

| Metric | Value |
|--------|-------|
| Total Templates/Manifests | 64 |
| With functional_role | 37 |
| Missing functional_role | 27 |
| Compliance Rate | 57% |

## Functional Distribution

| Category | Count | Percentage |
|----------|-------|------------|
| WordAct | 1 | 2% |
| FlowDef | 31 | 81% |
| KnowAct | 5 | 13% |

## Validation Results

❌ 27 templates missing functional_role
⚠️ Distribution skewed

## Templates Missing functional_role

- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcp/selectors/tool-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/curator/system_state_gather.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/curator/metacognition-escalate.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/curator/metacognition-diagnose.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/curator/metacognition-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/curator/metacognition-calibrate.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/curator/metacognition-maintain.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/cognition_detect.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/process_dispatch.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/prompt_render.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/memory/templates/recall.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/memory/templates/remember.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/memory/templates/agent_operation_memory.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/memory/selectors/operation-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/registry/selectors/selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/prompt_execute.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/process_recall.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/prompt_selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/git/selectors/operation-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/ensemble/standing_session_curator_instruction.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/ensemble/standing_session_metacognition_update.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/ensemble/selectors/participant-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/ensemble/standing_session_administrator_view.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/ensemble/standing_session_status_report.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/cns/selectors/alert-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/inference/selectors/model-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/cognition_calibrate.j2`

## Orthogonal Mapping

Functional logic and implementation logic are orthogonal surfaces.
See `docs/architecture/hlexicon-functional-logic-note.md` for design rationale.
