# Generalized Monad Logic: An Allosteric Framework for Conceptual Analysis

**Authors:** hKask Research Group  
**Version:** 0.1.0 (Preprint)  
**Date:** May 2026

---

## Abstract

We present Generalized Monad Logic (GML), a formal framework for analyzing conceptual dynamics inspired by the Monod-Wyman-Changeux (MWC) allosteric model from biochemistry. GML treats concepts as existing in probability distributions over interpretive states, with contextual factors acting as allosteric effectors that shift interpretive equilibrium without instructing new meanings. We formalize a six-operation algebra (`bind`, `equilibrium`, `cooperate`, `inhibit`, `activate`, `homeostasis`), implement it as Allosteric Regulation Logic (ARL) natively within the hKask Cybernetic Nervous System (CNS), and demonstrate its application to contested concepts including freedom, privacy, and intelligence. The regulation kernel — the MWC state function — operates as a CNS-native primitive rather than an external MCP tool, enabling direct integration with algedonic alerts and variety sensing. GML provides a mathematically grounded language for understanding conceptual flexibility, cooperativity between ideas, and network-level coherence. We discuss open questions including distribution composition verification, empirical parameter estimation, and connections to Boltzmann machines.

**Keywords:** allosteric thinking, conceptual analysis, MWC model, allosteric regulation logic, ARL, hKask, CNS

---

## 1. Introduction

### 1.1 The Problem of Conceptual Flexibility

Concepts in natural language have no single fixed meaning. The word "freedom" can denote negative liberty (freedom from interference) or positive liberty (freedom to self-realize) depending on context. "Privacy" oscillates between secrecy (hidden from view) and control (agency over disclosure). These interpretive shifts are not random but follow structured patterns responsive to contextual pressure.

Existing frameworks for conceptual analysis include:
- **Conceptual blending** (Fauconnier & Turner, 2002) — combines mental spaces
- **Frame semantics** (Fillmore, 1982) — meaning understood relative to structured backgrounds
- **Distributional semantics** (Manning & Schütze, 1999) — meaning from co-occurrence patterns

Each captures important aspects but lacks a quantitative model of *how* concepts shift under contextual pressure.

### 1.2 The Allosteric Analogy

Allosteric proteins exhibit precisely this behavior. Hemoglobin exists in two conformational states (Tense/Relaxed) with different oxygen affinities. Oxygen binding does not instruct new conformations but selectively stabilizes the R-state, shifting the equilibrium. The system is governed by the MWC equation (Monod et al., 1965):

```
R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)
```

Where R̄ is the fraction in the active state, L is the default T/R ratio, c is the affinity ratio, n is the number of binding sites, and α is ligand concentration.

**Core hypothesis:** Concepts behave like allosteric proteins:
1. They exist as probability distributions over interpretive conformations
2. They possess allosteric ports where contextual effectors bind
3. Effectors shift interpretive equilibrium through selective stabilization, not instruction

### 1.3 Contribution

This paper presents:
1. **Formal specification** of GML as a six-operation algebra
2. **Implementation** as a KnowAct cascade in hKask
3. **Worked examples** applying GML to contested concepts
4. **Research agenda** identifying open questions

---

## 2. Mathematical Foundations

### 2.1 The MWC State Function

The MWC model describes a system with two conformational states (T and R) in thermal equilibrium. The probability of being in the R-state is:

```
R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)
```

**Parameters:**
- `L = [T]₀/[R]₀ = exp(-(E_T - E_R)/kT)` — allosteric constant (default bias)
- `c = K_R/K_T` — affinity ratio (selectivity)
- `n` — number of binding sites (cooperativity dimensionality)
- `α = [X]/K_R` — normalized ligand concentration (contextual pressure)

**Limiting cases:**
- α → 0: R̄ → 1/(1+L) (default bias dominates)
- α → ∞: R̄ → 1/(1+L·cⁿ) (saturating context)
- c → 0: R̄ → 1 (strong activator)
- c → ∞: R̄ → 0 (strong inhibitor)

### 2.2 Hill Coefficient

Cooperativity is measured by the Hill coefficient:

```
n_H = n · (1-c)/(1+c) · √(α/(1+α))
```

**Interpretation:**
- n_H > 1: Switch-like (positive cooperativity)
- n_H = 1: Linear (no cooperativity)
- n_H < 1: Graded (negative cooperativity)

### 2.3 Partition Function

The partition function normalizes probabilities:

```
Z = (1 + α)ⁿ + L·(1 + cα)ⁿ
```

In the Boltzmann interpretation:

```
Z = Σᵢ exp(-Eᵢ/kT)
P(state) = exp(-E_state/kT) / Z
```

### 2.4 Conceptual Mapping

| MWC (Biochemistry) | GML (Conceptual) |
|--------------------|------------------|
| Protein | Concept |
| Conformational states (T/R) | Interpretive frames |
| Allosteric constant L | Default interpretive bias |
| Allosteric site | Contextual port |
| Ligand/Effector | Contextual modifier |
| Cooperativity n_H | Amplification between ideas |
| Partition function Z | Probability landscape |
| Homeostasis | Network coherence |

---

## 3. GML Algebra

### 3.1 Domain Types

```rust
struct ConceptualSystem {
    id: ConceptId,
    name: String,
    t_state: Interpretation,  // Conservative frame
    r_state: Interpretation,  // Progressive frame
    l: f64,                   // Default bias
    ports: Vec<AllostericPort>,
    current_alpha: f64,
    current_r_bar: f64,
}

struct Effector {
    id: EffectorId,
    name: String,
    concentration: f64,
    effect_type: EffectType,  // Activator/Inhibitor/Neutral
    shape: EffectorShape,
}
```

### 3.2 Six Operations

**Definition 1 (bind).** Apply effector to concept, compute new equilibrium:
```
bind(concept, effector) → shifted_concept
  α_new = α_old + [effector]
  R̄_new = mwc_state_function(L, c, n, α_new)
```

**Definition 2 (equilibrium).** Compute state distribution:
```
equilibrium(concept) → Distribution { p_r, p_t, n_h }
```

**Definition 3 (cooperate).** Compute amplification between concepts:
```
cooperate(a, b) → n_H_a × n_H_b
```

**Definition 4 (inhibit).** Stabilize T-state:
```
inhibit(concept, inhibitor) → bind(concept, inhibitor)
  where inhibitor.effect_type = Inhibitor (c > 1)
```

**Definition 5 (activate).** Stabilize R-state:
```
activate(concept, activator) → bind(concept, activator)
  where activator.effect_type = Activator (c < 1)
```

**Definition 6 (homeostasis).** Assess network coherence:
```
homeostasis(network) → mean(1 - |R̄_i - target|)
```

### 3.3 Capability Constraints (OCAP)

GML operations are capability-gated:
- No ambient authority — operations require explicit capability tokens
- Least privilege — default capability = Recognize only
- Attenuation — capabilities can be restricted (e.g., "Recognize + Bind, no Inhibit")
- End-to-end enforcement — capabilities enforced at storage layer

---

## 4. Implementation

### 4.1 ARL in CNS

The regulation kernel — the MWC state function — is implemented as **Allosteric Regulation Logic (ARL)** natively within `hkask-cns`. ARL is not an external MCP server; it is a CNS-native regulation primitive that operates at the same level as variety sensing and algedonic alerts. This architecture ensures that equilibrium shifts are computed within the homeostatic feedback loop rather than through inter-process tool calls.

The ARL module in `hkask-cns` is accessed via `crate::allosteric`:

```rust
use hkask_cns::allosteric::{ArlKernel, MwcParameters, ConceptualSystem, Effector};

// Compute equilibrium shift — native regulation, not MCP tool call
let kernel = ArlKernel::new(MwcParameters { l: 100.0, c: 0.05, n: 3 });
let r_bar = kernel.state_fraction(alpha);  // R̄ from MWC equation
let n_h = kernel.hill_coefficient(alpha);   // cooperativity measure

// Gate: escalate when R̄ crosses threshold
if r_bar > escalation_threshold {
    kernel.emit_algedonic(AlgedonicLevel::Warning);
}
```

ARL gates map directly to CNS escalation behavior:
- **R̄ < 0.1:** T-state dominant — no action (default bias holds)
- **0.1 ≤ R̄ < 0.5:** Transition zone — CNS increases monitoring frequency
- **0.5 ≤ R̄ < 0.8:** R-state emerging — algedonic alert at CNS level
- **R̄ ≥ 0.8:** Saturation — variety deficit check; if deficit > 100, escalate to Curator

### 4.2 KnowAct Cascade

The GML *thinking pattern* (Allosteric Thinking) remains a KnowAct cascade composed via templates, but it now delegates regulation computation to the CNS-native ARL kernel rather than invoking MCP tool calls:

```yaml
cascade:
  pre:
    - template: gml/recognize-ensemble.j2
      knowact: [recognize, discriminate, parse]
  core:
    - template: gml/bind-effector.j2
      knowact: [analogy, infer, abduct]
      # Delegates to crate::allosteric for MWC computation
    - template: gml/compute-equilibrium.j2
      knowact: [calculate, compare]
      # Reads R̄ from CNS ARL kernel, not MCP tool result
  post:
    - template: gml/assess-coherence.j2
      knowact: [evaluate, reflect, calibrate]
```

Five Jinja2 templates implement the cascade:
- `recognize-ensemble.j2` — Parse concept into T/R states and ports
- `bind-effector.j2` — Apply effector, delegate equilibrium shift to ARL kernel
- `compute-equilibrium.j2` — Read R̄, n_H from CNS, compare before/after
- `assess-coherence.j2` — Evaluate network homeostasis via CNS variety counters
- `reframe-concept.j2` — Generate alternative interpretation frames

### 4.3 CNS Monitoring

ARL operations are instrumented with CNS spans under the `cns.arl.*` namespace:
- `cns.arl.bind` — Effector binding and equilibrium shift computation
- `cns.arl.equilibrate` — R̄ and n_H calculation
- `cns.arl.assess` — Network coherence evaluation
- `cns.arl.gate` — Escalation gate threshold check
- `cns.arl.escalate` — Algedonic alert emission

Algedonic alerts trigger on variety deficit > 100, integrated with ARL gate thresholds.

---

## 5. Worked Examples

### 5.1 Freedom (Negative vs. Positive Liberty)

**Parameters:**
- T-State: "Freedom from interference" (E = -10.0)
- R-State: "Freedom to self-realize" (E = -5.0)
- L = 100.0 (default to negative liberty)
- Ports: threat_response (c=0.1), resource_access (c=0.5)

**Analysis:**
1. Without context: R̄ ≈ 0.01 (99% negative liberty interpretation)
2. Security crisis (α=10, c=0.1): R̄ ≈ 0.65 (shift to positive liberty)
3. ΔR̄ = +0.64 (significant reframing)

**Insight:** Security threats shift freedom from "freedom from" to "freedom to" (protective action).

### 5.2 Privacy (Secrecy vs. Control)

**Parameters:**
- T-State: "Hidden from view"
- R-State: "Agency over disclosure"
- L = 50.0
- Ports: data_flow (c=0.3), consent_mechanism (c=0.5)

**Analysis:**
1. Technology change (α=5): R̄ ≈ 0.6
2. Social norms (α=3): Amplifies through consent port
3. Combined: R̄ ≈ 0.75

**Insight:** Privacy debates activate different ports — technical infrastructure vs. consent frameworks.

### 5.3 Intelligence (Fixed vs. Malleable)

**Parameters:**
- T-State: "Innate, immutable trait"
- R-State: "Developed through effort"
- L = 10.0
- Effectors: growth_mindset (activator), test_scores (inhibitor)

**Analysis:**
1. Baseline: R̄ ≈ 0.09
2. Growth mindset: R̄ ≈ 0.55
3. Test scores: R̄ ≈ 0.25
4. Battle between activators and inhibitors

**Insight:** The intelligence debate is contextual warfare between neuroplasticity evidence and standardized testing.

---

## 6. The Five Questions Method

GML is operationalized through five questions:

| # | Question | Operation |
|---|----------|-----------|
| 1 | "What states is this idea dancing between?" | recognize + equilibrium |
| 2 | "What are its ports — what could bind and shift it?" | parse + discriminate |
| 3 | "What ideas amplify each other when co-present?" | analogy + cooperate |
| 4 | "What is suppressing this idea's generative state?" | detect + inhibit |
| 5 | "Is this idea-network self-reinforcing or decaying?" | evaluate + homeostasis |

This method provides structured guidance for applying GML to new concepts.

---

## 7. Open Questions

### 7.1 Distribution Composition Verification

GML's `bind` operation composes probability distributions over interpretive states. The composition structure is unverified:

```
Identity:     bind(unit_concept(x), f) = f(x)
Composition:   bind(bind(m, f), g) = bind(m, λx. bind(f(x), g))
```

Where `unit_concept` produces a concept with zero contextual pressure (α = 0, R̄ = 1/(1+L)). These are distribution composition laws, not monadic structure claims. Verification requires formalization of the probability algebra in a proof assistant.

**Status:** Open. Requires formalization in proof assistant (Lean, Coq).

### 7.2 Empirical Parameter Estimation

How do we measure L, c, n for abstract concepts?

**Approaches:**
- Direct elicitation (user ratings)
- Behavioral inference (interpretation choices under varying α)
- LLM-assisted estimation
- Bayesian updating

**Status:** Open. Validation study design in progress.

### 7.3 Partition Function for Idea-Spaces

Is E = -∑wᵢⱼsᵢsⱼ (Ising-like) appropriate for conceptual energies?

**Research direction:** Collect concept pairs with measured cooperativity, fit energy models, compare evidence.

### 7.4 Temporal Dynamics

Current model is equilibrium. Real conceptual shifts have dynamics:

```
dR̄/dt = k_on·α(t)·(1 - R̄) - k_off·R̄
```

**Status:** Open. Requires time-resolved experiments.

### 7.5 Collective Allostery

Do conceptual cascades occur? Are there homeostatic clusters?

**Research direction:** Network simulation with varying coupling strengths.

---

## 8. Limitations

1. **Parameter estimation requires judgment.** L, c, n encode analytical framing, not objective truth.
2. **Two-state simplification.** Real concepts may have >2 interpretive frames.
3. **Equilibrium assumption.** Temporal dynamics are unmodeled.
4. **Independent ligand assumption.** Multi-effector interactions may be non-independent.
5. **GML is a thinking tool, not truth machinery.** It reveals structure, not correctness.

---

## 9. Related Work

**Allosteric models:**
- Monod, Wyman, Changeux (1965) — original MWC formulation
- Changeux (2013) — allostery extended to brain function
- Phillips (2020) — MWC applied to signaling/regulation

**Conceptual analysis:**
- Fauconnier & Turner (2002) — conceptual blending
- Fillmore (1982) — frame semantics
- Gärdenfors (2000) — conceptual spaces

**Formal methods:**
- Wadler (1995) — monads for functional programming
- Boltzmann machines (Ackley et al., 1985) — statistical inference

---

## 10. Conclusion

GML provides a mathematically grounded framework for analyzing conceptual dynamics. By mapping the MWC allosteric model to abstract concepts, we gain:
- Quantitative language for interpretive flexibility
- Structured method for identifying contextual levers
- Network-level coherence assessment
- Capability-gated operations for security
- CNS-native ARL regulation with direct algedonic integration

Future work includes empirical validation, distribution composition verification, and temporal dynamics modeling.

---

## Acknowledgments

The hKask project benefits from the open-source ACP, MCP, and Okapi communities. This work is licensed under the same open principles.

---

## References

1. Ackley, D. H., Hinton, G. E., & Sejnowski, T. J. (1985). A learning algorithm for Boltzmann machines. *Cognitive Science*, 9(1), 147-169.

2. Changeux, J.-P. (2013). 50 years of allosteric interactions: the twists and turns of a model. *Nature Reviews Molecular Cell Biology*, 14(2), 133-142.

3. Fauconnier, G., & Turner, M. (2002). *The way we think: Conceptual blending and the mind's hidden complexities*. Basic Books.

4. Fillmore, C. J. (1982). Frame semantics. In *Linguistics in the morning calm* (pp. 111-137). Hanshin Publishing.

5. Gärdenfors, P. (2000). *Conceptual spaces: The geometry of thought*. MIT Press.

6. Manning, C. D., & Schütze, H. (1999). *Foundations of statistical natural language processing*. MIT Press.

7. Monod, J., Wyman, J., & Changeux, J.-P. (1965). On the nature of allosteric transitions: A plausible model. *Journal of Molecular Biology*, 12(2), 88-118.

8. Phillips, R. (2020). *The molecular switch: Signaling and allostery*. Princeton University Press.

9. Wadler, P. (1995). Monads for functional programming. In *Advanced Functional Programming* (pp. 24-52). Springer.

---

## Appendix A: Quick Reference

| Symbol | Meaning | Typical Range |
|--------|---------|---------------|
| L | Default bias (T/R ratio) | 0.01 – 1000 |
| c | Selectivity (R affinity / T affinity) | 0.01 – 10 |
| n | Binding sites (cooperativity dimensionality) | 1 – 10 |
| α | Contextual pressure | 0 – 100 |
| R̄ | Probability of R-state | 0 – 1 |
| n_H | Hill coefficient | 0 – n |

**Interpretation guide:**
- L > 10: Strong T-state bias
- c < 1: Activator (favors R)
- n_H > 1: Switch-like response
- R̄ > 0.7: Predominantly R-state

---

## Appendix B: ARL Sensitivity Table — MWC Parameters and Escalation Behavior

This table shows how R̄ responds to varying α (contextual pressure) across different MWC parameter regimes. Values are computed from R̄ = (1+α)ⁿ / ((1+α)ⁿ + L·(1+cα)ⁿ). The escalation column indicates which ARL gate would fire at each R̄ value.

| L | c | n | α=0.5 | α=1.0 | α=2.0 | α=5.0 | α=10.0 |
|---|---|---|-------|-------|-------|-------|--------|
| 100 | 0.05 | 3 | 0.0304 | 0.0647 | 0.1686 | 0.5252 | 0.7978 |
| 1000 | 0.01 | 3 | 0.0033 | 0.0077 | 0.0248 | 0.1573 | 0.5000 |
| 1000 | 0.1 | 6 | 0.0084 | 0.0349 | 0.1963 | 0.8038 | 0.9652 |

**Escalation key (ARL gate thresholds in CNS):**

| R̄ Range | ARL Gate | CNS Behavior |
|---------|----------|-------------|
| R̄ < 0.1 | No action | Default bias holds; T-state dominant |
| 0.1 ≤ R̄ < 0.5 | Monitor | CNS increases observation frequency |
| 0.5 ≤ R̄ < 0.8 | Alert | Algedonic alert emitted at CNS level |
| R̄ ≥ 0.8 | Escalate | Variety deficit check; if deficit > 100, escalate to Curator |

**Reading the table:**

- **Row 1 (L=100, c=0.05, n=3):** Moderate bias with strong activator. Escalation occurs between α=2 and α=5 (transition zone to alert). At α=10, approaching saturation.
- **Row 2 (L=1000, c=0.01, n=3):** Very strong bias with extreme activator. Requires high α to overcome default. Crossing 0.5 (alert threshold) near α=10 — the system is highly resistant to contextual pressure.
- **Row 3 (L=1000, c=0.1, n=6):** Strong bias but higher cooperativity. The steep sigmoid (n=6) means once α exceeds ~2, R̄ rises sharply through all four gates in quick succession. This is the switch-like regime — small changes in contextual pressure produce large equilibrium shifts.

**Operational parameter mapping:**

| MWC Parameter | Operational Quantity | Measurement |
|---------------|---------------------|-------------|
| L | Default interpretive bias | T-state selection frequency without context |
| c | Selectivity of contextual effector | R̄ shift per unit α at low concentration |
| n | Cooperativity dimensionality | Hill coefficient n_H at half-saturation |
| α | Normalized contextual pressure | Effector concentration / R-state affinity constant |
| R̄ | Active-state probability | Fraction of interpretations in R-state over observation window |

---

*Preprint. Under review. Comments welcome.*

*ℏKask — A Minimal Viable Container for Agents — GML v0.2.0*
