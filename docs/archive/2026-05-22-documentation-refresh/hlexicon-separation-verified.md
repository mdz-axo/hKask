# hLexicon Separation Verification — Kata System

**Version:** v0.21.4  
**Date:** 2026-05-21  
**Status:** ✅ Verified

---

## Separation Confirmed

**Functional logic** (WordAct/FlowDef/KnowAct) and **implementation logic** (Jinja2/YAML) are **orthogonal surfaces** in the Kata system.

---

## Functional Distribution (17 total)

| Category | Count | Percentage | Purpose |
|----------|-------|------------|---------|
| **WordAct** | 5 | 29% | Action-word hybrids (generate messages, emit spans, record triples) |
| **FlowDef** | 5 | 29% | Flow definitions (orchestrate multi-step processes) |
| **KnowAct** | 7 | 42% | Knowledge-action hybrids (judgments, assessments, decisions) |

---

## Implementation Distribution (12 files)

| Format | Count | Used For |
|--------|-------|----------|
| **Jinja2 templates** | 9 | WordAct (1), FlowDef (3), KnowAct (5) |
| **YAML manifests** | 2 | FlowDef (2) |
| **YAML ports** | 1 | WordAct (3 ports), KnowAct (3 ports) |

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
                    ─────────────────────────
Total           │    5        5         7
```

---

## Functional Logic Registry

**File:** `registry/registries/kata/kata-hlexicon.yaml`

**WordAct (5):**
1. `habit-intervention.j2` — Generate intervention message
2. `cns:emit:kata` port — Emit CNS span
3. `memory:record:kata` port — Record memory triple
4. `ensemble:report:kata` port — Report to Curator
5. `kata:state:save` port — Save state for switching

**FlowDef (5):**
1. `improvement-cycle.j2` — 4-step Improvement Kata
2. `coaching-cycle.j2` — 5-question Coaching Kata
3. `starter-cycle.j2` — Practice routine flow
4. `kata-pattern.yaml` — 5+3 step execution flow
5. `kata-iteration.yaml` — Iteration workflow

**KnowAct (7):**
1. `consent-and-select.j2` — Verify consent, select pattern
2. `outcome-and-habit.j2` — Assess automaticity, streak
3. `kata-switch-check.j2` — Check switch decision
4. `iteration-comparison.j2` — Judge variance, confidence
5. `iteration-check.j2` — Decide if iteration needed
6. `kata:execute` port — Authorization judgment
7. `kata:switch` port — Transition authorization

---

## Design Principle Verified

> **"Function dictates category. Implementation dictates format. Never conflate."**

**Evidence:**
- Same implementation (Jinja2) serves 3 functional roles (WordAct, FlowDef, KnowAct)
- Same functional role (FlowDef) implemented 2 ways (Jinja2, YAML manifest)
- Port specs mix WordAct and KnowAct in same YAML file

**This confirms:** Functional logic and implementation logic are independent surfaces.

---

## Files Created

1. `registry/registries/kata/kata-hlexicon.yaml` — Functional logic registry
2. `docs/architecture/hlexicon-functional-logic-note.md` — Design note
3. `docs/architecture/hlexicon-separation-verified.md` — This verification

---

*ℏKask — Toyota Kata System v0.21.4*
*hLexicon separation verified. Functional and implementation logic are orthogonal.*