# Superforecasting Pipeline

**Location:** `registry/manifests/superforecasting-pipeline.yaml`  
**Templates:** `registry/templates/superforecasting/`  
**Version:** 0.21.0

## Overview

This pipeline implements Philip Tetlock's Fermi-ization methodology from the Good Judgment Project. It provides a structured, multi-stage approach to producing well-calibrated probabilistic forecasts.

## Pipeline Stages

| Stage | Template | Purpose | Energy Cap |
|-------|----------|---------|------------|
| 0 | `stage_0_triage.j2` | Classify question difficulty (Goldilocks zone) | 3,000 |
| 1 | `stage_1_fermi_decompose.j2` | Decompose into tractable sub-questions | 5,000 |
| 2 | `stage_2_outside_view.j2` | Establish base rates from reference classes | 5,000 |
| 3 | `stage_3_inside_view.j2` | Generate and evaluate causal hypotheses | 6,000 |
| 4 | `stage_4_evidence_update.j2` | Bayesian belief revision | 4,000 |
| 5 | `stage_5_synthesis.j2` | Dragonfly eye aggregation of perspectives | 4,000 |
| 6 | `stage_6_calibration.j2` | Assign precise, calibrated probability | 2,000 |
| 7 | `stage_7_record.j2` | Record forecast for tracking/audit | 500 |

**Total Energy Budget:** 25,000 tokens

## Theoretical Foundation

Based on Tetlock's **Ten Commandments for Aspiring Superforecasters**:

1. **Triage** (Commandment 1) — Focus on questions where effort pays off
2. **Fermi-ization** (Commandment 2) — Decompose intractable problems
3. **Outside/Inside View** (Commandment 3) — Anchor on base rates, adjust for specifics
4. **Evidence Updating** (Commandment 4) — Bayesian belief revision
5. **Causal Synthesis** (Commandment 5) — Dragonfly eye perspective aggregation
6. **Precision Calibration** (Commandments 6-7) — Use full probability scale
7. **Error Tracking** (Commandment 8) — Prepare for post-mortem analysis

## Usage

### Invoking the Pipeline

```yaml
# Example pipeline invocation
manifest_id: superforecasting-pipeline
input:
  forecasting_question: "Will [specific outcome] occur by [date]?"
  domain: "geopolitics"  # optional
  time_horizon: "6 months"  # optional
  resolution_criteria: "How the outcome will be judged"
  expiration_date: "2026-12-31"
```

### Stage Outputs

Each stage produces structured JSON output that feeds into subsequent stages:

```json
// Stage 0: Triage
{
  "difficulty_level": "goldilocks",
  "goldilocks_zone": true,
  "proceed_recommendation": true,
  "rationale": "..."
}

// Stage 1: Fermi Decomposition
{
  "sub_questions": ["...", "..."],
  "assumptions": [...],
  "knowns": [...],
  "unknowns": [...]
}

// Stage 2: Outside View
{
  "reference_classes": [...],
  "base_rates": [...],
  "starting_probability": 0.35
}

// Stage 6: Final Calibration
{
  "final_probability": 0.42,
  "confidence_level": "medium",
  "precision_justification": "...",
  "defensible_range": {"lower": 0.35, "upper": 0.50}
}
```

## CNS Integration

The pipeline emits CNS spans for monitoring:
- `cns.prompt.select` — Pipeline selection
- `cns.prompt.render` — Template execution at each stage
- `cns.prompt.outcome` — Forecast recorded

**Variety Counters:**
- `hypothesis_count` — Number of causal hypotheses generated
- `reference_class_count` — Number of reference classes identified
- `evidence_item_count` — Number of evidence items evaluated

**Algedonic Alert:** Triggered if variety deficit >100 (escalates to Curator)

## OCAP Requirements

The pipeline requires the following capabilities:
- Template render permissions for all 8 stages
- Manifest execution permission
- CNS emission permission
- Memory storage permission (for forecast recording)

All capabilities are template-scoped and expire after 3600 seconds.

## Error Handling

| Error Type | Behavior |
|------------|----------|
| Energy exceeded | Abort |
| Timeout | Retry (max 2, 2s backoff) |
| Validation failure | Abort |
| Capability denied | Escalate to Curator |

## Audit Trail

All pipeline executions are logged with:
- Input question and parameters
- Output from each stage
- Energy costs per stage
- CNS event references
- Final forecast record

## Testing the Pipeline

1. **Unit tests:** Test each template independently with mock inputs
2. **Integration tests:** Run full pipeline on historical questions with known outcomes
3. **Calibration tests:** Compare predicted probabilities to actual outcomes over time

## Future Enhancements

- [ ] Iterative loop (return to earlier stages on new evidence)
- [ ] Ensemble mode (multiple parallel pipeline runs) — Note: distinct from hKask ensemble module (deferred 2026-06-14)
- [ ] Human-in-the-loop checkpoints
- [ ] Automatic reference class lookup from knowledge base
- [ ] Brier score tracking and feedback

## References

- Tetlock, P. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*
- Good Judgment Project: https://goodjudgment.com/
- Fermi-ization methodology: https://goodjudgment.com/superforecasters-toolbox-fermi-ization-in-forecasting/
- Ten Commandments: https://goodjudgment.com/philip-tetlocks-10-commandments-of-superforecasting/
