---
name: pragmatic-laziness
visibility: public
description: Procedural composition skill. Finds the path of least action through meaning-space by describing the intersection of pragmatic-semantics, pragmatic-cybernetics, essentialist, and grill-me through a 3-phase lazy loop. Activate when the user says "be lazy about this", "find the lazy path", "least action", "decompose this", or "pragmatic laziness review".
composes_skills: [pragmatic-semantics, pragmatic-cybernetics, essentialist, grill-me]
---

# Pragmatic Laziness

A procedural composition skill. Finds the path of least action through meaning-space by covering the intersection of pragmatic-semantics, pragmatic-cybernetics, essentialist, and grill-me through a 3-phase lazy loop. Does not teach — it describes. Does not add — it maps.

**Governing principle:** *The universe is lazy. Be lazier.* Every artifact, step, and abstraction must justify itself against: "Does this reduce total system action, or does it add friction?"

## Composition Contract

| Property | Value |
|----------|-------|
| **Polarity** | Procedural (describes discipline intersection; backbone) |
| **Phase** | Core (can span pre→core→post when used as primary process) |
| **Draws on** | `pragmatic-semantics`, `pragmatic-cybernetics`, `essentialist`, `grill-me` |
| **Regulative across phases** | `pragmatic-semantics` |
| **Convergence** | δS = 0 — no further action reduction on repeat pass |
| **Max iterations** | 3, then escalate |

**Note on composition edges:** The "Draws on" skills describe the *methodology* applied within the lazy-loop phases. As of v0.31.0, the monolithic `pragmatic-laziness-flow.j2` has been decomposed into three phase-level KnowAct templates (`lazy-decompose.j2`, `lazy-identify-loops.j2`, `lazy-stationary-action.j2`) wired as separate PDCA steps in the manifest. Each phase receives structured outputs from the prior phase, giving the PDCA engine per-phase observability. The composition with pragmatic-semantics, pragmatic-cybernetics, essentialist, and grill-me remains methodological guidance (not machine-executable template delegation).

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "be lazy about this" / "find the lazy path" / "least action" | Full lazy loop |
| "what's the simplest way?" / "minimum viable approach" | Phase 3 only (find stationary action) |
| "decompose this" / "separate syntax from semantics" | Phase 1 only (Morris's triad) |
| "what feedback loops are driving this?" | Phase 2 only (identify loops) |
| "pragmatic laziness review" | Full lazy loop |

## The Lazy Loop

```
┌─────────────────────────────────────────────┐
│ PHASE 1: DECOMPOSE                           │
│ Activate: pragmatic-semantics                │
│                                              │
│ Separate syntax (structure) from semantics   │
│ (literal meaning) from pragmatics (context-  │
│ dependent intent). Classify each statement   │
│ by epistemic mode.                           │
│                                              │
│ Output: classified decomposition             │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ PHASE 2: IDENTIFY LOOPS                      │
│ Activate: pragmatic-cybernetics              │
│                                              │
│ Map feedback loops (OODA, cybernetic, REPL). │
│ Check closure, fidelity, gain, delay.        │
│ Locate effort hotspots.                      │
│                                              │
│ Output: loop map with effort distribution    │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ PHASE 3: FIND STATIONARY ACTION              │
│ Activate: essentialist → grill-me            │
│                                              │
│ Apply deletion test. Find the brachistochrone│
│ — the path that may look longer but reduces  │
│ total system action.                         │
│ Stress-test: does solution survive small     │
│ perturbations? (δS = 0 check)                │
│                                              │
│ Output: minimal configuration or escalation │
└────────────────────┬────────────────────────┘
                     ▼
              δS = 0? ──No──→ repeat (max 3)
                │
               Yes
                │
               Done
```

**Pragmatic-semantics** runs across all phases: Prohibitions and Guardrails are never relaxed in pursuit of least action. Guidelines may be relaxed with reason stated.

## The Brachistochrone Rule

The laziest path is not always the most obvious one. The curve of fastest descent is a cycloid — it dips below the endpoint before rising. Naive simplification (just deleting code) is not always the true least-action path. Sometimes you must go *through* apparent complexity to extract the deeper pattern that ultimately reduces total system action. When Phase 3 finds a candidate that looks more complex than the status quo, ask: *"Does this reduce total system action across all phases, or just shift it elsewhere?"*

## Registry Templates
This skill's runtime templates live in `registry/templates/pragmatic-laziness/`:

| Template | Type | Purpose |
|----------|------|--------|
| `lazy-decompose.j2` | KnowAct | Phase 1: Decompose into syntax/semantics/pragmatics via Morris's Triad |
| `lazy-identify-loops.j2` | KnowAct | Phase 2: Identify feedback loops and effort hotspots |
| `lazy-stationary-action.j2` | KnowAct | Phase 3: Apply deletion test + brachistochrone rule to find least-action configuration |
| `pragmatic-laziness-converge.j2` | KnowAct | δS = 0 stationarity check between iterations |

Convergence metering delegates to `shared/convergence-check.j2`.

## When to Use This Skill

- **Before building anything:** Run the lazy loop. Does the thing earn its existence against the deletion test?
- **When a design feels heavy:** Decompose (Phase 1), find the loops driving the weight (Phase 2), eliminate (Phase 3).
- **When two approaches conflict:** Find which path minimizes total system action — not just local action in one module.
- **When essentialist gets stuck:** The brachistochrone rule explains why some complexity is transitional, not terminal.
- **Invoked from other skills:** `coding-guidelines` can call Phase 3 when "simplicity first" needs rigorous verification. `essentialist` can call the full loop when deletion test results are ambiguous.

## Quick Reference

1. **Decompose** (pragmatic-semantics): syntax / semantics / pragmatics. What's structure, what's meaning, what's context-dependent intent?
2. **Identify loops** (pragmatic-cybernetics): What OODA/cybernetic/REPL loops are operating? Where is effort spent?
3. **Find stationary action** (essentialist + grill-me): Delete what doesn't earn existence. Verify what remains survives perturbation.
4. **Repeat** until δS = 0. Max 3 iterations.
5. **Never relax** Prohibitions or Guardrails (`pragmatic-semantics`). Laziness respects boundaries.

*"Don't just do something, stand there."* — PRINCIPLES.md §0


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/pragmatic-laziness.yaml`

### PDCA Convergence
- **Threshold:** 0.25 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = stable least-action recommendation

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ (manifest `rjoule.cap` — see `registry/manifests/pragmatic-laziness.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
