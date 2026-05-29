---
title: "Tetlock Fermi-ization Pipeline Specification"
version: "0.21.0"
status: "Draft"
last_updated: "2026-05-21"
audience: [forecasters, analysts, developers]
domain: "Application"
ddmvss_categories: [domain]
---

# Tetlock Fermi-ization Pipeline Specification

**Document Type:** Technical Specification  
**Version:** 0.21.0  
**Status:** Draft  
**Last Updated:** 2026-05-21

---

## Executive Summary

The **Tetlock Fermi-ization Pipeline** (also called the **Superforecasting Pipeline**) is a structured, multi-stage reasoning system for producing well-calibrated probabilistic forecasts. It operationalizes the forecasting methodology developed by Philip Tetlock and the Good Judgment Project through an automated prompt cascade.

This specification defines the pipeline's architecture, stage contracts, theoretical foundations, and usage patterns for developers and end users.

---

## 1. Introduction

### 1.1 Purpose

This pipeline transforms natural language forecasting questions into calibrated probability estimates through a systematic process of:
- Question decomposition (Fermi-ization)
- Reference class analysis (outside view)
- Hypothesis generation and evaluation (inside view)
- Bayesian evidence updating
- Multi-perspective synthesis
- Probability calibration

### 1.2 Scope

This specification covers:
- Pipeline architecture and stage definitions
- Input/output contracts for each stage
- Theoretical foundations from forecasting research
- Usage patterns and examples
- Integration with hKask CNS and OCAP systems

### 1.3 Naming

The pipeline is known by two interchangeable names:
- **Tetlock Fermi-ization Pipeline** (technical name, emphasizes methodology)
- **Superforecasting Pipeline** (user-facing name, emphasizes outcome)

Internal identifier: `superforecasting-pipeline`

---

## 2. Theoretical Foundation

### 2.1 Source Material

This pipeline implements methodologies from:

1. **Tetlock, P. & Gardner, D. (2015).** *Superforecasting: The Art and Science of Prediction*
   - Ten Commandments for Aspiring Superforecasters
   - Outside/inside view distinction
   - Fermi-ization methodology

2. **Good Judgment Project** (IARPA-funded research tournament)
   - Empirical validation of forecasting techniques
   - Identification of superforecaster characteristics
   - Training module effectiveness studies

3. **Good Judgment Inc. Resources**
   - Fermi-ization toolbox documentation
   - Workshop training materials
   - Superforecaster case studies

### 2.2 Key Concepts

| Concept | Definition | Pipeline Stage |
|---------|------------|----------------|
| **Fermi-ization** | Decomposing intractable problems into tractable sub-problems | Stage 1 |
| **Outside View** | Reference class forecasting; base rate anchoring | Stage 2 |
| **Inside View** | Case-specific hypothesis generation and evaluation | Stage 3 |
| **Bayesian Updating** | Evidence-weighted belief revision | Stage 4 |
| **Dragonfly Eye View** | Multi-perspective synthesis and aggregation | Stage 5 |
| **Probability Calibration** | Precise, well-justified probability assignment | Stage 6 |

### 2.3 Ten Commandments Mapping

| Commandment | Pipeline Implementation |
|-------------|------------------------|
| 1. Triage | Stage 0: Difficulty classification |
| 2. Fermi-ize | Stage 1: Decomposition |
| 3. Outside/Inside View | Stages 2-3: Base rate + hypothesis analysis |
| 4. Evidence Updating | Stage 4: Bayesian revision |
| 5. Clashing Causal Forces | Stage 5: Synthesis |
| 6. Degrees of Doubt | Stage 6: Calibration |
| 7. Under/Overconfidence | Stage 6: Calibration |
| 8. Error Analysis | Stage 7: Recording for post-mortem |
| 9. Collaborative Improvement | Future: Ensemble mode |
| 10. Error-Balancing Practice | CNS variety monitoring |
| 11. No Binding Rules | Human override at all stages |

---

## 3. Pipeline Architecture

### 3.1 High-Level Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SUPERFORECASTING PIPELINE                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  INPUT: Forecasting Question                                            │
│    │                                                                     │
│    ▼                                                                     │
│  ┌──────────────┐                                                        │
│  │ Stage 0      │ Triage                                                 │
│  │ (Goldilocks) │ → Proceed/Abort                                        │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 1      │ Fermi Decomposition                                    │
│  │ (Breakdown)  │ → Sub-questions, Assumptions                           │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 2      │ Outside View                                           │
│  │ (Base Rates) │ → Reference Classes, Starting Probability              │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 3      │ Inside View                                            │
│  │ (Hypotheses) │ → Causal Pathways, Condition Analysis                  │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 4      │ Evidence Update                                        │
│  │ (Bayesian)   │ → Likelihood Ratios, Updated Probability               │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 5      │ Synthesis                                              │
│  │ (Dragonfly)  │ → Multi-Model Aggregation                              │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 6      │ Calibration                                            │
│  │ (Precision)  │ → Final Probability, Confidence                        │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌──────────────┐                                                        │
│  │ Stage 7      │ Recording                                              │
│  │ (Audit)      │ → Tracking ID, CNS Event                               │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  OUTPUT: Calibrated Forecast + Audit Trail                               │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Stage Specifications

#### Stage 0: Triage

**Purpose:** Classify question difficulty and determine if forecasting effort will pay off.

**Template:** `stage_0_triage.j2`

**Input Contract:**
```yaml
forecasting_question: string    # The question to forecast
domain: string|null             # Optional domain hint (e.g., "geopolitics")
time_horizon: string|null       # Optional time horizon (e.g., "6 months")
```

**Output Contract:**
```json
{
  "difficulty_level": "clocklike|goldilocks|cloudlike",
  "goldilocks_zone": boolean,
  "proceed_recommendation": boolean,
  "rationale": string
}
```

**Decision Criteria:**
- **Clocklike:** Simple rules of thumb suffice; low effort required
- **Goldilocks:** Effort meaningfully improves accuracy; proceed
- **Cloudlike:** Dominated by chance; effort unlikely to help

**Energy Budget:** 3,000 tokens

---

#### Stage 1: Fermi Decomposition

**Purpose:** Break the forecasting question into tractable, independent sub-questions.

**Template:** `stage_1_fermi_decompose.j2`

**Input Contract:**
```yaml
forecasting_question: string
triage_output: object           # From Stage 0
```

**Output Contract:**
```json
{
  "sub_questions": ["string"],
  "assumptions": [
    {
      "assumption": "string",
      "confidence": "low|medium|high",
      "notes": "string"
    }
  ],
  "knowns": ["string"],
  "unknowns": ["string"]
}
```

**Key Principles:**
- Unpack: "What would it take for yes? For no?"
- Separate knowable from unknowable
- Avoid question substitution
- Dare to be wrong with specific guesses

**Energy Budget:** 5,000 tokens

---

#### Stage 2: Outside View

**Purpose:** Establish base rates from reference classes.

**Template:** `stage_2_outside_view.j2`

**Input Contract:**
```yaml
forecasting_question: string
sub_questions: array            # From Stage 1
knowns: array                   # From Stage 1
```

**Output Contract:**
```json
{
  "reference_classes": [
    {
      "class_name": "string",
      "description": "string",
      "relevance": "string"
    }
  ],
  "base_rates": [
    {
      "reference_class": "string",
      "frequency": "string",
      "probability": float,
      "data_quality": "string"
    }
  ],
  "starting_probability": float
}
```

**Key Question:** "How often do things of this sort happen in situations of this sort?"

**Energy Budget:** 5,000 tokens

---

#### Stage 3: Inside View

**Purpose:** Generate and evaluate causal hypotheses.

**Template:** `stage_3_inside_view.j2`

**Input Contract:**
```yaml
forecasting_question: string
sub_questions: array            # From Stage 1
starting_probability: float     # From Stage 2
outside_view_output: object     # From Stage 2
```

**Output Contract:**
```json
{
  "hypotheses": [
    {
      "name": "string",
      "description": "string",
      "required_conditions": ["string"],
      "evidence_for": ["string"],
      "evidence_against": ["string"],
      "individual_probability": float
    }
  ],
  "hypothesis_probabilities": {
    "primary_hypothesis": "string",
    "primary_probability": float,
    "combined_probability": float,
    "reasoning": "string"
  }
}
```

**Key Question:** "What would it take for this hypothesis to be true?"

**Energy Budget:** 6,000 tokens

---

#### Stage 4: Evidence Update

**Purpose:** Bayesian belief revision based on evidence.

**Template:** `stage_4_evidence_update.j2`

**Input Contract:**
```yaml
forecasting_question: string
prior_probability: float        # From Stage 3
hypothesis_analysis: object     # From Stage 3
new_evidence: array|null        # Optional additional evidence
```

**Output Contract:**
```json
{
  "evidence_items": [
    {
      "evidence": "string",
      "strength": "weak|moderate|strong",
      "direction": "supports|contradicts|neutral",
      "likelihood_ratio": float
    }
  ],
  "likelihood_ratios": [float],
  "updated_probability": float
}
```

**Key Formula:** Posterior = Prior × LR / (1 + Prior × (LR - 1))

**Energy Budget:** 4,000 tokens

---

#### Stage 5: Synthesis

**Purpose:** Integrate multiple causal models and perspectives.

**Template:** `stage_5_synthesis.j2`

**Input Contract:**
```yaml
forecasting_question: string
updated_probability: float      # From Stage 4
hypothesis_analysis: object     # From Stage 3
inside_view_output: object      # From Stage 3
```

**Output Contract:**
```json
{
  "causal_models": [
    {
      "model_name": "string",
      "perspective": "string",
      "implied_probability": float,
      "confidence": "string"
    }
  ],
  "synthesized_probability": float,
  "dissenting_views": [
    {
      "view": "string",
      "steelmanned_argument": "string",
      "weight_considered": float
    }
  ]
}
```

**Key Practice:** "On the one hand... on the other hand... on the third hand..."

**Energy Budget:** 4,000 tokens

---

#### Stage 6: Calibration

**Purpose:** Assign precise, well-calibrated final probability.

**Template:** `stage_6_calibration.j2`

**Input Contract:**
```yaml
forecasting_question: string
synthesized_probability: float  # From Stage 5
synthesis_output: object        # From Stage 5
pipeline_outputs: object        # All previous stage outputs
```

**Output Contract:**
```json
{
  "final_probability": float,
  "confidence_level": "low|medium|high",
  "precision_justification": "string",
  "defensible_range": {
    "lower": float,
    "upper": float
  }
}
```

**Key Principle:** Use full 0-100% scale; avoid hedge words.

**Energy Budget:** 2,000 tokens

---

#### Stage 7: Recording

**Purpose:** Create structured audit record for tracking and post-mortem.

**Template:** `stage_7_record.j2`

**Input Contract:**
```yaml
forecasting_question: string
final_probability: float        # From Stage 6
confidence_level: string        # From Stage 6
pipeline_summary: object        # Summary of all stages
resolution_criteria: string|null
expiration_date: string|null
```

**Output Contract:**
```json
{
  "forecast_record": {
    "question": "string",
    "probability": float,
    "confidence": "string",
    "timestamp": "ISO 8601",
    "resolution_criteria": "string",
    "expiration_date": "string|null",
    "reasoning_summary": "string",
    "key_assumptions": ["string"]
  },
  "tracking_id": "string",
  "cns_event_id": "string"
}
```

**CNS Spans Emitted:**
- `cns.prompt.select`
- `cns.prompt.render`
- `cns.prompt.outcome`

**Energy Budget:** 500 tokens

---

### 3.3 Energy Budget Summary

| Stage | Budget (tokens) | Percentage |
|-------|-----------------|------------|
| 0: Triage | 3,000 | 12% |
| 1: Fermi Decomposition | 5,000 | 20% |
| 2: Outside View | 5,000 | 20% |
| 3: Inside View | 6,000 | 24% |
| 4: Evidence Update | 4,000 | 16% |
| 5: Synthesis | 4,000 | 16% |
| 6: Calibration | 2,000 | 8% |
| 7: Recording | 500 | 2% |
| **Total** | **25,000** | **100%** |

**Hard Limit:** Pipeline aborts if cap exceeded  
**Alert Threshold:** 80% (20,000 tokens)

---

## 4. Usage Guide

### 4.1 Basic Invocation

```yaml
# Minimal invocation
manifest_id: superforecasting-pipeline
input:
  forecasting_question: "Will Russia and Ukraine sign a ceasefire agreement by 2026-12-31?"
```

### 4.2 Full Invocation

```yaml
# Full invocation with all options
manifest_id: superforecasting-pipeline
input:
  forecasting_question: "Will the Fed lower interest rates by at least 50 basis points before Q3 2026?"
  domain: "economics"
  time_horizon: "6 months"
  resolution_criteria: >
    Resolution: Yes if the Federal Reserve announces a rate cut totaling
    50+ basis points from current levels before July 1, 2026. Source:
    Federal Reserve official announcements.
  expiration_date: "2026-07-01"
  options:
    skip_triage: false          # Always triage first
    max_iterations: 1           # Future: iterative refinement
    evidence_sources: []        # Future: pre-loaded evidence
```

### 4.3 Output Example

```json
{
  "pipeline_id": "sf-20260521-001",
  "status": "completed",
  "forecast": {
    "question": "Will the Fed lower interest rates by at least 50 basis points before Q3 2026?",
    "final_probability": 0.42,
    "confidence_level": "medium",
    "defensible_range": {
      "lower": 0.35,
      "upper": 0.50
    }
  },
  "tracking": {
    "tracking_id": "fcst-20260521-001",
    "cns_event_id": "cns-sf-20260521-001",
    "recorded_at": "2026-05-21T10:30:00Z",
    "expiration_date": "2026-07-01"
  },
  "stage_outputs": {
    "triage": {
      "difficulty_level": "goldilocks",
      "proceed_recommendation": true
    },
    "decomposition": {
      "sub_question_count": 5,
      "assumption_count": 7
    },
    "outside_view": {
      "reference_class_count": 3,
      "starting_probability": 0.35
    },
    "inside_view": {
      "hypothesis_count": 4,
      "primary_hypothesis": "Economic slowdown forces Fed action"
    },
    "evidence_update": {
      "evidence_count": 6,
      "net_likelihood_ratio": 1.4
    },
    "synthesis": {
      "model_count": 3,
      "dissenting_view_count": 2
    },
    "calibration": {
      "precision_justification": "Multiple models converge around 40-45%"
    }
  },
  "energy": {
    "total_cost": 23450,
    "by_stage": {
      "triage": 2800,
      "decomposition": 4900,
      "outside_view": 4800,
      "inside_view": 5900,
      "evidence_update": 3200,
      "synthesis": 3500,
      "calibration": 1850,
      "recording": 450
    }
  }
}
```

---

## 5. Integration

### 5.1 CNS Integration

The pipeline emits CNS spans for monitoring and variety tracking:

```yaml
cns:
  span_namespace: cns.prompt
  spans:
    - cns.prompt.select      # Pipeline selection
    - cns.prompt.render      # Each template execution
    - cns.prompt.outcome     # Final forecast recorded
  
  variety_counters:
    - hypothesis_count       # Number of hypotheses generated
    - reference_class_count  # Number of reference classes
    - evidence_item_count    # Number of evidence items
  
  algedonic:
    threshold: 100           # Variety deficit threshold
    escalation_target: Curator
```

### 5.2 OCAP Integration

Required capabilities:

```yaml
ocap:
  required_capabilities:
    - resource: template
      action: render
      template_id: superforecasting/stage_*
      energy_budget: [per-stage budget]
    - resource: manifest
      action: execute
      template_id: superforecasting-pipeline
      energy_budget: 500
    - resource: cns
      action: emit
      template_id: superforecasting-pipeline
      energy_budget: 500
    - resource: memory
      action: write
      template_id: superforecasting-pipeline
      energy_budget: 500
  
  delegation_chain_required: true
  signature_algorithm: ed25519
  capability_expiry_seconds: 3600
```

### 5.3 Memory Integration

Forecasts are recorded to memory for:
- Tracking and scoring
- Post-mortem analysis
- Calibration feedback
- Historical reference

```yaml
memory:
  storage_schema:
    forecast_id: string
    question: string
    probability: float
    confidence: string
    timestamp: datetime
    resolution_criteria: string
    expiration_date: datetime|null
    stage_outputs: json
    outcome: string|null        # Populated at resolution
    brier_score: float|null     # Calculated at resolution
```

---

## 6. Quality Assurance

### 6.1 Calibration Testing

Periodic calibration assessment:

```yaml
calibration_metrics:
  - brier_score              # Mean squared error
  - calibration_curve        # Predicted vs actual frequency
  - discrimination           # Separation of yes/no cases
  - resolution               # Deviation from base rate
  - uncertainty              # Ideal calibration limit
```

### 6.2 Validation Checks

At each stage:

| Stage | Validation |
|-------|------------|
| 0 | Difficulty classification justified |
| 1 | Sub-questions are tractable and independent |
| 2 | Reference classes are relevant; base rates sourced |
| 3 | Hypotheses are mutually exclusive where appropriate |
| 4 | Likelihood ratios are defensible |
| 5 | Dissenting views are steelmanned |
| 6 | Probability matches synthesis; confidence justified |
| 7 | Record is complete and auditable |

---

## 7. Limitations and Boundaries

### 7.1 What This Pipeline Does

- Produces calibrated probability estimates
- Documents reasoning transparently
- Enables post-mortem analysis
- Integrates with CNS monitoring
- Enforces structured thinking

### 7.2 What This Pipeline Does NOT Do

- Access real-time data (requires MCP integration)
- Guarantee accuracy (only calibration)
- Replace human judgment (augments it)
- Handle multi-turn iterative refinement (future)
- Aggregate multiple forecasters (future ensemble mode)

### 7.3 Known Limitations

1. **No Live Data:** Pipeline operates on knowledge available at execution time. For current events, integrate with `hkask-mcp-web`.

2. **Single Pass:** Current implementation is linear. Iterative refinement (returning to earlier stages) is future work.

3. **No Ensemble:** Single pipeline run. Multi-forecaster aggregation is future work.

4. **Human Override Required:** Final judgment calls (especially Stage 6) benefit from human review.

---

## 8. Future Enhancements

### 8.1 Planned

| Feature | Description | Priority |
|---------|-------------|----------|
| Iterative Loop | Return to earlier stages on new evidence | High |
| Ensemble Mode | Multiple parallel pipeline runs | High |
| Live Data Integration | Auto-fetch current context via MCP | Medium |
| Reference Class DB | Pre-computed base rate library | Medium |
| Brier Score Tracking | Automatic calibration feedback | High |

### 8.2 Experimental

- Human-in-the-loop checkpoints
- Automatic question clarification
- Counterfactual analysis mode
- Adversarial review (red team)

---

## 9. References

### 9.1 Primary Sources

- Tetlock, P. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*. Crown.
- Mellers, B., et al. (2015). "The psychology of intelligence analysis: Good judgment project findings." *Journal of Experimental Psychology*.
- Good Judgment Inc. "Superforecasters' Toolbox: Fermi-ization in Forecasting." https://goodjudgment.com/superforecasters-toolbox-fermi-ization-in-forecasting/
- Good Judgment Inc. "Ten Commandments for Aspiring Superforecasters." https://goodjudgment.com/philip-tetlocks-10-commandments-of-superforecasting/

### 9.2 Related Documentation

- `docs/architecture/hKask-architecture-master.md` — System architecture
- `docs/architecture/registry-templating-prompt-v2.md` — Registry and template system
- `registry/manifests/superforecasting-pipeline.yaml` — Pipeline manifest
- `registry/templates/superforecasting/` — Template files

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **Base Rate** | The frequency of an outcome in a reference class |
| **Brier Score** | Mean squared error of probability forecasts |
| **Calibration** | Match between predicted probabilities and actual frequencies |
| **Dragonfly Eye View** | Multi-perspective synthesis (named after dragonfly compound eyes) |
| **Fermi-ization** | Decomposition of complex problems into estimable sub-problems |
| **Goldilocks Zone** | Questions where forecasting effort meaningfully improves accuracy |
| **Inside View** | Case-specific analysis focusing on unique details |
| **Likelihood Ratio** | P(E\|H) / P(E\|~H); evidence strength metric |
| **Outside View** | Reference class forecasting; base rate anchoring |
| **Reference Class** | Category of similar events for base rate comparison |
| **Steelmanning** | Constructing the strongest version of an opposing argument |

---

## Appendix B: Example Forecast Walkthrough

### Question
"Will SpaceX successfully launch and recover Starship on its first orbital test flight in 2026?"

### Stage 0: Triage
- **Difficulty:** Goldilocks
- **Rationale:** Sufficient public data; outcome analyzable; not purely random

### Stage 1: Fermi Decomposition
**Sub-questions:**
1. What is SpaceX's historical success rate on first flights of new vehicles?
2. What is the technical readiness level of Starship systems?
3. What is the regulatory approval status?
4. What is the timeline pressure vs. safety tradeoff?

### Stage 2: Outside View
**Reference Classes:**
- First orbital flights of new launch vehicles: ~40% success
- SpaceX first flights (Falcon 1, Falcon 9, Dragon): ~50% success
- Highly publicized test flights: ~60% success (selection bias)

**Starting Probability:** 0.45

### Stage 3: Inside View
**Hypotheses:**
1. **Success on first attempt** (required: all systems nominal, no regulatory delays)
2. **Partial success** (launch but no recovery, or vice versa)
3. **Failure due to technical issues** (engine failure, structural failure)
4. **Failure due to external factors** (regulatory, weather, range safety)

### Stage 4: Evidence Update
**Evidence:**
- Recent test flight data (supports success)
- Regulatory approval progress (neutral)
- Technical anomaly reports (slightly contradicts)

**Updated Probability:** 0.48

### Stage 5: Synthesis
**Causal Models:**
- Historical analogy model: 0.45
- Technical readiness model: 0.50
- Organizational capability model: 0.55

**Synthesized Probability:** 0.47

### Stage 6: Calibration
**Final Forecast:** 47%  
**Confidence:** Medium  
**Defensible Range:** 40-55%

### Stage 7: Recording
**Tracking ID:** fcst-20260521-002  
**Resolution Criteria:** FAA/SpaceX official announcement of orbital test outcome  
**Expiration:** 2026-12-31

---

*Document Version: 0.21.0 | ℏKask — A Minimal Viable Container for Agents*
