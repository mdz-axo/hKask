# GML Research Agenda

**Version:** 0.1.0  
**Status:** Open

---

## Overview

This document tracks open questions and research directions for Generalized Monad Logic (GML). Each question includes research direction, priority, and expected deliverables.

---

## Open Questions

| # | Question | Priority | Status |
|---|----------|----------|--------|
| [9.1](#91-partition-function-for-idea-spaces) | What is the mathematical form of Z for idea-spaces? | High | Open |
| [9.2](#92-empirical-measurement-of-nh) | How do we measure cooperativity empirically? | High | Open |
| [9.3](#93-monad-law-verification) | Does `bind` satisfy monad laws? | High | Open |
| [9.4](#94-parameter-estimation-methods) | How do we estimate L, c, n parameters? | High | Open |
| [9.5](#95-multi-ligand-interactions) | Multi-ligand dynamics? | Medium | Open |
| [9.6](#96-temporal-dynamics) | Temporal dynamics (dR̄/dt)? | Medium | Open |
| [9.7](#97-collective-allostery) | Collective allostery in networks? | Medium | Open |
| [9.8](#98-learning-and-adaptation) | Learning/adaptation mechanisms? | Medium | Open |
| [9.9](#99-memory-integration) | Integration with memory systems? | Low | Open |
| [9.10](#910-empirical-validation) | Which operations carry load? | Low | Open |

---

## 9.1: Partition Function for Idea-Spaces

**Priority:** High  
**Status:** Open

### Problem

The Boltzmann machine formulation assumes E = -∑wᵢⱼsᵢsⱼ (Ising-like), but conceptual energies may have different structure.

### Research Questions

- What is the appropriate energy function for interpretive states?
- Are there interaction terms beyond pairwise (wᵢⱼₖ sᵢsⱼsₖ)?
- How does semantic similarity relate to energy?

### Approach

1. Collect dataset of concept pairs with measured cooperativity
2. Fit energy models: E = f(similarity, context, history)
3. Compare model evidence for different energy function forms
4. Test predictive power on held-out concept shifts

### Deliverable

Paper: "Energy Functions for Conceptual State Spaces"

---

## 9.2: Empirical Measurement of n_H

**Priority:** High  
**Status:** Open

### Problem

Hill coefficient is well-defined for proteins but has no established measurement protocol for concepts.

### Research Questions

- What is the observable corresponding to n_H?
- How many data points are needed for reliable estimation?
- Does n_H vary across populations or contexts?

### Experimental Design

1. Select 10 target concepts with clear T/R interpretations
2. Recruit N = 100 participants
3. Present concepts with varying contextual pressures (α = 0, 1, 2, 5, 10)
4. Measure interpretation choice (T vs R) at each pressure
5. Fit MWC curve: R̄(α) = (1+α)ⁿ/((1+α)ⁿ + L(1+cα)ⁿ)
6. Extract n_H from fitted parameters

### Deliverable

Dataset + analysis notebook; validation study paper

---

## 9.3: Monad Law Verification

**Priority:** High  
**Status:** Open

### Problem

GML is called a "Generalized Monad Logic" but the monad structure has not been formally verified.

### Monad Laws to Verify

```
Left identity:  bind(return(x), f) = f(x)
Right identity: bind(m, return) = m
Associativity:  bind(bind(m, f), g) = bind(m, λx. bind(f(x), g))
```

### Approach

1. Formalize `return` for concepts (what is the "pure" concept?)
2. Formalize `bind` signature: `(Concept, Concept → Concept) → Concept`
3. Use proof assistant (Lean, Coq, Agda) to verify laws
4. If laws fail, identify required modifications

### Deliverable

Formal proof (or counterexample); revised definition if needed

---

## 9.4: Parameter Estimation Methods

**Priority:** High  
**Status:** Open

### Problem

L, c, n are free parameters with no established elicitation protocol.

### Approaches to Compare

| Method | Description | Pros | Cons |
|--------|-------------|------|------|
| Direct elicitation | Ask users to rate "default bias" on 1-100 scale | Simple, fast | Subjective, noisy |
| Behavioral inference | Observe interpretation choices under varying context | Objective, grounded | Requires data |
| LLM-assisted | Prompt LLM to estimate parameters from concept descriptions | Scalable, informed | LLM biases |
| Bayesian updating | Start with priors, update from observations | Principled, adaptive | Computationally intensive |

### Research Design

1. Implement all four methods
2. Compare parameter estimates across methods
3. Validate against held-out interpretation data
4. Recommend best method (or ensemble)

### Deliverable

Parameter estimation library; comparison paper

---

## 9.5: Multi-Ligand Interactions

**Priority:** Medium  
**Status:** Open

### Problem

Current model assumes independent ligand binding. Real contextual factors may interact.

### Extended MWC Equation

```
Z = (1 + α₁)ⁿ(1 + α₂)ⁿ + L·(1 + c₁α₁ + c₂α₂ + c₁₂α₁α₂)ⁿ
```

Where c₁₂ captures interaction between effector 1 and 2.

### Research Questions

- What interaction patterns are empirically observed?
- Can we classify interactions (synergy, antagonism, independence)?
- How many interaction terms are needed before overfitting?

### Approach

1. Extend MWC equation with interaction terms
2. Fit to multi-effector experimental data
3. Use model selection (AIC, BIC) to determine optimal complexity
4. Build interaction taxonomy

### Deliverable

Extended MWC implementation; interaction taxonomy

---

## 9.6: Temporal Dynamics

**Priority:** Medium  
**Status:** Open

### Problem

Current model is at equilibrium. Real conceptual shifts have dynamics.

### Dynamical Extension

```
dR̄/dt = k_on·α(t)·(1 - R̄) - k_off·R̄
```

Where:
- k_on = association rate constant
- k_off = dissociation rate constant
- α(t) = time-varying contextual pressure

### Research Questions

- What are typical timescales for conceptual shifts?
- Do concepts exhibit hysteresis (path-dependence)?
- How does repeated exposure affect dynamics?

### Approach

1. Time-resolved experiments: measure R̄(t) after step change in α
2. Fit dynamical model to extract k_on, k_off
3. Test for hysteresis: forward vs reverse trajectories
4. Model repeated exposure (habituation, sensitization)

### Deliverable

Dynamical GML library; timescale measurements

---

## 9.7: Collective Allostery

**Priority:** Medium  
**Status:** Open

### Problem

Network-level behavior may not be reducible to individual concept dynamics.

### Research Questions

- Do conceptual cascades occur (one shift triggering others)?
- Are there homeostatic clusters (mutually reinforcing concepts)?
- Can networks undergo phase transitions?

### Approach

1. Simulate concept networks with varying coupling strengths
2. Identify emergent phenomena (cascades, clusters, transitions)
3. Compare to empirical network data (belief systems, ideologies)
4. Develop network-level diagnostics

### Deliverable

Network simulation toolkit; cascade theory paper

---

## 9.8: Learning and Adaptation

**Priority:** Medium  
**Status:** Open

### Problem

Parameters are static; real conceptual systems adapt from experience.

### Bayesian Updating Rule

```
P(L | data) ∝ P(data | L) · P(L)
```

Where:
- P(L) = prior over allosteric constant
- P(data | L) = likelihood of observed R̄ given L
- P(L | data) = posterior (updated estimate)

### Research Questions

- What constitutes an "observation" in GML?
- How quickly should parameters update (learning rate)?
- Can the system meta-learn (learn the learning rate)?

### Approach

1. Implement Bayesian updating for L, c, n
2. Test on synthetic data with known parameter drift
3. Validate on longitudinal concept data
4. Explore meta-learning approaches

### Deliverable

Adaptive GML implementation; learning dynamics paper

---

## 9.9: Memory Integration

**Priority:** Low  
**Status:** Open

### Problem

GML operates on concepts, but concepts are stored/retrieved via hkask-memory.

### Research Questions

- Does allosteric state affect encoding strength?
- Does retrieval cue allosteric state?
- Can memory consolidation be modeled as allosteric stabilization?

### Hypotheses

- H1: R-state concepts are encoded more strongly (generative = memorable)
- H2: Retrieval cues act as effectors (shift state during recall)
- H3: Consolidation = gradual L-update toward stable interpretation

### Approach

1. Integrate GML with hkask-memory semantic/episodic stores
2. Test H1: Compare recall for R-state vs T-state concepts
3. Test H2: Measure state shift before/after cued recall
4. Model consolidation as parameter update

### Deliverable

GML-memory integration; memory encoding study

---

## 9.10: Empirical Validation

**Priority:** Low  
**Status:** Open

### Problem

GML utility is untested at scale.

### Validation Design

1. Apply GML to 100+ conceptual analysis problems
2. Track: operation usage, parameter values, insight quality ratings
3. Analyze: which operations predict high-quality insights?
4. Prune: remove operations that don't carry load (P6)

### Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| Usage frequency | How often is each operation invoked? | Track |
| Insight quality | User-rated novelty/usefulness (1-5 scale) | > 3.5 |
| Parameter stability | Do L, c estimates converge across uses? | r > 0.7 |
| Time to insight | How long until user reports "aha" moment? | Track |

### Deliverable

Validation dataset; operation efficacy analysis; pruned algebra if needed

---

## Validation Study Design

### Title

"Allosteric Thinking: Empirical Validation of Generalized Monad Logic"

### Participants

N = 50 (mixed expertise: philosophers, cognitive scientists, general public)

### Tasks

1. Analyze 5 concepts using GML (freedom, privacy, intelligence, security, justice)
2. Apply 3 contextual effectors per concept
3. Rate insight quality (novelty, usefulness, clarity)
4. Complete debriefing interview

### Measures

| Type | Measure |
|------|---------|
| Primary | Insight quality ratings (1-5 Likert) |
| Secondary | Task completion time, operation usage patterns, parameter stability |
| Exploratory | Pre/post conceptual flexibility |

### Analysis

- Mixed-effects regression: insight_quality ~ operation_usage + concept + participant
- Clustering: identify operation usage patterns
- Qualitative: thematic analysis of interview data

### Timeline

- Recruitment: 1 month
- Data collection: 1 month
- Analysis: 1 month

---

## Success Criteria

| Criterion | Metric | Target |
|-----------|--------|--------|
| Mathematical correctness | MWC equations verified against reference | 100% match |
| Monad law compliance | Formal proof or counterexample | Proof completed |
| Parameter estimation | Correlation with ground truth | r > 0.7 |
| User utility | Mean insight quality rating | > 3.5 / 5.0 |
| Operation efficacy | At least 4 of 6 operations show predictive validity | p < 0.05 |
| Line budget | tokei reports ≤ 2,000 lines Rust | Pass |
| Empirical grounding | Validation studies published | 2+ papers |

---

## Backlog Issues

### Enhancement

- [ ] #001: Implement partition function derivation for idea-spaces
- [ ] #002: Design empirical n_H measurement protocol
- [ ] #003: Formal monad law verification (Lean/Coq)
- [ ] #004: Parameter estimation method comparison
- [ ] #005: Multi-ligand interaction terms
- [ ] #006: Temporal dynamics (dR̄/dt)
- [ ] #007: Network cascade simulation
- [ ] #008: Bayesian parameter updating
- [ ] #009: Memory system integration
- [ ] #010: Large-scale validation study (100+ problems)

### Documentation

- [ ] #011: API documentation for hkask-gml-* crates
- [ ] #012: Tutorial notebook (Jupyter)
- [ ] #013: Comparison to related frameworks (conceptual blending, etc.)

### Testing

- [ ] #014: Property-based tests for MWC equations
- [ ] #015: Integration tests for cascade execution
- [ ] #016: Fuzz testing for capability enforcement

---

## See Also

- [User Guide](./gml-user-guide.md)
- [Architecture](./gml-architecture.md)
- [API Reference](./gml-api.md)

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
