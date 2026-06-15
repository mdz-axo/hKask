---
name: pragmatics
visibility: public
description: "Meta-cognitive codebase review bundle. Composes pragmatic-semantics, pragmatic-cybernetics, pragmatic-laziness, essentialist, and coding-guidelines into a unified architecture analysis and refactoring discipline. Use when reviewing codebase patterns, analyzing architecture against principles, auditing for principle violations, or when the user says 'pragmatics review', 'analyze the codebase', 'audit architecture', or 'apply the principles'."
---

# Pragmatics — Meta-Cognitive Codebase Review Bundle

A composed skill bundle for disciplined architecture analysis grounded in hKask's twelve principles (P1–P12). Orchestrates five skills through a 4-phase cascade: Decompose → Map Loops → Find Stationary Action → Constrain Execution.

## Bundle Manifest

```yaml
bundle:
  name: pragmatics
  version: 1.0.0
  description: "Meta-cognitive codebase review — architecture analysis against hKask principles"
  skills:
    - name: pragmatic-semantics
      polarity: evaluative
      phase: pre-core
      cascade_order: 1
      role: "Decompose problem space — classify IS/OUGHT, epistemic modes, provenance"
    - name: pragmatic-cybernetics
      polarity: evaluative
      phase: pre-core
      cascade_order: 2
      role: "Map feedback loops — VSM S1–S5, variety balance, Good Regulator check"
    - name: pragmatic-laziness
      polarity: procedural
      phase: core
      cascade_order: 3
      role: "Find stationary action — 3-phase lazy loop, brachistochrone, δS = 0"
    - name: essentialist
      polarity: evaluative
      phase: post-core
      cascade_order: 4
      role: "Validate minimalism — G1 Exist, G2 Surface, G3 Contract deletion test"
    - name: coding-guidelines
      polarity: regulative
      phase: across-phases
      cascade_order: 5
      role: "Constrain execution — think first, surgical, simple, goal-driven"
  conflicts: []
  complementarities:
    - skills: [pragmatic-semantics, pragmatic-cybernetics]
      leveraged: "Semantics classifies what we know (IS); cybernetics maps how the system regulates itself (feedback loops). Together they produce a complete diagnostic picture: what is true + what is healthy."
    - skills: [pragmatic-cybernetics, pragmatic-laziness]
      leveraged: "Cybernetics identifies effort hotspots in feedback loops; laziness finds the path of least action through those hotspots. The brachistochrone is a cybernetic concept — the path that minimizes control action."
    - skills: [pragmatic-laziness, essentialist]
      leveraged: "Laziness finds the minimal configuration; essentialist validates it survives the 3-gate deletion test. Laziness proposes; essentialist disposes."
    - skills: [coding-guidelines, all]
      leveraged: "Every change proposed by the analysis must survive coding-guidelines: think before acting, keep it simple, touch only what's needed, verify with tests."
  convergence:
    criterion: "δS = 0 (no further action reduction) AND zero principle violations (P1–P12) AND zero coding-guidelines anti-patterns in any proposed diff"
```

## The 4-Phase Cascade

```
┌─────────────────────────────────────────────────────────────┐
│ PHASE 1: DECOMPOSE (pragmatic-semantics)                     │
│                                                              │
│ Classify every architectural claim:                          │
│ • IS vs OUGHT — what is measured vs what should be          │
│ • Epistemic mode — declarative, probabilistic, subjunctive   │
│ • Provenance — directly stated, implicit, inherited          │
│ • Constraint force — Prohibition, Guardrail, Guideline,      │
│   Evidence, Hypothesis                                       │
│                                                              │
│ Output: Classified decomposition of codebase state           │
└────────────────────┬────────────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 2: MAP LOOPS (pragmatic-cybernetics)                   │
│                                                              │
│ Map the system cybernetically:                               │
│ • VSM S1–S5 — are all five systems present and viable?      │
│ • Feedback loops — polarity, delay, gain, closure, fidelity │
│ • Variety balance — regulator_variety >= system_variety?    │
│ • Good Regulator — does the CNS model match reality?         │
│ • Channel capacity — context window as Shannon limit         │
│                                                              │
│ Output: Loop map with effort hotspots and variety gaps       │
└────────────────────┬────────────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 3: FIND STATIONARY ACTION (pragmatic-laziness)         │
│                                                              │
│ Apply the lazy loop:                                         │
│ • Deletion test — what vanishes if removed?                 │
│ • Brachistochrone — the path that looks longer but reduces   │
│   total system action                                        │
│ • δS = 0 check — is the configuration at a stationary point?│
│ • Principle alignment — does the path respect P1–P12?       │
│                                                              │
│ Output: Minimal configuration or escalation                  │
└────────────────────┬────────────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 4: VALIDATE (essentialist)                              │
│                                                              │
│ Apply the 3-gate eliminative review to every artifact:       │
│ • G1 EXIST — does behavior vanish if deleted?               │
│ • G2 SURFACE — ≤ 7 public items? Extras justified?          │
│ • G3 CONTRACT — any pass-through abstractions?              │
│                                                              │
│ Loop until zero deltas on repeat pass.                       │
│                                                              │
│ Output: Elimination report with essentialism score           │
└─────────────────────────────────────────────────────────────┘

coding-guidelines runs across ALL phases:
• Think Before Coding — surface assumptions before acting
• Simplicity First — no speculative features
• Surgical Changes — touch only what must change
• Goal-Driven Execution — define success criteria, loop until verified
```

## Principle-to-Phase Mapping

Each phase checks specific principles from PRINCIPLES.md:

| Phase | Principles Checked | How |
|-------|-------------------|-----|
| 1. Decompose | P8 (Semantic Grounding) | Classify every claim by epistemic mode and provenance |
| 1. Decompose | P1–P4 (Magna Carta) | Identify Prohibitions — what must never be violated |
| 2. Map Loops | P9 (Homeostatic Self-Regulation) | VSM mapping, feedback loop analysis, Good Regulator check |
| 2. Map Loops | P4 (Clear Boundaries) | Are OCAP boundaries correctly placed in feedback loops? |
| 3. Find Stationary | P5 (Essentialism & Minimalism) | Least action principle — δS = 0, brachistochrone |
| 3. Find Stationary | P7 (Evolutionary Architecture) | Does the path follow gradient descent from actual usage? |
| 4. Validate | P5 (Essentialism) | G1–G3 deletion test on every artifact |
| 4. Validate | P6 (Space for Replicants) | Does the architecture enable agent emergence? |
| Across | P3 (Generative Space) | Are all settings exposed? No hidden parameters? |
| Across | P12 (Replicant Host Mandate) | Does every action have an author? |

## Usage Patterns

### Full Architecture Review

```
User: "pragmatics review the codebase"
→ Run all 4 phases against the entire crate graph
→ Produce: classified decomposition, loop map, stationary action recommendations, elimination report
```

### Targeted Principle Audit

```
User: "audit P5 compliance" or "check essentialism across crates"
→ Phase 3 + Phase 4 only
→ Focus on deletion test, surface counts, abstraction traces
```

### Feedback Loop Diagnosis

```
User: "diagnose the CNS feedback loop" or "check variety balance"
→ Phase 1 + Phase 2 only
→ Classify CNS state, map feedback loops, check Good Regulator
```

### Pre-Refactor Analysis

```
User: "analyze before extracting X"
→ All 4 phases, scoped to the affected crate subgraph
→ Produce: what to move, what to delete, what to preserve
```

## Quick Reference

### When to Activate

| User says | Action |
|-----------|--------|
| "pragmatics review" / "analyze the codebase" / "audit architecture" | Full 4-phase cascade |
| "apply the principles" / "check principle compliance" | Full cascade with principle traceability matrix |
| "audit P5" / "check essentialism" | Phase 3 + 4 only |
| "diagnose feedback loops" / "check CNS health" | Phase 1 + 2 only |
| "find the lazy path for X" | Phase 3 only |
| "validate this module" / "essentialist review" | Phase 4 only |

### Principle Traceability Quick Card

| Principle | Force | Check |
|-----------|-------|-------|
| P1 — User Sovereignty | Prohibition | Data ownership, consent atomicity, portability |
| P2 — Affirmative Consent | Prohibition | Default-deny, scoped/versioned/expiring consent |
| P3 — Generative Space | Prohibition | All settings exposed, no hidden params, no admin gate |
| P4 — Clear Boundaries (OCAP) | Prohibition | Dual gate, unforgeable tokens, no admin bypass |
| P5 — Essentialism | Guardrail | Deletion test, ≤7 public items, no pass-throughs |
| P6 — Space for Replicants | Guideline | Pod isolation, A2A/H2A modes, WebID identity |
| P7 — Evolutionary Architecture | Guardrail | Types from usage, convergence of divergent impls |
| P8 — Semantic Grounding | Guardrail | IS/OUGHT, epistemic mode, provenance on every claim |
| P9 — Homeostatic Self-Regulation | Guardrail | VSM S1–S5, feedback closure, variety balance |
| P10 — Bot/Replicant Taxonomy | Guardrail | Distinct types, persona, ACP mode, cadence |
| P11 — Digital Public/Private Sphere | Guardrail | Visibility gating, default private, consent for public |
| P12 — Replicant Host Mandate | Prohibition | Every action has author, no anonymous agency |

### Anti-Pattern Detection (from PRINCIPLES.md §5)

During any phase, flag these immediately:
- Visual UI / dashboards / Grafana / Prometheus (P1.6 violation)
- `todo!()`, `unimplemented!()`, `#[deprecated]` (P5 violation)
- Pass-through abstractions (P5 violation — essentialist G3)
- Broken feedback closure (P9 violation — cybernetics)
- Missing provenance on claims (P8 violation — semantics)
- Stubs that deny generative space (P3 + P5 violation)
- Anonymous agency / missing host (P12 violation)
