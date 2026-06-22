---
title: "Bug Hunting Skill — Design, Theory, and Implementation"
version: "0.30.0"
date: 2026-06-22
status: "Active — Implemented"
audience: [architects, QA practitioners, agents]
mds_categories: [domain, composition, trust, lifecycle]
---

# Bug Hunting Skill — Design, Theory, and Implementation

**Status:** Implemented as `.agents/skills/bug-hunt/SKILL.md`. Active in agent repertoire.

> **Incorporates:** `bug-hunting-as-autopoietic-skill-unified.md`, `bug-hunting-skill-corrected-design.md`, `bug-hunting-skill-implementation-plan.md`

---

## 1. Theoretical Foundation

The bug-hunt skill synthesizes two theoretical frameworks:

### 1.1 QA Canon
Grounded in Weinberg's quality definition (quality is value to some person), Beizer's bug taxonomy, Bach's Heuristic Test Strategy Model, and Hendrickson's exploratory testing charters. The skill applies structured exploratory testing through MCP tool probes, CNS-observed invariants, and LLM-powered triage.

### 1.2 Second-Order Cybernetics
Autopoietic (self-producing) testing: the system observes its own behavior through CNS spans, identifies deviations from expected invariants, and feeds findings back through the Curator's regulatory loop. The testing system and the tested system are the same system — this is second-order cybernetics in the Maturana/Varela tradition.

---

## 2. Design

The bug-hunt skill operates through hKask's existing machinery:

| Component | Role | Implementation |
|-----------|------|----------------|
| **CNS spans** | Observability | `cns.qa.*` spans track bug detection and triage |
| **OCAP delegation** | Access control | Tool dispatch gated by capability tokens |
| **Property-based testing** | Invariant verification | Proptest + bolero fuzz targets |
| **LLM triage** | Classification | `hkask-services-classify` → Gemma 4 diagnosis |
| **Curator feedback** | Loop closure | Findings → algedonic escalation → human review |

### Template Architecture (≤7 public templates, P5 Essentialism)

| Template | Type | Purpose |
|----------|------|----------|
| `bug-hunt-expedition` | FlowDef | Orchestrates a bug hunting expedition |
| `bug-hunt-classify` | KnowAct | Classifies findings by Beizer taxonomy |
| `bug-hunt-triage` | KnowAct | Routes findings by severity/confidence |
| `bug-hunt-report` | KnowAct | Produces structured bug reports |

---

## 3. Implementation

The bug-hunt skill is a `.agents/skills/bug-hunt/SKILL.md` with 4 Jinja2 templates. It integrates with:

- **Fuzz targets** (10 crates via bolero)
- **Mutation analysis** (cargo-mutants)
- **LLM triage** (`kask qa triage` → auto-repair/issue/feedback)
- **CNS homeostatic loop** (algedonic alert on quality deficit)

### Falsifiability Criterion
`CnsHealth.overall_deficit` must monotonically decrease across sessions as bugs are found and fixed.

---

## 4. References

- Weinberg, G. (1992). *Quality Software Management.*
- Beizer, B. (1990). *Software Testing Techniques.*
- Bach, J. (2004). "Heuristic Test Strategy Model."
- Maturana, H. & Varela, F. (1980). *Autopoiesis and Cognition.*
- `docs/architecture/core/TESTING_DISCIPLINE.md` — Testing methodology
- `.agents/skills/bug-hunt/SKILL.md` — Runtime skill definition
