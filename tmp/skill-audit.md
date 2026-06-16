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
| stale_warning | 12 |
| critical | 6 |
| recommend_deprecation | 25 |

## Skill Details

### adversarial-red-team — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ generate-adversarial.j2: unknown hlexicon terms ['generate', 'adversarial', 'inject', 'hijack', 'manipulate', 'exploit', 'attack', 'indirect', 'tool_misuse', 'exfiltrate']

### caveman — stale_warning (0.75)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)

### chain-of-density — critical (0.36)
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

### coding-guidelines — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

### condenser-continuation — active (0.91)
- Zed: ✓, Registry: ✓
- Templates: 4 .j2 | WordAct=True KnowAct=True FlowDef=True
- ⚠ condenser-continuation-restore.j2: unknown hlexicon terms ['restore']
- ⚠ condenser-continuation-verify.j2: unknown hlexicon terms ['check', 'test']

### constraint-forces — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False

### curator — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 6 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ metacognition-calibrate.j2: missing [inference] frontmatter

### dct-pipeline — critical (0.36)
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
- ⚠ deep-module-delete.j2: unknown hlexicon terms ['delete']

### diagnose — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=True FlowDef=False

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

### flowdef — critical (0.33)
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

### grill-me — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

### handoff — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 4 .j2 | WordAct=True KnowAct=True FlowDef=False

### improv — recommend_deprecation (0.00)
- Zed: ✓, Registry: ✓
- Templates: 6 .j2 | WordAct=False KnowAct=False FlowDef=False
- ⚠ improv-cycle.j2: missing [inference] frontmatter
- ⚠ improv-cycle.j2: invalid visibility None
- ⚠ improv-cycle.j2: missing/empty contract

### improve-codebase-architecture — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

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

### knowact — critical (0.27)
- Zed: ✗, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ calibrate.j2: unknown hlexicon terms ['baseline', 'adjust', 'normalize', 'align']

### magna-carta-verifier — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

### mcda — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ identify-criteria.j2: unknown hlexicon terms ['identify', 'criteria', 'benefit', 'cost', 'independent', 'dimension']

### mcp — recommend_deprecation (0.15)
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

### metacognition — critical (0.48)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ meta_decompose.j2: unknown hlexicon terms ['subgoal', 'dependency', 'strategy', 'effort']

### pragmatic-cybernetics — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

### pragmatic-laziness — active (0.85)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False
- ⚠ pragmatic-laziness-converge.j2: unknown hlexicon terms ['compare']
- ⚠ pragmatic-laziness-flow.j2: unknown hlexicon terms ['identify', 'eliminate', 'iterate', 'delegate']

### pragmatic-semantics — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

### pragmatics — stale_warning (0.75)
- Zed: ✓, Registry: ✗
- ⚠ missing registry layer

### prompt-defense — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ analyze-attack-surface.j2: unknown hlexicon terms ['surface', 'capability', 'tool', 'trust', 'boundary', 'privilege', 'escalation', 'blast']

### rag — recommend_deprecation (0.15)
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

### reasoning — critical (0.27)
- Zed: ✗, Registry: ✓
- Templates: 2 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ reason_constrained.j2: unknown hlexicon terms ['reason', 'diverge', 'evidence', 'constraint', 'tool']

### refactor-service-layer — active (0.97)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=True
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

### review — stale_warning (0.51)
- Zed: ✗, Registry: ✓
- Templates: 1 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ self_critique.j2: unknown hlexicon terms ['contradiction', 'confidence', 'calibration']

### rust-expertise — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 7 .j2 | WordAct=False KnowAct=True FlowDef=False

### scenario — recommend_deprecation (0.00)
- Zed: ✗, Registry: ✓
- Templates: 5 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ axes-and-narratives.j2: unknown hlexicon terms ['axis', 'narrative', 'scenario', 'diverge', 'plausible', 'consistent', '2x2', 'quadrant', 'story']

### self-critique-revision — recommend_deprecation (0.18)
- Zed: ✗, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=False FlowDef=True
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ critique.j2: unknown hlexicon terms ['identify', 'weakness', 'improvement']

### skill-bundler — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 3 .j2 | WordAct=False KnowAct=True FlowDef=False

### skill-discovery — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False

### skill-maintenance — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False

### skill-manager — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False

### skill-translator — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 2 .j2 | WordAct=False KnowAct=True FlowDef=False

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

### structured-extraction — recommend_deprecation (0.09)
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

### wordact — recommend_deprecation (0.07)
- Zed: ✗, Registry: ✓
- Templates: 4 .j2 | WordAct=True KnowAct=False FlowDef=False
- ⚠ missing Zed layer (SKILL.md)
- ⚠ missing manifest.yaml
- ⚠ execute.j2: unknown hlexicon terms ['execute', 'respond', 'complete', 'dispatch']

### zoom-out — active (1.00)
- Zed: ✓, Registry: ✓
- Templates: 1 .j2 | WordAct=False KnowAct=True FlowDef=False

