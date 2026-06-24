---
name: mcda
visibility: public
description: Multi-Criteria Decision Analysis. Identifies decision criteria, weights and scores alternatives, ranks options with compensation masking detection, and performs sensitivity analysis to assess decision robustness. Use when choosing between multiple alternatives on multiple criteria, when you need structured decision rationale to defend, or when you suspect a "cheap but dangerous" option is hiding behind a single strong score.
activation: "compare these options"
---

# MCDA — Multi-Criteria Decision Analysis

Structured decision support for choosing between alternatives evaluated on multiple criteria. Identifies criteria (benefits and costs), weights them, scores alternatives, ranks by composite — and critically, detects **compensation masking**: when strong performance on one criterion hides dangerous weakness on another.

## Why MCDA?

The human mind is bad at multi-criteria decisions. We latch onto one dimension ("it's the cheapest!") and rationalize the rest, or oscillate between criteria without converging. MCDA makes the tradeoffs explicit by naming criteria, weighting them, scoring per-criterion, and stress-testing the ranking through sensitivity analysis.

The output is not "the right answer." It's a structured rationale you can defend, revisit, and calibrate — with explicit warnings when a single strong score is masking a critical weakness.

## The Four-Stage Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│ STAGE 1: IDENTIFY CRITERIA                                   │
│                                                              │
│ Enumerate all relevant decision criteria.                    │
│ Classify each as BENEFIT (more is better) or COST            │
│ (less is better). Check independence — no double-counting.   │
│                                                              │
│ Output: criteria[] (name, type, direction, definition)       │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 2: WEIGHT AND SCORE                                    │
│                                                              │
│ Weight criteria (direct or swing method).                    │
│ Score each alternative on each criterion (0–100).            │
│ Normalize scores and compute weighted composites.            │
│                                                              │
│ Output: weights[], scores[][], composites[]                  │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 3: RANK WITH COMPENSATION MASKING DETECTION            │
│                                                              │
│ Rank alternatives by composite score.                        │
│ DETECT COMPENSATION MASKING: an alternative with a high      │
│ composite but at least one criterion below danger threshold   │
│ gets flagged. "Option A scores 88 overall but only 15 on     │
│ safety. This is compensation masking."                       │
│                                                              │
│ Output: rankings[], compensation_warnings[], top_choice      │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 4: SENSITIVITY ANALYSIS                                │
│                                                              │
│ Perturb weights: what if each criterion were ±10%?           │
│ Identify rank reversals — does the top choice change?        │
│ Find critical weights — the value at which ranking flips.    │
│ Classify robustness: robust / moderate / fragile             │
│                                                              │
│ Output: sensitivity_report, critical_weights[], robustness   │
└─────────────────────────────────────────────────────────────┘
```

## Understanding Compensation Masking

This is the key differentiator from simple weighted scoring:

| Alternative | Cost | Safety | Speed | Composite |
|-------------|------|--------|-------|-----------|
| Option A | 95 | **15** ⚠️ | 90 | 67 |
| Option B | 60 | 85 | 70 | 72 |

Option B wins on composite (72 > 67). But without masking detection, you might not notice that Option A's composite is propped up by its cost score — and it scores 15/100 on safety. The pipeline flags this explicitly so you don't accidentally pick the cheap, dangerous option.

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "compare these options" / "which is better?" / "MCDA" | Full 4-stage pipeline |
| "what criteria matter here?" / "identify criteria" | Stage 1 only — criteria identification |
| "weight and score these" / "score my alternatives" | Stage 2 only — weighting and scoring |
| "is this ranking robust?" / "sensitivity check" | Stage 4 only — sensitivity analysis |
| "am I compensating?" / "cheap but dangerous?" | Stage 3 only — compensation masking check |

## Composition

- **Pragmatic-laziness:** Laziness narrows the alternative set (δS = 0 elimination); MCDA ranks the survivors on weighted criteria. Together: reduce, then decide.
- **Scenario-builder:** Evaluate strategies across scenarios — which strategy scores highest in the most futures?
- **Decision-journal:** MCDA produces the weighted rationale; the journal records it with context for later revisit and Brier scoring.
- **Dokkodo-mindset:** Precept 11 ("have no preferences") — MCDA makes preferences explicit as weighted criteria so you can check whether preference is distorting the ranking.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `identify-criteria.j2` | KnowAct | Identify and classify decision criteria |
| `weight-and-score.j2` | KnowAct | Weight criteria and score alternatives |
| `rank-alternatives.j2` | KnowAct | Rank with compensation masking detection |
| `sensitivity-analysis.j2` | KnowAct | Perturb weights, identify rank reversals, classify robustness |

## Quick Reference

1. **Identify** criteria — enumerate, classify benefit/cost, check independence
2. **Weight** — assign importance weights (direct or swing method)
3. **Score** — each alternative on each criterion (0–100), normalize
4. **Rank** — composite scores with compensation masking warnings
5. **Sensitivity** — perturb weights, find rank reversals, classify as robust / moderate / fragile

*"When you cannot measure, your knowledge is of a meager and unsatisfactory kind."* — Lord Kelvin
*"Not everything that counts can be counted, and not everything that can be counted counts."* — William Bruce Cameron


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/mcda.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = ranking confidence is high and sensitivity analysis shows acceptable robustness

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 22000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
