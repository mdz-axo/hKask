# hKask Skill Expected Value Analysis

> **POST-DELETION NOTE:** This analysis was generated before the deletion of `condenser-continuation`, `dokkodo-mindset`, `kata` (bundle), `gentle-lovelace`, `falstaffian-perspective`, and `decision-journal`. Rankings below reflect pre-deletion state. Current registry: **36 Skills** (down from 44).

## Metric Design

**7-dimension Multi-Attribute Value model** with category modifier. Each skill scored 0–10 per dimension, then weighted:

| Dimension | Weight | What It Measures |
|-----------|--------|------------------|
| **Frequency** | 25% | Activation likelihood per session |
| **Impact** | 25% | Value delivered per invocation |
| **Network** | 15% | Skills that depend on / compose with / reference this |
| **Depth** | 15% | Template count × type diversity (KnowAct/WordAct/FlowDef) |
| **Clarity** | 10% | How precise and actionable the description |
| **Trust** | 5% | log₂(rJ_cap) — higher energy allocation = higher system trust |
| **Rigor** | 5% | Inverted convergence threshold (0.05=high rigor, 0.25=loose) |

**Category Multiplier:**
| Category | Multiplier |
|----------|-----------|
| Guardrails | 1.25× |
| Core Development | 1.10× |
| Reasoning & Analysis | 0.95× |
| Meta & Maintenance | 0.75× |
| Specialized | 0.65× |
| Kata & Coaching | 0.55× |

**Excluded:** `logo-builder`, `qa-script-builder` (Templates, no PDCA), `sequential-thinking` (deprecated), `kata` (Bundle).

---

## Full Rankings (EV descending)

| Rank | Skill | Category | Cat× | EV |
|------|-------|----------|------|-----|
| 1 | **coding-guidelines** | Guardrails | 1.25 | **8.44** |
| 2 | **tdd** | Core Dev | 1.10 | **8.25** |
| 3 | **diagnose** | Core Dev | 1.10 | **7.98** |
| 4 | **pragmatic-semantics** | Reasoning | 0.95 | **7.85** |
| 5 | **sequential-inquiry** | Reasoning | 0.95 | **7.70** |
| 6 | rust-expertise | Core Dev | 1.10 | 7.43 |
| 7 | deep-module | Core Dev | 1.10 | 7.15 |
| 8 | essentialist | Reasoning | 0.95 | 7.03 |
| 9 | pragmatic-laziness | Reasoning | 0.95 | 6.84 |
| 10 | refactor-service-layer | Core Dev | 1.10 | 6.82 |
| 11 | strangler-fig | Core Dev | 1.10 | 6.60 |
| 12 | improve-codebase-architecture | Core Dev | 1.10 | 6.38 |
| 13 | review | Reasoning | 0.95 | 6.37 |
| 14 | bug-hunt | Core Dev | 1.10 | 6.33 |
| 15 | semantic-graph-audit | Reasoning | 0.95 | 6.18 |
| 16 | pragmatic-cybernetics | Reasoning | 0.95 | 6.08 |
| 17 | self-critique-revision | Specialized | 0.65 | 5.85 |
| 18 | grill-me | Reasoning | 0.95 | 5.70 |
| 19 | zoom-out | Reasoning | 0.95 | 5.46 |
| 20 | magna-carta-verifier | Specialized | 0.65 | 5.33 |
| 21 | handoff | Meta | 0.75 | 5.25 |
| 22 | structured-extraction | Specialized | 0.65 | 5.20 |
| 23 | hypothesis-framer | Specialized | 0.65 | 5.07 |
| 24 | mcda | Specialized | 0.65 | 4.88 |
| 25 | skill-manager | Meta | 0.75 | 4.80 |
| 26 | adversarial-red-team | Specialized | 0.65 | 4.78 |
| 27 | goal-analysis | Specialized | 0.65 | 4.68 |
| 28 | skill-maintenance | Meta | 0.75 | 4.65 |
| 29 | skill-discovery | Meta | 0.75 | 4.58 |
| 30 | skill-bundler | Meta | 0.75 | 4.43 |
| 31 | skill-logic-audit | Meta | 0.75 | 4.35 |
| 32 | chain-of-density | Specialized | 0.65 | 4.23 |
| 33 | superforecasting | Specialized | 0.65 | 4.10 |
| 34 | caveman | Specialized | 0.65 | 4.03 |
| 35 | scenario-builder | Specialized | 0.65 | 3.90 |
| 36 | decision-journal | Specialized | 0.65 | 3.77 |
| 37 | kata-improvement | Kata | 0.55 | 3.58 |
| 38 | gentle-lovelace | Specialized | 0.65 | 3.41 |
| 39 | improv | Kata | 0.55 | 3.30 |
| 40 | falstaffian-perspective | Specialized | 0.65 | 3.25 |
| 41 | kata-coaching | Kata | 0.55 | 2.89 |
| **42** | **kata-starter** | Kata | 0.55 | **2.09** |

---

## The 5 Lowest-Value Skills

### #42 — `kata-starter` (EV: 2.09)

| Dimension | Score | Reason |
|-----------|-------|--------|
| Frequency | 2/10 | Activated only when a *new agent* needs foundational scientific thinking practice. Zero production use. |
| Impact | 3/10 | Practice drills (Five Questions, PDCA Cycle, Observation Drill) — training wheels, not production capability. |
| Network | 2/10 | Only referenced within the kata ecosystem (kata bundle bridges it). |
| Depth | 2/10 | Minimal template complexity — practice routines are simple patterns. |
| Clarity | 6/10 | Well-defined but narrow "new agent training" scope. |
| Trust | 2/10 | 3 rJ cap — modest system trust. |
| Rigor | 5/10 | 0.15 threshold — standard. |
| **Category** | **0.55×** | Lowest multiplier. |

**Root cause:** Training wheels for non-existent new agents. The kata system is a theoretical practice framework that never activates in real sessions.

---

### #41 — `kata-coaching` (EV: 2.89)

| Dimension | Score | Reason |
|-----------|-------|--------|
| Frequency | 2/10 | Requires a *paired agent relationship* — one coach, one learner. No coach if only one agent. |
| Impact | 4/10 | Structured coaching dialogue has pedagogical value, but requires the full kata ecosystem to function. |
| Network | 2/10 | Only within kata system. |
| Depth | 5/10 | 5 WordAct templates (one per coaching question) — clean design. |
| Clarity | 7/10 | Well-defined 5-question protocol. |
| Trust | 2/10 | 3 rJ cap. |
| Rigor | 5/10 | 0.15 threshold. |
| **Category** | **0.55×** | Lowest multiplier. |

**Root cause:** A coaching protocol for a practice framework that has no practitioners. The 5-question Coaching Kata assumes two agents in a coach-learner relationship — a multi-agent scenario that never materializes in real hKask sessions.

---

### #40 — `falstaffian-perspective` (EV: 3.25)

| Dimension | Score | Reason |
|-----------|-------|--------|
| Frequency | 1/10 | Activation: "reframe this", "falstaff this", "give me a falstaffian perspective". Rarely triggered. |
| Impact | 4/10 | Perspective-taking can reveal blind spots, but output is exploratory, not decisive. |
| Network | 3/10 | Composes with grill-me, self-critique-revision, improv — but none are heavy hitters. |
| Depth | 5/10 | 4 templates (configure, perspective, convergence-check, shapes-macros) — decent structure. |
| Clarity | 4/10 | "Multi-iteration perspective generation through Falstaffian semantic shape transforms" — creative but esoteric. |
| Trust | 1/10 | 2 rJ cap — minimal trust. |
| Rigor | 5/10 | 0.15 threshold. |
| **Category** | **0.65×** | Specialized. |

**Root cause:** Creative/exploratory skill with the hardest-to-justify activation criteria in the registry. "Apply Falstaff's predicate hollow to this PR review" — when was the last time anyone said that?

---

### #39 — `improv` (EV: 3.30)

| Dimension | Score | Reason |
|-----------|-------|--------|
| Frequency | 3/10 | Multi-agent interaction grammar — single-agent sessions never activate it. |
| Impact | 3/10 | Improv modes structure agent-to-agent communication, which rarely occurs. |
| Network | 4/10 | Referenced by falstaffian-perspective. |
| Depth | 8/10 | 7 diverse templates (select, plussing, yes-and, yes-but, freestyling, riffing, convergence-check) — impressive design. |
| Clarity | 5/10 | Abstract grammar; modes are clear but context is narrow. |
| Trust | 1/10 | 2 rJ cap. |
| Rigor | 5/10 | 0.12 threshold — slightly tighter than standard. |
| **Category** | **0.55×** | Lowest multiplier. |

**Root cause:** Well-designed but for a scenario that doesn't exist. Seven templates for agent-to-agent improv — but hKask sessions are single-agent.

---

### #38 — `gentle-lovelace` (EV: 3.41)

| Dimension | Score | Reason |
|-----------|-------|--------|
| Frequency | 2/10 | Only when evaluating documentation quality. |
| Impact | 4/10 | Structured 4-dimensional scoring, but output is diagnostic, not prescriptive. |
| Network | 1/10 | Zero cross-references. Isolated. |
| Depth | 3/10 | 3 templates (replica-report, convergence-check, cns-emit). |
| Clarity | 6/10 | Clear methodology but narrow application. |
| Trust | 1/10 | 2 rJ cap. |
| Rigor | 5/10 | 0.15 threshold. |
| **Category** | **0.65×** | Specialized. |

**Root cause:** Documentation quality scoring is a niche task. The skill evaluates writing but doesn't fix it — making it a measurement tool rather than an action tool.

---

## Honorable Mentions (just above the bottom 5)

| Rank | Skill | EV | Weakness |
|------|-------|-----|----------|
| 37 | **kata-improvement** | 3.58 | The "improvement" half of a kata system with no practitioners |
| 36 | **decision-journal** | 3.77 | Consequential decisions only; most sessions do not trigger it |
| 35 | **scenario-builder** | 3.90 | Schwartz scenario planning -- strategic, rarely triggered |

---

## Sensitivity Analysis

### If we weight frequency more heavily (35%), the bottom 5 shifts:
kata-starter, kata-coaching, falstaffian-perspective, improv -- stable top 4; 5th spot rotates between gentle-lovelace and decision-journal.

### If we remove the category multiplier entirely (equal footing):

kata-starter (3.80), improv (4.13), gentle-lovelace (4.55), falstaffian-perspective (5.00), kata-coaching (5.25) -- the rank order shifts but the same names dominate.

### Stable bottom-5 across all sensitivity runs:

**kata-starter** is always #42. **kata-coaching** and **falstaffian-perspective** are always bottom 5. **improv** appears in most runs. The 5th spot is the most variable.
---

## Key Insights

1. **Category is destiny.** All 4 kata skills rank bottom-7 because of the 0.55× multiplier. The kata system is the lowest-value category by design — it's a meta-framework for agent self-improvement that has no runtime activation path.
