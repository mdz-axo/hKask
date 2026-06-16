# Dual-Layer Skill Audit Summary

Workspace version: 0.27.0

| Metric | Count |
|--------|-------|
| workspace_version | 0.27.0 |
| total_skills | 66 |
| complete_both_layers | 29 |
| zed_only | 1 |
| registry_only | 36 |
| active | 23 |
| stale_warning | 11 |
| critical | 6 |
| recommend_deprecation | 26 |

## Skill Details

### adversarial-red-team — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ generate-adversarial.j2: unknown hlexicon terms ['generate', 'adversarial', 'inject', 'hijack', 'manipulate', 'exploit', 'attack', 'indirect', 'tool_misuse', 'exfiltrate']

### caveman — stale_warning (0.73)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ caveman-compress.j2: energy_cap nested under contract (spec says top-level)
- ⚠ caveman-compress.j2: visibility nested under contract (spec says top-level)

### chain-of-density — critical (0.32)
- Zed: ✗, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ density-pass.j2: unknown hlexicon terms ['condense', 'density', 'entity', 'optimize']

### chat-template — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### cns — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### coding-guidelines — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ guidelines-apply.j2: energy_cap nested under contract (spec says top-level)
- ⚠ guidelines-apply.j2: visibility nested under contract (spec says top-level)
- ⚠ guidelines-assess.j2: energy_cap nested under contract (spec says top-level)

### condenser-continuation — active (0.83)
- Zed: ✓, Registry: ✓
- Templates: 4 .j2 | WordAct=True KnowAct=True FlowDef=True
- ⚠ condenser-continuation-compose.j2: energy_cap nested under contract (spec says top-level)
- ⚠ condenser-continuation-compose.j2: visibility nested under contract (spec says top-level)
- ⚠ condenser-continuation-prioritize.j2: energy_cap nested under contract (spec says top-level)

### constraint-forces — active (0.96)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ constraint-forces-classify.j2: energy_cap nested under contract (spec says top-level)
- ⚠ constraint-forces-classify.j2: visibility nested under contract (spec says top-level)
- ⚠ constraint-forces-resolve.j2: energy_cap nested under contract (spec says top-level)

### curator — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 6 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ metacognition-calibrate.j2: missing [inference] frontmatter

### dct-pipeline — critical (0.32)
- Zed: ✗, Registry: ✓
- Templates: 2 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ classification.j2: unknown hlexicon terms ['categorize', 'parent', 'ontological', 'epistemic']

### decision-journal — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ compute-brier.j2: unknown hlexicon terms ['brier', 'probability', 'outcome', 'accuracy']

### deep-module — active (0.91)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ deep-module-assess.j2: unknown hlexicon terms ['count', 'estimate']
- ⚠ deep-module-assess.j2: energy_cap nested under contract (spec says top-level)
- ⚠ deep-module-assess.j2: visibility nested under contract (spec says top-level)

### diagnose — active (0.92)
- Zed: ✓, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ diagnose-fix.j2: energy_cap nested under contract (spec says top-level)
- ⚠ diagnose-fix.j2: visibility nested under contract (spec says top-level)
- ⚠ diagnose-hypothesise.j2: energy_cap nested under contract (spec says top-level)

### document-update — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 7 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ doc-align-metadata.j2: missing [inference] frontmatter
- ⚠ doc-align-metadata.j2: invalid visibility None
- ⚠ doc-align-metadata.j2: missing/empty contract

### essentialist — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 1 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ essentialist-flow.j2: unknown hlexicon terms ['iterate', 'reduce']
- ⚠ essentialist-flow.j2: energy_cap nested under contract (spec says top-level)
- ⚠ essentialist-flow.j2: visibility nested under contract (spec says top-level)

### flowdef — critical (0.29)
- Zed: ✗, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ dispatch.j2: unknown hlexicon terms ['dispatch', 'invoke', 'tool', 'capability']

### gentle-lovelace — stale_warning (0.54)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ replica-report.j2: unknown hlexicon terms ['diagnose', 'compare']

### git — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### gml — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 10 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ assess-coherence.j2: missing [inference] frontmatter

### goal — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ create.j2: missing [inference] frontmatter

### grill-me — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ grill-me-assess.j2: energy_cap nested under contract (spec says top-level)
- ⚠ grill-me-assess.j2: visibility nested under contract (spec says top-level)
- ⚠ grill-me-escalate.j2: energy_cap nested under contract (spec says top-level)

### handoff — active (0.92)
- Zed: ✓, Registry: ✓
- Templates: 4 .j2 | WordAct=True KnowAct=True FlowDef=False
- ⚠ handoff-artifacts.j2: energy_cap nested under contract (spec says top-level)
- ⚠ handoff-artifacts.j2: visibility nested under contract (spec says top-level)
- ⚠ handoff-compact.j2: energy_cap nested under contract (spec says top-level)

### improv — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 6 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ improv-cycle.j2: missing [inference] frontmatter
- ⚠ improv-cycle.j2: invalid visibility None
- ⚠ improv-cycle.j2: missing/empty contract

### improve-codebase-architecture — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ arch-candidates.j2: energy_cap nested under contract (spec says top-level)
- ⚠ arch-candidates.j2: visibility nested under contract (spec says top-level)
- ⚠ arch-deepen.j2: energy_cap nested under contract (spec says top-level)

### inference — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### kata — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 7 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ consent-and-select.j2: missing [inference] frontmatter
- ⚠ consent-and-select.j2: invalid visibility None
- ⚠ consent-and-select.j2: missing/empty contract

### kata-coaching — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 6 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ coaching-cycle.j2: missing [inference] frontmatter
- ⚠ coaching-cycle.j2: invalid visibility None
- ⚠ coaching-cycle.j2: missing/empty contract

### kata-improvement — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ improvement-cycle.j2: missing [inference] frontmatter
- ⚠ improvement-cycle.j2: invalid visibility None
- ⚠ improvement-cycle.j2: missing/empty contract

### kata-starter — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ starter-cycle.j2: missing [inference] frontmatter
- ⚠ starter-cycle.j2: invalid visibility None
- ⚠ starter-cycle.j2: missing/empty contract

### knowact — recommend_deprecation (0.17)
- Zed: ✗, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ calibrate.j2: unknown hlexicon terms ['baseline', 'adjust', 'normalize', 'align']

### magna-carta-verifier — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ mc-verify-procedure.j2: energy_cap nested under contract (spec says top-level)
- ⚠ mc-verify-procedure.j2: visibility nested under contract (spec says top-level)
- ⚠ mc-verify-report.j2: energy_cap nested under contract (spec says top-level)

### mcda — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ identify-criteria.j2: unknown hlexicon terms ['identify', 'criteria', 'benefit', 'cost', 'independent', 'dimension']

### mcp — recommend_deprecation (0.05)
- Zed: ✗, Registry: ✓
- Templates: 5 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ condense_session.j2: unknown hlexicon terms ['condense', 'gist']

### media — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ classify_style.j2: missing [inference] frontmatter

### memory — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### metacognition — critical (0.46)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ meta_decompose.j2: unknown hlexicon terms ['subgoal', 'dependency', 'strategy', 'effort']

### pragmatic-cybernetics — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ cybernetics-analyze-loop.j2: energy_cap nested under contract (spec says top-level)
- ⚠ cybernetics-analyze-loop.j2: visibility nested under contract (spec says top-level)
- ⚠ cybernetics-variety-check.j2: energy_cap nested under contract (spec says top-level)

### pragmatic-laziness — active (0.85)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ pragmatic-laziness-converge.j2: unknown hlexicon terms ['compare']
- ⚠ pragmatic-laziness-converge.j2: energy_cap nested under contract (spec says top-level)
- ⚠ pragmatic-laziness-converge.j2: visibility nested under contract (spec says top-level)

### pragmatic-semantics — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ semantics-classify-statement.j2: energy_cap nested under contract (spec says top-level)
- ⚠ semantics-classify-statement.j2: visibility nested under contract (spec says top-level)
- ⚠ semantics-conflict-resolve.j2: energy_cap nested under contract (spec says top-level)

### pragmatics — stale_warning (0.75)
- Zed: ✓, Registry: ✗
- ⚠ missing registry layer

### prompt-defense — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ analyze-attack-surface.j2: unknown hlexicon terms ['surface', 'capability', 'tool', 'trust', 'boundary', 'privilege', 'escalation', 'blast']

### rag — recommend_deprecation (0.09)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ reformulate-query.j2: unknown hlexicon terms ['reformulate', 'expand', 'sub-question', 'intent']

### rca — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ corrective-action.j2: unknown hlexicon terms ['corrective', 'action', 'prevent', 'assign', 'owner', 'deadline', 'systemic']

### reasoning — critical (0.23)
- Zed: ✗, Registry: ✓
- Templates: 2 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ reason_constrained.j2: unknown hlexicon terms ['reason', 'diverge', 'evidence', 'constraint', 'tool']

### refactor-service-layer — active (0.91)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=True
- ⚠ rsl-audit.j2: energy_cap nested under contract (spec says top-level)
- ⚠ rsl-audit.j2: visibility nested under contract (spec says top-level)
- ⚠ rsl-strangle.j2: unknown hlexicon terms ['migrate']

### registry — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### replica — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ discovery-corpus.j2: missing [inference] frontmatter

### review — critical (0.49)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ self_critique.j2: unknown hlexicon terms ['contradiction', 'confidence', 'calibration']

### rust-expertise — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 7 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ rust-error-design.j2: energy_cap nested under contract (spec says top-level)
- ⚠ rust-error-design.j2: visibility nested under contract (spec says top-level)
- ⚠ rust-idiom-audit.j2: energy_cap nested under contract (spec says top-level)

### scenario — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ axes-and-narratives.j2: unknown hlexicon terms ['axis', 'narrative', 'scenario', 'diverge', 'plausible', 'consistent', '2x2', 'quadrant', 'story']

### self-critique-revision — recommend_deprecation (0.12)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ critique.j2: unknown hlexicon terms ['identify', 'weakness', 'improvement']

### skill-bundler — active (0.94)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ bundler-compose.j2: energy_cap nested under contract (spec says top-level)
- ⚠ bundler-compose.j2: visibility nested under contract (spec says top-level)
- ⚠ bundler-evolve.j2: energy_cap nested under contract (spec says top-level)

### skill-discovery — active (0.96)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ skill-discovery-detect-gap.j2: energy_cap nested under contract (spec says top-level)
- ⚠ skill-discovery-detect-gap.j2: visibility nested under contract (spec says top-level)
- ⚠ skill-discovery-evaluate.j2: energy_cap nested under contract (spec says top-level)

### skill-maintenance — active (0.96)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ skill-maintenance-audit.j2: energy_cap nested under contract (spec says top-level)
- ⚠ skill-maintenance-audit.j2: visibility nested under contract (spec says top-level)
- ⚠ skill-maintenance-coverage.j2: energy_cap nested under contract (spec says top-level)

### skill-manager — active (0.96)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ skill-manager-build.j2: energy_cap nested under contract (spec says top-level)
- ⚠ skill-manager-build.j2: visibility nested under contract (spec says top-level)
- ⚠ skill-manager-validate.j2: energy_cap nested under contract (spec says top-level)

### skill-translator — active (0.96)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ skill-translator-analyze.j2: energy_cap nested under contract (spec says top-level)
- ⚠ skill-translator-analyze.j2: visibility nested under contract (spec says top-level)
- ⚠ skill-translator-translate.j2: energy_cap nested under contract (spec says top-level)

### spec — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 6 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ constraint-bind.j2: missing [inference] frontmatter

### strangler-fig — active (0.91)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ strangler-fig-execute.j2: unknown hlexicon terms ['execute', 'wire', 'migrate']
- ⚠ strangler-fig-execute.j2: energy_cap nested under contract (spec says top-level)
- ⚠ strangler-fig-execute.j2: visibility nested under contract (spec says top-level)

### structured-extraction — recommend_deprecation (0.03)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ extract-relations.j2: unknown hlexicon terms ['identify', 'relate', 'connect', 'subject', 'predicate', 'object', 'link', 'context']

### superforecasting — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 8 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ stage_0_triage.j2: missing [inference] frontmatter

### tdd — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ tdd-gap-check.j2: energy_cap nested under contract (spec says top-level)
- ⚠ tdd-gap-check.j2: visibility nested under contract (spec says top-level)
- ⚠ tdd-plan.j2: energy_cap nested under contract (spec says top-level)

### templates — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### traceability-assurance — stale_warning (0.60)
- Zed: ✗, Registry: ✓
- Templates: 0 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml

### web — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ extract-synthesize.j2: missing [inference] frontmatter

### wordact — recommend_deprecation (0.01)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ execute.j2: unknown hlexicon terms ['execute', 'respond', 'complete', 'dispatch']

### zoom-out — active (0.98)
- Zed: ✓, Registry: ✓
- Templates: 1 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ zoom-out-context.j2: energy_cap nested under contract (spec says top-level)
- ⚠ zoom-out-context.j2: visibility nested under contract (spec says top-level)

