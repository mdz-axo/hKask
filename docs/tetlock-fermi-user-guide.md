---
title: "Tetlock Fermi-ization Pipeline — User Guide"
version: "0.21.0"
status: "Active"
last_updated: "2026-05-21"
audience: [forecasters, analysts, developers]
domain: "Application"
ddmvss_categories: [domain]
---

# Tetlock Fermi-ization Pipeline — User Guide

**For:** Forecasters, analysts, decision-makers  
**Version:** 0.21.0  
**Last Updated:** 2026-05-21

---

## Quick Start

### What This Does

The Tetlock Fermi-ization Pipeline helps you produce **well-calibrated probability forecasts** for uncertain future events. It structures your thinking through 8 stages:

1. **Triage** — Is this question worth forecasting?
2. **Breakdown** — What smaller questions compose it?
3. **Base Rates** — How often does this happen generally?
4. **Hypotheses** — What specific pathways could occur?
5. **Evidence** — What does new information tell us?
6. **Synthesis** — How do different perspectives combine?
7. **Calibration** — What's the precise probability?
8. **Recording** — Track for later scoring

### When to Use

**Use this pipeline for:**
- Geopolitical forecasts (elections, conflicts, policy changes)
- Business forecasts (market entries, product launches, M&A)
- Technology forecasts (adoption rates, milestone achievements)
- Any uncertain future event where you want calibrated probabilities

**Don't use for:**
- Questions with obvious answers (use quick judgment)
- Questions dominated by pure chance (lottery, random noise)
- Questions without resolution criteria (unfalsifiable claims)

---

## How to Invoke

### Basic Usage

```bash
kask forecast --pipeline tetlock \
  --question "Will the Fed cut rates by 50+ basis points before Q3 2026?"
```

### With Options

```bash
kask forecast --pipeline tetlock \
  --question "Will Russia-Ukraine ceasefire be signed by 2026-12-31?" \
  --domain geopolitics \
  --time-horizon "8 months" \
  --resolution "Official signing announced by both governments" \
  --expiration 2026-12-31
```

### Programmatic Usage

```yaml
# In your workflow YAML
steps:
  - manifest: superforecasting-pipeline
    input:
      forecasting_question: "Will X occur by date Y?"
      domain: "your domain"
      time_horizon: "time until resolution"
      resolution_criteria: "How outcome will be determined"
      expiration_date: "when this expires"
```

---

## Understanding the Output

### Forecast Result

```json
{
  "forecast": {
    "question": "Will the Fed cut rates by 50+ basis points before Q3 2026?",
    "final_probability": 0.42,
    "confidence_level": "medium",
    "defensible_range": {
      "lower": 0.35,
      "upper": 0.50
    }
  }
}
```

**What this means:**
- **42% probability** — Your calibrated estimate
- **Medium confidence** — Evidence quality is moderate
- **35-50% defensible** — Reasonable people could argue for this range

### Stage Breakdown

The pipeline also shows what happened at each stage:

```
Stage 0 (Triage): Goldilocks zone — proceed ✓
Stage 1 (Breakdown): 5 sub-questions, 7 assumptions
Stage 2 (Base Rates): 3 reference classes, starting at 35%
Stage 3 (Hypotheses): 4 causal pathways identified
Stage 4 (Evidence): 6 evidence items, net LR 1.4
Stage 5 (Synthesis): 3 models, 2 dissenting views
Stage 6 (Calibration): 42% final, medium confidence
Stage 7 (Recording): Tracking ID fcst-20260521-001
```

---

## Interpreting Probabilities

### What Does 42% Mean?

A 42% probability means:
- In 100 similar situations, you'd expect this outcome ~42 times
- It's less likely than unlikely (below 50%)
- But far from impossible (above, say, 10%)

### Confidence Levels

| Confidence | Meaning | When to Use |
|------------|---------|-------------|
| **High** | Strong evidence, multiple agreeing models, stable base rates | Historical data available; experts agree |
| **Medium** | Moderate evidence, some model disagreement | Some data; reasonable disagreement possible |
| **Low** | Weak evidence, high uncertainty, novel situation | Little data; first-of-its-kind event |

### Defensible Range

If your defensible range is 35-50%:
- 35% is also reasonable given the evidence
- 50% is also reasonable given the evidence
- You chose 42% as your best estimate
- This acknowledges uncertainty in your own estimate

---

## Quality Checklist

### Before You Trust a Forecast

✓ **Triage passed** — Question is in Goldilocks zone  
✓ **Sub-questions are specific** — Not vague or circular  
✓ **Base rates are sourced** — Not invented  
✓ **Hypotheses are distinct** — Not rephrased same idea  
✓ **Evidence is evaluated** — Both pro and con considered  
✓ **Dissenting views steelmanned** — Strongest version considered  
✓ **Probability is precise** — Not just "maybe" or "likely"  
✓ **Tracking ID assigned** — Can be scored later

### Red Flags

⚠ **Question substitution** — Answering easier question than asked  
⚠ **Base rate invented** — No actual reference class  
⚠ **Single hypothesis** — Only one pathway considered  
⚠ **Evidence ignored** — Contradicting evidence dismissed  
⚠ **Hedge words** — "Probably," "maybe," "likely" instead of numbers  
⚠ **No resolution criteria** — How will we know if it's true?

---

## Common Use Cases

### 1. Geopolitical Forecasting

**Question:** "Will Country X hold elections by Date Y?"

**Key Reference Classes:**
- Election schedules in similar regimes
- Historical delays in comparable situations
- Regional stability patterns

**Key Hypotheses:**
- On schedule (institutional commitment)
- Delayed (technical/logistical issues)
- Cancelled (political/security reasons)

---

### 2. Business Forecasting

**Question:** "Will Company X launch Product Y before Date Z?"

**Key Reference Classes:**
- Company's historical launch delays
- Industry average delay rates
- Similar product category timelines

**Key Hypotheses:**
- On time (development complete)
- Delayed (technical/regulatory issues)
- Cancelled (strategic shift)

---

### 3. Technology Forecasting

**Question:** "Will AI system X achieve benchmark Y by Date Z?"

**Key Reference Classes:**
- Historical AI milestone timelines
- Similar benchmark achievement rates
- Research lab track records

**Key Hypotheses:**
- Achievement (technical progress sufficient)
- Partial achievement (some but not all criteria)
- Not achieved (technical barriers remain)

---

## Best Practices

### 1. Write Clear Questions

**Good:** "Will the Fed lower rates by ≥50 basis points before 2026-07-01?"

**Bad:** "Will the Fed do something about rates soon?"

### 2. Define Resolution Criteria

**Good:** "Resolution: Yes if Federal Reserve announces total cut ≥50bp from current levels before 2026-07-01. Source: Federal Reserve official announcements."

**Bad:** "Resolution: When we see what happens."

### 3. Check Base Rates

Always ask: "What's the reference class? What's the base rate?"

**Example:**
- Question: "Will this startup succeed?"
- Reference class: "Venture-backed startups in this sector"
- Base rate: "~15% succeed within 5 years"
- Starting probability: 0.15

### 4. Steelman Opposing Views

Don't just acknowledge opposing views—make them as strong as possible.

**Weak:** "Some people think it won't happen."

**Strong:** "The strongest argument against is X, supported by evidence Y and Z. If true, probability would be ~20%."

### 5. Track and Score

Every forecast should have:
- Tracking ID
- Expiration date
- Resolution criteria
- Plan for scoring (Brier score calculation)

---

## Iterative Use

### Updating Forecasts

When new evidence arrives:

1. **Re-run pipeline** with `new_evidence` parameter
2. **Compare** new probability to old
3. **Note the change** and reason
4. **Update tracking record**

```bash
kask forecast --pipeline tetlock \
  --question "Will X occur?" \
  --prior 0.42 \
  --new-evidence "New data point Y just announced"
```

### When to Re-Run

- **Major new evidence** (policy change, unexpected event)
- **Scheduled review** (weekly/monthly check)
- **Time decay** (approaching expiration date)

---

## Limitations

### What This Pipeline Does

✓ Structures your thinking  
✓ Forces base rate consideration  
✓ Requires multiple hypotheses  
✓ Produces calibrated probabilities  
✓ Creates audit trail

### What This Pipeline Does NOT Do

✗ Access real-time data (you must provide current info)  
✗ Guarantee accuracy (only calibration)  
✗ Replace human judgment (augments it)  
✗ Handle unresolved questions (needs clear resolution)

### Human Judgment Required

The pipeline is a **tool**, not an oracle. You must:
- Verify base rates are accurate
- Ensure hypotheses are plausible
- Evaluate evidence fairly
- Make final calibration judgment

---

## Troubleshooting

### Pipeline Aborts on Triage

**Problem:** Question classified as "cloudlike" (unpredictable)

**Solution:**
- Reformulate question to be more specific
- Add resolution criteria
- Shorten time horizon
- Accept that some questions aren't forecastable

### Energy Budget Exceeded

**Problem:** Pipeline exceeds 25,000 token cap

**Solution:**
- Simplify the question
- Reduce sub-question count
- Accept lower precision
- Run stages individually if needed

### Low Confidence Output

**Problem:** Pipeline reports "low confidence"

**This is normal** for:
- Novel events (first occurrence)
- Limited data situations
- Highly contingent outcomes

**Action:** Accept low confidence; it's honest uncertainty quantification.

---

## Next Steps

### After Running Pipeline

1. **Record the forecast** (automatic via Stage 7)
2. **Set calendar reminder** for expiration date
3. **Monitor for new evidence** (update if significant)
4. **Score at resolution** (calculate Brier score)
5. **Review calibration** (are your probabilities well-calibrated over time?)

### Building Forecasting Skill

1. **Track all forecasts** — Use pipeline consistently
2. **Review calibration** — Are 70% predictions happening ~70% of time?
3. **Identify biases** — Where do you systematically over/underestimate?
4. **Practice Fermi-ization** — Break down questions even outside pipeline
5. **Study superforecasters** — Read Tetlock's work; learn their habits

---

## Resources

### Documentation

- `docs/architecture/tetlock-fermi-pipeline-spec.md` — Technical specification
- `registry/manifests/superforecasting-pipeline.yaml` — Pipeline manifest
- `registry/templates/superforecasting/` — Template files

### Reading

- Tetlock, P. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*
- Good Judgment Inc. resources: https://goodjudgment.com/resources/
- Fermi-ization guide: https://goodjudgment.com/superforecasters-toolbox-fermi-ization-in-forecasting/

### Training

- Good Judgment Open (free forecasting tournaments)
- Good Judgment training workshops
- Self-practice: Forecast daily events; score monthly

---

*ℏKask — A Minimal Viable Container for Agents | v0.21.0*