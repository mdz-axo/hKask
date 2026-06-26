---
name: superforecasting
visibility: public
description: Calibrated probability forecasting following Tetlock's Good Judgment Project methodology. Eight-stage pipeline from question triage through Fermi decomposition, outside/inside views, Bayesian evidence updating, dragonfly-eye synthesis, probability calibration, and forecast recording. Use when making predictions about uncertain future events, estimating probabilities, or calibrating judgment against known baselines.
activation: "forecast this"
---

# Superforecasting

Calibrated probability forecasting following the methodology developed and validated by Philip Tetlock's Good Judgment Project. An eight-stage pipeline that transforms a forecasting question into a calibrated probability estimate with full evidential trail — enabling later Brier scoring and calibration tracking.

## Why Superforecasting?

Most predictions are noise. People anchor on a gut feeling, adjust insufficiently, and never check whether they were right. Superforecasters — the top 2% of forecasters in Tetlock's multi-year tournament — don't have higher IQ or better access to information. They have **better process**:

1. **Fermi decomposition** — break the question into tractable sub-questions
2. **Outside view first** — start with base rates, not the specifics of this case
3. **Inside view second** — then adjust for case-specific evidence
4. **Bayesian updating** — revise probabilities as new evidence arrives, not once
5. **Dragonfly-eye synthesis** — integrate multiple perspectives, including dissenting views
6. **Calibrated precision** — use the full 0–100% scale, not vague ranges
7. **Record and score** — track every forecast; Brier scores reveal whether you're actually calibrated

The eight-stage pipeline encodes this process. Each stage produces output that feeds the next — the final probability is not a guess but a traceable chain of reasoning with explicit assumptions at every step.

## The Eight Stages

```
┌─────────────────────────────────────────────────────────────┐
│ STAGE 0: TRIAGE                                              │
│ Is this question forecastable? Is it in the Goldilocks zone   │
│ (not too easy, not too hard)? Worth the full pipeline?        │
│ Output: triage_classification, pipeline_warranted             │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 1: FERMI DECOMPOSITION                                  │
│ Break the question into independent, tractable sub-questions. │
│ Separate knowns from unknowns. Document assumptions.          │
│ Output: sub_questions[], knowns, unknowns, assumptions        │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 2: OUTSIDE VIEW                                        │
│ Establish base rates. Identify reference class. How often     │
│ do events like this occur? This is your ANCHOR.               │
│ Output: reference_class, base_rate_probability                │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 3: INSIDE VIEW                                         │
│ Generate multiple causal hypotheses. What specific evidence    │
│ for THIS case moves you away from the base rate? Adjust.      │
│ Output: causal_hypotheses[], inside_view_adjustment           │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 4: EVIDENCE UPDATE (Bayesian)                           │
│ Incorporate new evidence with likelihood ratios.              │
│ P(H|E) = P(E|H) × P(H) / P(E). Revise prior.                 │
│ Output: updated_probability, evidence_strength                │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 5: DRAGONFLY-EYE SYNTHESIS                              │
│ Integrate multiple causal models. Steel-man dissenting views. │
│ The dragonfly has 30,000 lenses — see from many angles.       │
│ Output: synthesized_probability, dissenting_views_considered  │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 6: CALIBRATION                                         │
│ Use the full 0–100% scale. 70% means "would happen 7 of 10   │
│ times in parallel universes." Justify precision level.        │
│ Output: calibrated_probability, precision_justification       │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 7: RECORD                                              │
│ Create structured forecast record with resolution criteria,   │
│ expiration date, and confidence bounds for later Brier        │
│ scoring and post-mortem analysis.                             │
│ Output: forecast_record                                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │  (Brier scoring triggers
                           │   Bayesian update re-entry)
                           ▼
               ┌──────────────────────┐
               │  STAGE 4 (re-entry)   │
               └──────────────────────┘
```

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "forecast this" / "superforecast this" / "what's the probability of..." | Full 8-stage pipeline |
| "base rate for this" / "outside view on this" | Stage 2 only — establish reference class |
| "decompose this question" / "Fermi this" | Stage 1 only — break into sub-questions |
| "update my forecast" / "new evidence on..." | Stage 4 only — Bayesian update with new evidence |
| "calibrate my probability" / "score my forecast" | Stage 6 only — calibration check |

## Composition

- **Pragmatic-laziness [Evidence]:** The outside view (Stage 2) implicitly uses lazy-universe reasoning — base rates are the "least action" probability before case-specific evidence adds complexity. The outside-view template already encodes base-rate reasoning inline; pragmatic-laziness provides informational context rather than a gate.
- **Grill-me:** Grill-me stress-tests the forecast's assumptions; superforecasting provides the structured probability to interrogate.
- **Dokkodo-mindset:** Precept 3 ("Do not ever rely on a partial feeling") maps directly to Fermi decomposition — don't forecast from a gut feeling; decompose first.
- **Structured-extraction:** Superforecasting's Stage 7 (record) receives populated forecast records from structured-extraction's schema-mapping pipeline — converting narrative forecasting sessions into structured, scorable forecast records.

## Registry Templates

| Template | Type | Stage | Purpose |
|----------|------|-------|---------|
| `stage_0_triage.j2` | WordAct | 0 | Triage question for forecastability |
| `stage_1_fermi_decompose.j2` | WordAct | 1 | Fermi decomposition into sub-questions |
| `stage_2_outside_view.j2` | WordAct | 2 | Reference class base rates |
| `stage_3_inside_view.j2` | WordAct | 3 | Case-specific causal hypotheses |
| `stage_4_evidence_update.j2` | WordAct | 4 | Bayesian evidence updating |
| `stage_5_synthesis.j2` | WordAct | 5 | Dragonfly-eye synthesis |
| `stage_6_calibration.j2` | WordAct | 6 | Probability calibration |
| `stage_7_record.j2` | WordAct | 7 | Structured forecast record |
| `superforecasting.yaml` | FlowDef | — | Cascading 8-stage pipeline orchestrator (convergence criteria, gas accounting, CNS spans) |

All eight are WordAct templates — each produces a structured artifact (triage report, decomposition, probability estimate, forecast record) that feeds the next stage.

## Quick Reference

1. **Triage** — is this worth forecasting?
2. **Fermi** — break into independent sub-questions
3. **Outside view** — what's the base rate?
4. **Inside view** — what makes this case different?
5. **Bayesian update** — revise with new evidence
6. **Dragonfly-eye** — integrate multiple perspectives
7. **Calibrate** — use the full 0–100% scale
8. **Record** — track for Brier scoring later

*"Superforecasters are not gurus or geniuses. They are careful, curious, and actively open-minded thinkers who update their beliefs."* — Philip Tetlock, *Superforecasting*


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/superforecasting.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = the forecast is sufficiently calibrated, confidence is justified, and record quality is adequate for later scoring

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 5 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
