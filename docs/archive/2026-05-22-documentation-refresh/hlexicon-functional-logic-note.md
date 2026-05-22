# hLexicon Functional Logic — Design Note

**Version:** v0.21.4  
**Date:** 2026-05-21  
**Topic:** Separation of Functional vs Implementation Logic

---

## Key Insight

**Functional logic** (hLexicon categories) and **implementation logic** (file formats) are **orthogonal surfaces**:

| Dimension | Categories | Purpose |
|-----------|------------|---------|
| **Functional** | WordAct, FlowDef, KnowAct | What it IS semantically |
| **Implementation** | Jinja2, YAML manifest, YAML port | How it's BUILT technically |

---

## Orthogonal Mapping

```
                    Functional Logic
                    WordAct  FlowDef  KnowAct
                    ─────────────────────────
Implementation  │
Jinja2          │    1        3         5
YAML Manifest   │    0        2         0
YAML Port       │    3        0         3
```

---

## Definitions

### WordAct (Action-Word Hybrid)
**Functional role:** Generate messages, emit spans, record triples  
**Examples:**
- `habit-intervention.j2` — Generate intervention message
- `cns:emit:kata` port — Emit CNS span
- `memory:record:kata` port — Record memory triple

### FlowDef (Flow Definition)
**Functional role:** Orchestrate multi-step processes  
**Examples:**
- `improvement-cycle.j2` — 4-step Kata process
- `kata-pattern.yaml` — 5+3 step execution flow
- `kata-iteration.yaml` — Iteration workflow

### KnowAct (Knowledge-Action Hybrid)
**Functional role:** Make judgments, assessments, decisions  
**Examples:**
- `consent-and-select.j2` — Verify consent, select pattern
- `outcome-and-habit.j2` — Assess automaticity, streak
- `iteration-comparison.j2` — Judge variance, confidence
- `kata:execute` port — Authorization judgment

---

## Why Separation Matters

1. **Semantic clarity** — Know what each component DOES, not just how it's built
2. **Composability** — FlowDefs compose KnowActs and WordActs
3. **Testing strategy** — Different tests for each category:
   - WordAct: Verify output format
   - FlowDef: Verify step sequence
   - KnowAct: Verify decision accuracy

4. **Evolution** — Can change implementation without changing function:
   - Replace Jinja2 with LLM prompt (same functional role)
   - Replace YAML manifest with code (same functional role)

---

## hKask Kata System — Functional Distribution

| Category | Count | Percentage |
|----------|-------|------------|
| WordAct | 5 | 29% |
| FlowDef | 5 | 29% |
| KnowAct | 7 | 42% |
| **Total** | **17** | **100%** |

**Interpretation:**
- Heavy KnowAct bias (42%) — Kata is judgment-heavy (consent, assessment, variance)
- Balanced FlowDef (29%) — Clear process orchestration
- Light WordAct (29%) — Minimal action generation (messages, spans, triples)

---

## Implementation Independence Examples

**Same Function, Different Implementation:**

| Functional Role | Implementation A | Implementation B |
|-----------------|------------------|------------------|
| FlowDef | `kata-pattern.yaml` (manifest) | Python function `execute_kata()` |
| KnowAct | `consent-and-select.j2` (Jinja2) | Rust function `verify_consent()` |
| WordAct | `habit-intervention.j2` (Jinja2) | SQL INSERT statement |

**Same Implementation, Different Function:**

| Implementation | Function A | Function B |
|----------------|------------|------------|
| Jinja2 template | `improvement-cycle.j2` (FlowDef) | `consent-and-select.j2` (KnowAct) |
| YAML file | `kata-pattern.yaml` (FlowDef) | `kata-ports.yaml` (KnowAct + WordAct) |

---

## Design Principle

> **"Function dictates category. Implementation dictates format. Never conflate."**

A FlowDef remains a FlowDef whether implemented as:
- YAML manifest (current)
- Python function (alternative)
- Rust struct (alternative)
- BPMN diagram (alternative)

A KnowAct remains a KnowAct whether implemented as:
- Jinja2 prompt (current)
- LLM classifier (alternative)
- Rule engine (alternative)
- Decision tree (alternative)

---

*ℏKask — Design Note v0.21.4*
*Functional logic and implementation logic are orthogonal surfaces.*