---
title: "hLexicon Alignment Validation Report"
audience: [architects, developers, agents]
last_updated: 2026-06-07
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain]
---

# hLexicon Alignment Validation Report

**Date:** 2026-06-07
**Version:** v0.22.0

## Summary

| Metric | Value |
|--------|-------|
| Total Templates/Manifests | 243 |
| With functional_role | 124 |
| Missing functional_role | 119 |
| Compliance Rate | 51% |

## Functional Distribution

| Category | Count | Percentage |
|----------|-------|------------|
| WordAct | 11 | 8% |
| FlowDef | 56 | 44% |
| KnowAct | 57 | 45% |

## Validation Results

❌ 119 templates missing functional_role
✅ Distribution balanced (no category >60%)

## Templates Missing functional_role

- `/home/mdz-axolotl/Clones/hKask/registry/templates/caveman/caveman-compress.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/pragmatic-composition/prompt_template.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/knowact/reflect.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/knowact/calibrate.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/knowact/falstaffian-perspective.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/knowact/ellipsis-analysis.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/knowact/detect.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcda/identify-criteria.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcda/rank-alternatives.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcda/weight-and-score.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcda/sensitivity-analysis.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcp/inference_call.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcp/scholar_extract.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcp/doc_extract.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcp/web_extract.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/mcp/condense_session.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/wordact/render.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/wordact/execute.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/wordact/selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/wordact/soap.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/tdd/tdd-plan.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/tdd/tdd-refactor.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/tdd/tdd-verify.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/tdd/tdd-gap-check.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/tdd/tdd-tracer.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/create_exemplar.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/get_persona.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/create_persona.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/unbind_persona.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/validate_capability.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/get_exemplar.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/propose_capability.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/create_reference.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/get_reference.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/publish_capability.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/ingest_snapshot.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/get_capability.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/promote_snapshot.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/templates/spandrel/index_ontology_concept.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_2_outside_view.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_3_inside_view.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_6_calibration.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_1_fermi_decompose.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_5_synthesis.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_0_triage.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_4_evidence_update.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/superforecasting/stage_7_record.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/metacognition/meta_decompose.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/rag/synthesize-grounded.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/rag/reformulate-query.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/rag/verify-citations.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/adversarial-red-team/test-against-target.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/adversarial-red-team/multi-turn-attack.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/adversarial-red-team/select-target.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/adversarial-red-team/generate-adversarial.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/decision-journal/record-decision.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/decision-journal/compute-brier.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/decision-journal/define-expected-outcomes.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/decision-journal/revisit-evaluate.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/improve-codebase-architecture/arch-candidates.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/improve-codebase-architecture/arch-explore.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/improve-codebase-architecture/arch-deepen.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/grill-me/grill-me-assess.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/grill-me/grill-me-escalate.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/grill-me/grill-me-round.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/self-critique-revision/generate.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/self-critique-revision/revise.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/self-critique-revision/critique.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/coding-guidelines/guidelines-assess.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/coding-guidelines/guidelines-verify.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/coding-guidelines/guidelines-apply.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/composition/hemingway-style-synthesizer.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/composition/answer_composition.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/handoff/handoff-skills-suggest.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/handoff/handoff-compose.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/handoff/handoff-artifacts.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/handoff/handoff-compact.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/chain-of-density/density-pass.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/chain-of-density/initial-summary.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/zoom-out/zoom-out-context.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/improvement-step2-current.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/improvement-step4-experiment.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/coaching-q3-obstacles.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/coaching-q1-target.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/coaching-q4-experiment.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/improvement-step1-direction.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/coaching-q5-learn.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/coaching-q2-actual.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/starter-five-questions.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/starter-observation-drill.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/kata-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/starter-pdca-cycle.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/starter-selector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/kata/improvement-step3-target.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/chat-template/templates/chat-prompt.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/dct-pipeline/decimation.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/dct-pipeline/classification.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/reasoning/reasoning.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/reasoning/reason_constrained.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/diagnose/diagnose-fix.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/diagnose/diagnose-loop.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/diagnose/diagnose-hypothesise.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/diagnose/diagnose-instrument.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/structured-extraction/extract-relations.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/structured-extraction/identify-entities.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/structured-extraction/map-to-schema.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/flowdef/memory_recall.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/flowdef/dispatch.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/error-validation.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/macros.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/assess-coherence.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/error-generic.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/bind-effector.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/recognize-ensemble.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/compute-equilibrium.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/validate-inputs.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/cns-instrument.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/gml/reframe-concept.j2`
- `/home/mdz-axolotl/Clones/hKask/registry/templates/review/self_critique.j2`

## Orthogonal Mapping

Functional logic and implementation logic are orthogonal surfaces.
See `docs/architecture/hlexicon-functional-logic-note.md` for design rationale.
