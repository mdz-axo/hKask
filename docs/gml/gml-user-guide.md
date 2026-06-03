# Allosteric Thinking — User Guide

**Version:** 0.2.0  
**Type:** KnowAct (Allosteric Regulation Logic in CNS)  
**Status:** MVP

---

## Overview

Allosteric Thinking applies the Monod-Wyman-Changeux (MWC) allosteric protein model to abstract concept recombination and regulation.

The regulation kernel — the MWC state function — runs natively as **Allosteric Regulation Logic (ARL)** within the hKask CNS (`hkask-cns`). ARL is not an external MCP server; it is a CNS-native regulation primitive accessed through ARL gates in the homeostatic feedback loop. The GML thinking pattern (Allosteric Thinking) delegates computation to ARL rather than invoking MCP tool calls.

**Core hypothesis:** Ideas, like allosteric proteins:
1. Have no single fixed shape but exist as probability distributions over conceptual conformations
2. Possess allosteric ports where other ideas bind to shift conceptual equilibrium
3. Recombine through structured interaction patterns

**Regulation:** ARL gates in `hkask-cns` monitor R̄ values and escalate through the CNS algedonic pathway when interpretive equilibrium shifts beyond configured thresholds.

**Mathematical kernel:**
```
R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)
```

Where:
- `R̄` = fraction in active/relaxed (progressive) interpretation
- `L` = default bias (T/R ratio without context)
- `c` = selectivity (how context favors R over T)
- `n` = cooperativity dimensionality (number of ports)
- `α` = normalized contextual pressure

---

## The Five Questions

| # | Question | GML Operations | Template |
|---|----------|----------------|----------|
| **1** | "What states is this idea dancing between?" | `recognize` + `equilibrium` | `recognize-ensemble.j2` |
| **2** | "What are its ports — what could bind and shift it?" | `parse` + `discriminate` | `recognize-ensemble.j2` |
| **3** | "What ideas amplify each other when co-present?" | `analogy` + `cooperate` | `compute-equilibrium.j2` |
| **4** | "What is suppressing this idea's generative state?" | `detect` + `inhibit` | `bind-effector.j2` |
| **5** | "Is this idea-network self-reinforcing or decaying?" | `evaluate` + `homeostasis` | `assess-coherence.j2` |

---

## Question 1: "What states is this idea dancing between?"

**Purpose:** Identify the interpretive frames a concept oscillates between.

**Key insight:** Concepts have no single fixed meaning. They exist as probability distributions over interpretive conformations.

**What to look for:**
- **T-state (Tense/Conservative):** The closed, constrained, protective interpretation
- **R-state (Relaxed/Progressive):** The open, generative, expansive interpretation
- **Default bias (L):** Which state is favored without contextual pressure?

**Example: Freedom**

| State | Type | Description | Energy |
|-------|------|-------------|--------|
| T-State | Conservative | "Freedom FROM" — negative liberty, non-interference | -10.0 |
| R-State | Progressive | "Freedom TO" — positive liberty, self-realization | -5.0 |

**Interpretation:** With L = 100, freedom defaults strongly to the "freedom from" interpretation (~99% of the time without contextual pressure).

**Mathematical check:**
```
R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)

At α = 0 (no context): R̄ = 1 / (1 + L) = 1/101 ≈ 0.01
```

**When to use:** When you notice a concept being used ambiguously or when debates seem to talk past each other.

---

## Question 2: "What are its ports — what could bind and shift it?"

**Purpose:** Identify allosteric sites where contextual effectors can bind.

**Key insight:** Effectors don't instruct new meanings — they selectively stabilize pre-existing interpretations.

**What to look for:**
- **Port name:** What contextual dimension does this port respond to?
- **Effector shape:** What kind of context fits here?
- **Affinity (c):** Does binding favor R (c < 1) or T (c > 1)?

**Example: Freedom's Ports**

| Port | Effector Shape | Affinity (c) | Effect |
|------|----------------|--------------|--------|
| Threat Response | Security Crisis | 0.1 | Activator (favors R) |
| Resource Access | Economic Condition | 0.5 | Mild Activator |
| Social Comparison | Status Anxiety | 2.0 | Inhibitor (favors T) |

**Interpretation:** A security crisis binds strongly to the threat response port, shifting freedom toward the "freedom to" (protective action) interpretation.

**Capability constraint:** Effectors bind only to ports they are shaped for. You cannot apply economic pressure to a threat port. ARL gates in CNS enforce capability-gated access — operations require explicit capability tokens, and escalation is bounded by the OCAP budget.

**When to use:** When you want to understand what contextual changes would shift how a concept is interpreted.

---

## Question 3: "What ideas amplify each other when co-present?"

**Purpose:** Identify cooperativity between concepts.

**Key insight:** Concepts can exhibit positive cooperativity — the presence of one makes the other more accessible.

**Mathematical check:**
```
n_H = n · (1-c)/(1+c) · √(α/(1+α))

If n_H > 1: Switch-like (amplified response)
If n_H < 1: Graded (damped response)
If n_H = 1: Linear (no cooperativity)
```

**Example: Freedom + Security**

| Concept | n_H (alone) | n_H (together) | Amplification |
|---------|-------------|----------------|---------------|
| Freedom | 1.2 | — | — |
| Security | 0.8 | — | — |
| **Combined** | — | 2.4 | **2× amplification** |

**Interpretation:** When freedom and security are discussed together, they exhibit positive cooperativity. Small contextual shifts produce large interpretive changes.

**Logic gate behavior:**
- **AND gate:** Both concepts must be activated for full response
- **OR gate:** Either concept can activate the interpretive shift
- **XOR:** NOT realizable without explicit cooperativity engineering

**When to use:** When analyzing why certain concept pairings are politically or rhetorically powerful.

---

## Question 4: "What is suppressing this idea's generative state?"

**Purpose:** Identify inhibitors stabilizing the T-state.

**Key insight:** Inhibitors (c > 1) preferentially bind and stabilize the conservative interpretation.

**What to look for:**
- **Inhibitor name:** What contextual factor is present?
- **Concentration:** How strong is the pressure?
- **Effect:** How much does R̄ decrease?

**Example: Intelligence (Fixed vs. Malleable)**

| Effector | Type | Concentration | Effect on R̄ |
|----------|------|---------------|-------------|
| Evidence of growth | Activator | 5.0 | R̄: 0.1 → 0.6 |
| Standardized test scores | Inhibitor | 10.0 | R̄: 0.6 → 0.2 |
| Growth mindset framing | Activator | 3.0 | R̄: 0.2 → 0.5 |

**Interpretation:** Standardized test scores act as an inhibitor, shifting intelligence toward the "fixed trait" interpretation. Growth mindset framing counteracts this.

**Mathematical check:**
```
Before inhibitor: R̄ = 0.6
After inhibitor (α = 10, c = 2.0): R̄ = 0.2
ΔR̄ = -0.4 (significant shift toward T-state)
```

**When to use:** When you notice an idea being consistently interpreted in a closed/conservative way despite evidence for alternative frames.

---

## Question 5: "Is this idea-network self-reinforcing or decaying?"

**Purpose:** Assess network-level coherence (homeostasis).

**Key insight:** Networks of concepts can exhibit emergent stability or instability based on how their interpretations align.

**Mathematical check:**
```
Coherence score = mean(1 - |R̄_i - target_R̄|)

If score > 0.8: Stable (self-reinforcing)
If 0.5 < score < 0.8: Transitioning
If score < 0.5: Unstable (decaying)
```

**Example: Political Ideology Network**

| Concept | R̄ | Target R̄ | Coherence |
|---------|---|----------|-----------|
| Freedom | 0.3 | 0.5 | 0.8 |
| Equality | 0.7 | 0.5 | 0.8 |
| Authority | 0.2 | 0.5 | 0.7 |
| Community | 0.6 | 0.5 | 0.9 |

**Network coherence:** (0.8 + 0.8 + 0.7 + 0.9) / 4 = **0.8** → **Stable**

**Interpretation:** This ideology network is self-reinforcing. The concepts mutually support each other's current interpretations.

**When to use:** When analyzing why certain belief systems resist change or why others are in flux.

---

## Worked Examples

### Example 1: Privacy (Secrecy vs. Control)

| Parameter | Value |
|-----------|-------|
| **T-State** | Secrecy (hidden from view) |
| **R-State** | Control (agency over disclosure) |
| **L** | 50.0 (defaults to secrecy) |
| **Ports** | data_flow (c=0.3), consent_mechanism (c=0.5) |
| **Effectors** | technology_change (α=5.0), social_norms (α=3.0) |

**Analysis:**
1. Privacy defaults to "secrecy" interpretation (R̄ ≈ 0.02 without context)
2. Technology change binds to data_flow port, shifting toward "control" (R̄ ≈ 0.6)
3. Social norms amplify this shift through consent_mechanism port
4. Combined effectors produce R̄ ≈ 0.75 (strong "control" framing)

**Insight:** Privacy debates are often about which port is being activated — technical infrastructure vs. consent frameworks.

---

### Example 2: Intelligence (Fixed vs. Malleable)

| Parameter | Value |
|-----------|-------|
| **T-State** | Fixed trait (innate, immutable) |
| **R-State** | Malleable (developed through effort) |
| **L** | 10.0 (slight default to fixed) |
| **Ports** | evidence (c=0.2), feedback (c=0.4), challenge (c=0.6) |
| **Effectors** | growth_mindset (activator, α=5.0), test_scores (inhibitor, α=10.0) |

**Analysis:**
1. Without context, intelligence leans "fixed" (R̄ ≈ 0.09)
2. Growth mindset framing shifts toward "malleable" (R̄ ≈ 0.55)
3. Test scores as inhibitor pull back toward "fixed" (R̄ ≈ 0.25)
4. Combined: depends on concentration ratio

**Insight:** The intelligence debate is a battle between activators (evidence of neuroplasticity, growth interventions) and inhibitors (standardized testing, fixed language).

---

### Example 3: Security (Protection vs. Resilience)

| Parameter | Value |
|-----------|-------|
| **T-State** | Protection (barriers, boundaries) |
| **R-State** | Resilience (adaptation, recovery) |
| **L** | 100.0 (strongly defaults to protection) |
| **Ports** | boundary (c=0.8), monitoring (c=0.5), trust (c=0.2) |
| **Effectors** | threat_level (inhibitor, α=varies), trust_building (activator, α=varies) |

**Analysis:**
1. Security strongly defaults to "protection" (R̄ ≈ 0.01)
2. High threat levels further inhibit resilience (R̄ → 0.001)
3. Trust-building can shift toward resilience, but requires high concentration
4. Network effects: security + freedom interact cooperatively

**Insight:** Security is one of the most T-state-biased concepts — shifting toward resilience requires sustained, high-concentration activators.

---

## Step-by-Step Workflow

1. **Select a concept** you want to analyze
2. **Run `recognize`** to identify T/R states and default bias
3. **Identify ports** — what contextual factors could bind?
4. **Apply effectors** — simulate contextual shifts
5. **Compute equilibrium** — ARL kernel in CNS computes R̄ shift; read result from `cns.arl.equilibrate` span
6. **Assess coherence** — CNS variety counters report network stability after the shift
7. **Reframe** — generate alternative interpretations based on analysis

**ARL gate feedback:** At each step, CNS checks R̄ against escalation thresholds:
- R̄ < 0.1 → no action
- 0.1 ≤ R̄ < 0.5 → increased monitoring
- 0.5 ≤ R̄ < 0.8 → algedonic alert
- R̄ ≥ 0.8 → variety deficit check; potential Curator escalation

**CLI access:** ARL operations are available through `kask chat` and the HTTP API, not through MCP tool calls:
```bash
kask chat                           # Interactive — ARL gates fire automatically
kask chat -m qwen3:8b              # With specific model
echo "analyze freedom" | kask chat -f -  # Non-interactive
```

---

## Quick Reference Card

| Symbol | Meaning | Typical Range |
|--------|---------|---------------|
| **L** | Default bias (T/R ratio) | 0.01 – 1000 |
| **c** | Selectivity (R affinity / T affinity) | 0.01 – 10 |
| **n** | Binding sites (cooperativity dimensionality) | 1 – 10 |
| **α** | Contextual pressure (normalized concentration) | 0 – 100 |
| **R̄** | Probability of R-state | 0 – 1 |
| **n_H** | Hill coefficient (cooperativity) | 0 – n |

**Interpretation guide:**
- L > 10: Strong T-state bias
- L < 0.1: Strong R-state bias
- c < 1: Activator (favors R)
- c > 1: Inhibitor (favors T)
- n_H > 1: Switch-like response
- n_H < 1: Graded response
- R̄ > 0.7: Predominantly R-state
- R̄ < 0.3: Predominantly T-state

---

## Limitations

**GML is a thinking tool, not truth machinery.**

1. **Parameter estimation requires judgment.** L, c, n values are not objectively measurable for abstract concepts — they encode your analytical framing.

2. **Best used for exploration, not conclusion.** GML helps you see interpretive structure — it doesn't tell you which interpretation is "correct."

3. **The model is simplified.** Real conceptual dynamics may involve more than two states, non-concerted transitions, or time-dependent parameters.

4. **Cooperativity is approximate.** The Hill coefficient is a useful measure but doesn't capture all interaction patterns.

5. **Network effects are emergent.** Coherence scores are descriptive, not prescriptive — a "stable" network may be stably wrong.

---

## See Also

- [GML Research Paper](./gml-research-paper.md) — formal framework and ARL architecture
- [Allosteric Thinking V2](./gml-allosteric-thinking-v2.md) — implementation prompt and domain types
- `hkask-cns` crate — ARL kernel source (`crate::allosteric`)

---

*ℏKask — A Minimal Viable Container for Agents — GML v0.2.0*
*The second secret of life, generalized.*
