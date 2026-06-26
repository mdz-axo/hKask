---
name: decision-journal
visibility: public
description: Kahneman-style decision journal for improving judgment quality. Records decisions with full context (reasoning, assumptions, alternatives, emotional state, bias awareness), defines calibrated probability predictions with time horizons, revisits outcomes to measure accuracy via Brier scores, and evaluates process quality independently of outcome quality. Use when making consequential decisions, tracking judgment calibration over time, or running decision post-mortems.
activation: "journal this decision"
---

# Decision Journal

A Kahneman-style decision journal for improving judgment quality through structured recording, calibrated prediction, and systematic revisit. Based on the methodology Daniel Kahneman (and later Tetlock's Good Judgment Project) advocated: the fastest way to improve decision quality is to record what you thought would happen *before* you know what happened — then check.

## Why a Decision Journal?

The human mind is a self-justifying machine. After an outcome is known, we unconsciously rewrite our memory of what we predicted. "I knew it all along" (hindsight bias) makes every decision look well-calibrated in retrospect. The only defense: **record predictions before outcomes are known, then score them after.**

The decision journal does four things a mental log cannot:

1. **Captures context** — reasoning, assumptions, alternatives considered, emotional state, and bias awareness *at the time of decision*
2. **Makes predictions explicit** — calibrated probability estimates with time horizons and observable indicators
3. **Schedules revisits** — automated revisit dates so you don't forget to check
4. **Computes Brier scores** — mathematical calibration measurement that reveals overconfidence, underconfidence, and true accuracy

Over time, a decision journal reveals patterns: which types of decisions you're well-calibrated on, which you're overconfident about, and whether your process quality is improving.

## The Decision Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│ 1. RECORD DECISION                                           │
│    • What is the decision?                                   │
│    • What was the reasoning?                                 │
│    • What alternatives were considered?                      │
│    • What disconfirming evidence was noted?                  │
│    • What was the emotional state? (affect heuristic check)  │
│    • What biases might be operating? (bias awareness)        │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. DEFINE EXPECTED OUTCOMES                                  │
│    • What specific, observable outcomes do you predict?      │
│    • Assign calibrated probabilities (0–100%)                │
│    • Set time horizons for each outcome                      │
│    • Define resolution criteria (what counts as "happened"?) │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
              ┌─────────────────────────┐
              │  Time passes...          │
              │  Revisit date triggered  │
              └────────────┬────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. REVISIT & EVALUATE                                        │
│    • Compare predicted vs. actual outcomes                   │
│    • Hindsight bias check: would you have predicted          │
│      differently knowing only what you knew then?            │
│    • Evaluate process quality independently of outcome       │
│      (good process can produce bad outcomes; bad process     │
│       can produce lucky good outcomes)                       │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. COMPUTE BRIER SCORE                                       │
│    • Brier = (1/N) × Σ(predicted - actual)²                 │
│    • 0.0 = perfect calibration, 0.25 = coin flip,            │
│      1.0 = perfectly wrong                                   │
│    • Detect overconfidence (Brier > expected from            │
│      probability distribution) and underconfidence           │
│    • Track calibration trends over time                      │
└─────────────────────────────────────────────────────────────┘
```

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "journal this decision" / "record this decision" / "decision journal" | Full record → define → schedule cycle |
| "what did I predict about..." / "revisit decision X" | Revisit & evaluate a prior decision |
| "score my predictions" / "Brier score" / "how calibrated am I?" | Compute Brier scores across all recorded decisions |
| "decision post-mortem" / "what went wrong with..." | Revisit + process quality evaluation |

## Pipeline

- **Superforecasting → decision-journal:** Superforecasting produces calibrated probabilities; decision-journal records them with context, schedules revisits, and computes Brier scores. Bidirectional pipeline confirmed by both skills.
- **Structured-extraction → decision-journal:** Structured-extraction populates decision records from narrative descriptions — mapping unstructured decision narratives to schema fields (reasoning, assumptions, alternatives) that the decision journal records.

## Conceptual Alignment

- **Pragmatic-laziness:** The decision journal is itself a brachistochrone — recording decisions takes time now but reduces total judgment error across time. No runtime delegation.
- **pragmatic-semantics:** Decision context includes which constraints were considered and at what force level. No runtime delegation.

## Understanding Brier Scores

| Brier Score | Calibration | What it means |
|-------------|-------------|---------------|
| 0.00–0.05 | Excellent | Your probability estimates closely match outcome frequencies |
| 0.05–0.15 | Good | Generally well-calibrated with some variance |
| 0.15–0.25 | Fair | Meaningful miscalibration — overconfident or underconfident |
| 0.25–0.50 | Poor | Little better than guessing; process needs work |
| > 0.50 | Worse than chance | Your predictions are systematically wrong |

Overconfidence bias shows as a Brier score higher than expected given the forecaster's average probability spread. A forecaster who always says "80%" but is right only 60% of the time has a high Brier score and an overconfidence problem.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `record-decision.j2` | KnowAct | Record decision with full context and bias awareness |
| `define-expected-outcomes.j2` | KnowAct | Define calibrated probability predictions with time horizons |
| `revisit-evaluate.j2` | KnowAct | Revisit recorded decision, compare predicted vs. actual |
| `compute-brier.j2` | KnowAct | Compute Brier scores and detect calibration bias patterns |
| `decision-journal-convergence-check.j2` | KnowAct | Compute normalized convergence metric for the journal cycle |

## Quick Reference

1. **Record** — what did you decide and why? Include context, assumptions, emotional state, bias awareness
2. **Predict** — what specific outcomes do you expect? Assign calibrated probabilities with time horizons
3. **Revisit** — compare predictions to outcomes. Check for hindsight bias
4. **Score** — compute Brier scores. Detect overconfidence and underconfidence
5. **Improve** — track patterns. Which types of decisions are you well-calibrated on?

*"The first step to better decision-making is to keep score."* — Daniel Kahneman, *Thinking, Fast and Slow*
*"I have always believed that scientific research is a process of systematic overconfidence followed by humbling recalibration."* — Philip Tetlock


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/decision-journal.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = calibration quality is acceptable (low error, clear process evaluation, and actionable learning signal)

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ (manifest `rjoule.cap` — see `registry/manifests/decision-journal.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
