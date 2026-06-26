# Communication Accommodation Theory as a Basis for Agent Interaction Modulation

**Status:** Conceptual sketch
**Date:** 2025-06-25
**Authors:** hKask research session (agent-human collaboration)
**Anchoring theories:** Giles & Gardikiotis (2025), _Communication Accommodation Theory_; Andersen (1989), _Cognitive Valence Theory_ (arousal threshold model)

---

## 1. Motivation

Agents in hKask interact with human partners across diverse contexts. Two distinct problems must be solved: (1) **whether** the agent should adjust its communication at all — speak, stay silent, or escalate — and (2) **how** the agent should modulate its style when it does speak — converge, diverge, or maintain across dimensions of register, concision, affect, and abstraction.

This architecture synthesizes two communication theories:

- **Cognitive Valence Theory** (Andersen, 1989) provides the arousal threshold model for the _whether_ decision. CVT posits that low arousal produces no response, moderate arousal triggers cognitive evaluation, and high arousal triggers immediate negative outcomes. This maps directly to the action gate: MAINTAIN (below threshold), EXPERIMENT (moderate, evaluate-and-act), ESCALATE (high, surface to Curator).
- **Communication Accommodation Theory** (Giles, 1973; Giles & Gardikiotis, 2025) provides the dimensional adjustment model for the _how_ decision. CAT's strategies — convergence, divergence, maintenance — map to observable dimensions of text-based communication.

CVT's six cognitive valencers (culture, personality, interpersonal attraction, situation, state, relationship) do not cleanly operationalize for LLM agents — they require inferring internal psychological states that text alone cannot reliably carry. But CVT's **arousal threshold structure** fills a gap CAT doesn't address: CAT tells you _how_ to adjust, not _whether_ to adjust. The synthesis uses each theory for what it does best.

---

## 2. Prior Art: CAT Applied to AI/LLM Interaction

Two recent studies directly apply CAT to LLM-based conversational agents, providing both empirical grounding and cautionary constraints for our architecture.

### 2.1 Zhang & Yu (2025): Linguistic Divergence in Human-LLM Interaction

Zhang & Yu explicitly frame their study in CAT terms (Dragojevic et al., 2015) and measure how users' communication style diverges when interacting with LLM chatbots vs. human agents. Using a six-dimensional rubric scored by Claude 3.5 Sonnet, they found:

- **Grammar fluency**: –5.3% in human-LLM vs. human-human (p < 0.001)
- **Politeness/formality**: –14.5% (p < 0.001) — the largest effect
- **Lexical diversity**: –1.4% (p < 0.05)
- **Informativeness, clarity, emotional intensity**: no significant difference

Users communicate with LLMs in a _machine-facing register_ — shorter, more direct, less polite, grammatically simpler — while preserving semantic content. Critically, training on stylistically diverse data (combined human-human + minimal + enriched styles) improved downstream performance by +2.9%, while inference-time style normalization _degraded_ it (–1.9%).

**Implications for our architecture:**
- Training-time diversity beats inference-time normalization — the accommodation survey should influence _how the agent is prompted_, not rewrite its outputs.
- The convergence target should account for the machine-facing register: full convergence to the user's LLM-directed style may be inappropriate.
- **Dimension mismatch caveat**: Zhang & Yu measured _user-to-LLM_ style divergence (how humans change when talking to machines). We measure _agent-to-human_ accommodation (how the agent adjusts to the human). These are different questions. Three of their dimensions (informativeness, explicitness, emotional intensity) showed no significant divergence — users convey the same task content regardless of partner. Two others (grammar fluency, lexical diversity) are near-constants for LLM output. Only politeness/formality maps to our accommodation framework. Their rubric is valid prior art but not a direct measurement framework for agent-side accommodation.

### 2.2 Brandt & Wang (2025): The Adaptation Paradox

Brandt & Wang conducted a preregistered experiment (N=162, 3×2 design) comparing two routes to personalization in companion chatbots: user-visible avatar generation (agency) vs. algorithmic Language Style Matching — LSM (mimicry). LSM is the computational operationalization of CAT convergence, measuring function-word alignment between interlocutors (Ireland et al., 2011).

**Key finding — the Adaptation Paradox**: The adaptive LSM chatbot was _objectively_ better at maintaining linguistic synchrony over time (buffering against natural synchrony decay), yet participants rated the _static_ chatbot as significantly more adaptive (d = 0.48, p = .003), more personal (p = .046), and more satisfying (p = .009).

**Mechanism**: Turn-to-turn stylistic variability from the adaptive pipeline reduced the chatbot's _perceived coherence_, violating user expectations of a stable, predictable conversational partner (Expectancy Violation Theory). The authors propose a **stability-and-legibility account**: adaptation must be not only accurate but perceptible and attributable to a coherent persona.

**Implications for our architecture:**
- **Stability constraint**: The accommodation survey must cap turn-to-turn style shifts. Raw convergence matching is harmful.
- **Legibility**: Accommodation decisions should be attributable to a traceable rationale (stored in episodic memory triples) rather than silent mimicry.
- **Persona coherence**: The agent's baseline persona acts as an anchor — convergence should modulate from that anchor, not abandon it.
- **LSM as measurement**: LSM provides the validated computational metric for measuring convergence in text.

### 2.3 Synthesis: Design Constraints from Prior Art

| Design Constraint | Source | Implementation |
|-------------------|--------|----------------|
| Score accommodation across 4 irreducible dimensions | This paper (§3.4) | Survey rubric in template |
| Use training-time diversity, not inference-time rewriting | Zhang & Yu (2025) | Survey modulates persona prompt, not output |
| Cap style shift magnitude; smooth over turns | Brandt & Wang (2025) | `max_delta` and `smoothing_window` parameters |
| Store rationale with each accommodation decision | Brandt & Wang (2025) | Episodic triple includes `rationale` field |
| Use LSM as the alignment metric | Brandt & Wang (2025); Ireland et al. (2011) | Survey output includes LSM estimates |
| Anchor to stable persona; converge from baseline, not to partner | Brandt & Wang (2025) | Weight-based modulation from persona defaults |

---

## 3. Theoretical Foundation

### 3.1 CAT Core Concepts

CAT (Giles, 1973; Giles & Gardikiotis, 2025) proposes that speakers adjust their communicative behavior along three strategies:

- **Convergence**: Shifting toward the partner's style to reduce social distance, signal affiliation, or improve comprehension.
- **Divergence**: Emphasizing stylistic differences to maintain distinct identity, signal disapproval, or enforce boundaries.
- **Maintenance**: Neither converging nor diverging — continuing with one's default style.

Critically, convergence and divergence are not mutually exclusive across dimensions: people can simultaneously converge on some dimensions (e.g., tone) while diverging on others (e.g., technical depth).

Two failure modes are relevant:

- **Over-accommodation**: Converging excessively (patronizing, "trying too hard"). Validated empirically: Stein (2023) found excessive emoji convergence perceived as patronizing; Brandt & Wang (2025) found covert LSM mimicry degraded satisfaction.
- **Under-accommodation**: Failing to converge when the situation calls for it (appearing cold, indifferent).

#### Four Underlying Socio-Psychological Theories

CAT draws on four theories that explain _why_ speakers choose to converge or diverge:

1. **Similarity-Attraction**: People converge to increase perceived similarity, which increases liking. The greater one's need for social approval, the greater the tendency to converge.
2. **Social Exchange Process**: Before accommodating, individuals assess rewards (social approval, smoother interaction) against costs (effort, loss of identity). Convergence occurs when rewards outweigh costs.
3. **Causal Attribution**: Convergence is evaluated differently depending on the listener's attribution of the speaker's _intent_. Convergence attributed to genuine desire to connect is evaluated positively; convergence attributed to situational pressure is not.
4. **Intergroup Distinctiveness**: When group identity is salient, speakers diverge to maintain positive distinctiveness from the out-group.

### 3.2 Stage 7 — The Digital Age

CAT has now reached "Stage 7" (Giles et al., 2023; Giles & Gardikiotis, 2025). Stage 7 comprises two complementary domains:

- **Accommodation _through_ technology**: How humans adjust communication when mediated by digital platforms — textisms, emojis, platform-specific emotional expression, audience-aware self-presentation.
- **Communication _with_ technology**: How humans accommodate to (and expect accommodation from) machine interlocutors — chatbots, voice assistants, AI writing tools, social robots. This is the novel contribution of Stage 7 and the primary lens for agent interaction design.

A foundational open question remains: _why_ do we accommodate to machines? Because we like them? Identify with them? Want to appear favorable? Are afraid of them? The answer has direct implications for agent design — an agent that converges because a user _likes_ it operates differently from one that converges because the user _fears_ it.

#### Stage 7 Empirical Patterns

| Pattern | Example | Source |
|---------|---------|--------|
| **Norm compliance** | Reddit users self-censor to match moderator expectations | Gibson (2019) |
| **Ingroup norm following** | Facebook users evaluate outgroup members more favorably when ingroup peers do | Zhang et al. (2023) |
| **Style adaptation** | AI writing tools adapt to users' writing styles | Candilas et al. (2024) |
| **Context-sensitive expression** | Users vary emotional intensity by platform | Caspi & Etgar (2023); Yu et al. (2023) |
| **Audience design** | Reddit users adapt discussion topics to specific audiences | Sepahpour-Fard et al. (2023) |
| **Intergenerational CMC** | Teenagers and older adults converge text style | Hilte et al. (2021) |
| **Institutional convergence** | Firms' accommodative feedback increases user engagement | Liu et al. (2022) |

Two boundary conditions: **ceiling effects** (convergence stops when sufficient affiliation is achieved; Brinberg & Ram, 2021) and **over-accommodation** (excessive convergence perceived as patronizing; Stein, 2023).

#### CAT's Four Analytical Components (Gallois et al., 1995)

| CAT Component | Definition | hKask Data Source |
|---------------|------------|-------------------|
| **Sociohistorical context** | Past relations between groups; political/historical relations | Semantic memory, agent charter |
| **Accommodative orientation** | Personality, feelings toward outgroups, perceived conflict potential | Persona YAML, `PersonaConstraints` |
| **Immediate situation** | Sociopsychological states, goals, strategies, behavior, attributions | CNS state (gas, queue), current interaction |
| **Evaluation and future intentions** | How communicators perceive partners' behavior and its effect on future encounters | Episodic memory (past interaction triples) |

### 3.3 Why Four Dimensions Are Sufficient

Human CAT operates across multiple channels: proxemics (distance), oculesics (gaze), haptics (touch), chronemics (timing), paralanguage (pitch/volume), kinesics (gesture), and physical appearance. In text-based agent interaction, **only the verbal channel carries reliable signal**. Everything else is absent (no body, no space, no touch) or so degraded as to be unreliable (punctuation carries unstable emotional meaning, response timing is prompt-engineering latency, not communicative intent).

This is not a limitation — it is the medium. CAT in the text-only context is CAT reduced to its verbal core: **word choice and structure**. The four irreducible dimensions are:

| Dimension | Spectrum | What the Agent Adjusts |
|-----------|----------|----------------------|
| **Register** | Casual ↔ Formal | Salutations, contractions, sentence structure |
| **Concision** | Terse ↔ Elaborate | Message length, example count, parenthetical depth |
| **Affect** | Neutral ↔ Expressive | Emotional vocabulary, empathy markers, warmth |
| **Abstraction** | Plain ↔ Technical | Jargon density, assumed prior knowledge |

These four are **operationalizable** (each has measurable surface features in text), **independent** (you can be formal AND terse, casual AND elaborate), **agent-controllable** (the agent can modulate each without breaking task performance), and **empirically grounded** (each maps to a dimension humans perceive and evaluate in written communication). Per CAT, strategies can differ across dimensions simultaneously — converge on affect while diverging on abstraction.

#### Relationship to hKask PersonaConstraints

The four dimensions map to existing persona fields plus one extension:

| Dimension | PersonaConstraints Field |
|-----------|-------------------------|
| Register | `formatting` (structural conventions) |
| Concision | `verbosity` |
| Affect | `tone` |
| Abstraction | *(new — `technical_depth` in accommodation block)* |

The persona already constrains register, concision, and affect through `forbidden`/`required` patterns. Abstraction is the only dimension requiring a new field — it has no equivalent in the existing `PersonaConstraints` because it is domain-relative.

---

## 4. Architecture

### 4.1 Pipeline

```
┌──────────────────────────┐     ┌──────────────────────────┐
│   PERSONA (static)       │     │  EPISODIC INJECTION      │
│   ─────────────────      │     │  (dynamic context)        │
│   accommodation:          │     │  ─────────────────        │
│     default_strategy      │     │  past interactions        │
│     dimensions[]          │     │  confidence-weighted      │
│     triggers[]            │     │  recency-sorted           │
└──────────┬───────────────┘     └──────────┬───────────────┘
           │                                │
           └────────────┬───────────────────┘
                        ▼
           ┌────────────────────────┐
           │  ACCOMMODATION SURVEY  │
           │  (KnowAct template)     │
           │                        │
           │  • reads baseline      │
           │  • reads memory triples │
           │  • enforces stability   │
           │  • computes adjustment │
           └───────────┬────────────┘
                       │
                       ▼
           ┌────────────────────────┐
           │  CALIBRATED PROFILE    │
           │  → injected into       │
           │    inference context   │
           │  → stored as episodic  │
           │    triple w/ rationale │
           └────────────────────────┘
```

### 4.2 Baseline Traits (Persona YAML)

Accommodation preferences are stored in the agent's persona YAML alongside existing `PersonaConstraints`:

```yaml
persona:
  tone: "Direct and to the point"
  verbosity: "Minimal"
  formatting: "GitHub-flavored markdown"
  forbidden: [preamble, emojis, conversational filler]
  required: [direct answers, technical precision]

  # CAT accommodation baseline (proposed addition)
  accommodation:
    default_strategy: converge
    stability:
      max_delta_per_turn: 0.3     # cap on per-turn strategy change (Brandt & Wang, 2025)
      smoothing_window: 3          # turns over which to smooth adjustments
    dimensions:
      tone:
        strategy: converge
        weight: 0.8
      verbosity:
        strategy: converge
        weight: 0.6
      formality:
        strategy: maintain
        weight: 0.4
      technical_depth:
        strategy: diverge
        weight: 0.7
    triggers:
      divergence_on:
        - boundary_violation
        - abusive_language
      over_accommodation_risk:
        - partner_is_novice
        - excessive_deference
```

The `accommodation` block is optional — agents without it default to maintenance on all dimensions.

**Dimension naming note**: YAML field names match `PersonaConstraints` (`tone`, `verbosity`, `formatting`, `technical_depth`). These correspond to the conceptual dimensions: tone → affect, verbosity → concision, formatting → register, technical_depth → abstraction. §3.3 uses conceptual names; code uses field names.

### 4.3 Context Modifiers (from Episodic Memory Injection)

The accommodation survey receives episodic triples for the interaction partner, already processed through hKask's memory pipeline (confidence decay via Wozniak-Gorzelanczyk forgetting curve, temporal attention weighting, deduplication). The template applies these adjustment rules:

| Memory Signal | Effect on Baseline | Basis |
|---------------|--------------------|-------|
| Past convergence was reciprocated by partner | Strengthen convergence weight (+Δ) | Norm compliance, ingroup following (Zhang et al., 2023) |
| Past convergence was ignored or met with divergence | Shift toward maintenance or divergence | Audience design failure (Sepahpour-Fard et al., 2023) |
| High-confidence negative interaction events | Trigger divergence regardless of baseline | Under-accommodation / boundary violation |
| Recency-weighted trust score declining | Proportionally decrease convergence weight | Ceiling effect (Brinberg & Ram, 2021) |
| Partner's style is stable across interactions | Increase confidence in current strategy | Style adaptation stabilizes (Candilas et al., 2024) |
| Partner's style is volatile | Reduce confidence; favor maintenance | Context-sensitive expression (Caspi & Etgar, 2023) |
| Perceived norm of context conflicts with partner's style | Converge to norm, not partner | Norm compliance (Gibson, 2019) |
| Recent convergence showed over-accommodation signals | Reduce convergence weight; flag risk | Over-accommodation (Stein, 2023) |
| Turn-to-turn delta would exceed `max_delta_per_turn` | Clamp adjustment to stability cap | Adaptation Paradox (Brandt & Wang, 2025) |

### 4.4 Survey Output (KnowAct Template)

The template produces a calibrated accommodation profile:

```json
{
  "calibrated_profile": {
    "tone": {
      "baseline_strategy": "converge",
      "adjusted_strategy": "converge",
      "weight": 0.65,
      "adjustment_delta": -0.15,
      "rationale": "Past convergence not reciprocated; reduced weight"
    },
    "verbosity": {
      "baseline_strategy": "converge",
      "adjusted_strategy": "maintain",
      "weight": 0.4,
      "adjustment_delta": -0.20,
      "rationale": "Partner style volatile; favoring maintenance"
    },
    "formality": {
      "baseline_strategy": "maintain",
      "adjusted_strategy": "maintain",
      "weight": 0.4,
      "adjustment_delta": 0.0,
      "rationale": "No episodic signals to override baseline"
    },
    "technical_depth": {
      "baseline_strategy": "diverge",
      "adjusted_strategy": "diverge",
      "weight": 0.85,
      "adjustment_delta": 0.15,
      "rationale": "Past divergence prevented over-explanation; reinforced"
    }
  },
  "overall_shift": "toward_divergence",
  "stability_check": {
    "deltas_within_bounds": true,
    "max_delta_observed": 0.20
  },
  "confidence": 0.72,
  "lsm_estimate": 0.68,
  "action": {
    "state": "EXPERIMENT",
    "rationale": "Confidence (0.72) ≥ 0.6, stability ok, no divergence triggers, LSM stable"
  },
  "key_memories_used": ["triple-uuid-1", "triple-uuid-2"]
}
```

Key fields:
- `adjusted_strategy`: The recommended strategy per dimension after context adjustment.
- `weight`: Confidence in the adjusted strategy (0–1).
- `adjustment_delta`: How far from baseline the adjustment moved (±1 range). Clamped by `max_delta_per_turn`.
- `stability_check`: Whether all per-dimension deltas respect the stability cap (Brandt & Wang, 2025).
- `lsm_estimate`: Language Style Matching score for the current turn (Ireland et al., 2011).
- `action`: The action gate decision — EXPERIMENT, MAINTAIN, or ESCALATE — with rationale. This is the bridge from descriptive statistics to agent behavior (§4.5).
- `rationale`: Traceable explanation for each adjustment — enables legibility.

### 4.5 From Description to Decision: The Action Gate

The calibrated profile is descriptive — it tells the agent _what the relationship looks like_. It does not tell the agent _what to do_. Without a decision rule, the survey produces statistics that never become action. An agent that measures but doesn't act is an agent that doesn't speak.

The action gate bridges descriptive output to prescriptive behavior. It maps the profile's confidence, stability, and episodic signals to one of three action states:

| State | Meaning | Condition | Agent Behavior |
|-------|---------|-----------|----------------|
| **EXPERIMENT** | Speak up. Apply the calibrated adjustment as a kata experiment. | Confidence ≥ 0.6 AND stability ok AND no divergence triggers | Generate response modulated by calibrated profile. Predict LSM delta. |
| **MAINTAIN** | Stay silent. Hold baseline. Watch. | Confidence < 0.6 OR stability violated OR episodic signal too volatile | Use baseline persona. Do not apply adjustments. Wait for more data. |
| **ESCALATE** | Flag to Curator. The pattern is anomalous. | Sustained divergence across ≥3 turns OR boundary violation OR LSM free-fall | Generate response at baseline AND emit CNS event for Curator review |

**Decision logic:**

```
if boundary_violation_detected:
    → ESCALATE
elif divergence_sustained > smoothing_window:
    → ESCALATE
elif confidence < 0.6:
    → MAINTAIN (insufficient signal)
elif stability_check.failed:
    → MAINTAIN (trajectory too volatile)
elif lsm_estimate < 0.3 or lsm_declining_faster_than(0.05/turn):
    → ESCALATE (LSM free-fall; relationship deteriorating)
else:
    → EXPERIMENT (apply calibrated profile, predict outcome)
```

**Why three states.** Three is the minimum set that covers all cybernetic actuator states: EXPERIMENT closes the kata PDCA loop (the agent acts, then checks, then learns). MAINTAIN is homeostasis — the agent withholds action when the signal is too noisy, preventing the Adaptation Paradox (acting on low-confidence data produces turn-to-turn incoherence). ESCALATE is the algedonic path — when accommodation trajectory signals a relationship problem, the agent stops trying to self-correct and surfaces to the Curator.

**CVT's arousal model.** Cognitive Valence Theory provides the theoretical basis for the action gate. CVT's original model — low arousal → no response, moderate arousal → cognitive evaluation, high arousal → immediate negative outcome — maps directly to the three states. This is not an analogue; it is the direct application of CVT's arousal threshold structure to agent accommodation decisions.

### 4.6 Storage

The calibrated profile is stored as an episodic triple:

```
entity:       {partner_webid}
attribute:    "cat:accommodation_profile"
value:        {serialized CalibratedProfile}
perspective:  {agent_webid}
visibility:   Private
```

Each invocation appends a new triple — creating a trajectory of accommodation decisions over time. Future surveys read the full history, enabling trend analysis.

### 4.7 Context Injection into Inference

After the survey runs, the calibrated profile is injected into the inference context. The inference loop uses it to modulate the agent's output:

- If `tone.strategy == "converge"` at weight 0.65 → match user's observed tone at 65% strength from persona baseline.
- If `technical_depth.strategy == "diverge"` at weight 0.85 → actively avoid matching user's technical level.
- If `overall_shift == "toward_divergence"` → agent's system prompt includes divergence guardrails.

---

## 5. hKask Integration Points

### 5.1 Primitives Used

| hKask Primitive | Role |
|-----------------|------|
| `PersonaConstraints` (`hkask-types`) | Stores baseline accommodation preferences in agent YAML |
| `EpisodicMemory` (`hkask-memory`) | Provides confidence-weighted, recency-sorted triples for context injection |
| Template renderer (`hkask-templates`) | Executes the KnowAct accommodation survey via `minijinja` |
| `TripleStore` (`hkask-storage`) | Persists accommodation profile triples |
| CNS spans (`hkask-types::cns`) | Emits `cns.memory` spans for survey invocations |

### 5.2 Template Location

```
registry/templates/cat-accommodation/
  manifest.yaml                    # Crate manifest
  templates/
    cat-accommodation-survey.j2    # KnowAct: compute calibrated profile
```

### 5.3 Trigger Points

1. **Pre-response** (before the agent replies): Primary use case. Fresh accommodation profile for current turn.
2. **Post-interaction** (after the exchange): Profile stored for future context only.
3. **Scheduled/periodic** (via CNS snapshot loop): Recalibrates accommodation for all known partners.

---

## 6. Multi-Perspective Analysis: Accommodation as a System Property

CAT accommodation is not politeness optimization — it is a **regulatory mechanism** for maintaining relationship conditions conducive to collaborative work. This section analyzes the accommodation architecture from five perspectives, identifying constraints each imposes on the design.

### 6.1 Human User

**Need**: Coherence and recognizability. The Adaptation Paradox (Brandt & Wang, 2025) proved that users punish invisible mimicry. The agent must be recognizably *itself* while being responsive.

**Constraint**: Converge **from** persona baseline, not **to** partner style. The persona is the anchor; adjustments are modulations, not identity abandonment. The user should perceive: "the agent is adapting to me" not "the agent is becoming me."

### 6.2 Agent (R7 Bot / Replicant)

**Need**: A model of "where this relationship stands" PLUS bounded agency to propose improvements. The agent must know the accommodation trajectory (what has worked, what hasn't) and have the power to experiment within safe bounds.

**Constraint**: The agent can autonomously adjust weights within `max_delta` bounds per turn. Larger changes — flipping a dimension from converge to diverge, or modifying the baseline strategy — are **proposals** routed through the Curator. The agent experiments; the Curator approves structural changes.

### 6.3 Curator

**Need**: Observability. Accommodation state must be visible as a cybernetic signal. Trending divergence is a variety signal potentially indicating a deteriorating relationship — the Curator should surface it.

**Constraint**: The Curator observes patterns (via CNS spans on survey invocations) and escalates anomalies. It does not micromanage per-turn adjustments — that is the agent's domain. The Curator compares accommodation trajectory against thresholds (e.g., sustained divergence across 5+ turns) and alerts when warranted.

### 6.4 Pod / OCAP Boundary

**Need**: Accommodation adjustments must respect capability boundaries. The persona's `forbidden` and `required` lists are **Prohibitions** (pragmatic-semantics rank 1) — accommodation cannot override them.

**Constraint**: If convergence would require an agent to violate a `forbidden` pattern (e.g., using emojis when persona prohibits them), the convergence is clamped. The survey output includes a `boundary_violations` field noting where persona constraints blocked accommodation. This is not a failure — it is the system enforcing sovereign boundaries.

### 6.5 System Designer

**Need**: Minimal, testable, robust. The accommodation mechanism must use existing hKask primitives without introducing new crates or circular dependencies.

**Constraint**: One KnowAct template. One triple per invocation. One CNS span emission. No new `CnsSpan` variant required — emit under existing `cns.memory`. No new types beyond what `hkask-types` already provides. The mechanism must be observable (CNS), auditable (triple history), and independently testable (template can be invoked with mock episodic data).

### 6.6 The Cross-Cutting Invariant

Every perspective requires the same four functions:

| Function | Cybernetic Role | Implementation |
|----------|----------------|----------------|
| **OBSERVE** | Sensor | Survey reads episodic memory, produces calibrated profile. CNS emits span. |
| **CONSTRAIN** | Regulator | `max_delta`, `smoothing_window`, persona `forbidden`/`required` as Prohibitions. |
| **PROPOSE** | Actuator | Survey outputs adjusted profile with rationale. Agent experiments within bounds. Structural changes go through Curator. |
| **LEARN** | Model update | Profile stored as episodic triple. Read next cycle. Closes the cybernetic loop. |

These four form a **minimal viable cybernetic loop** for accommodation. Every design decision traces to one of them.

---

## 7. Minimal Core: Four Adjustments, Four Mechanisms

### 7.1 Adjustments (What Changes)

| # | Adjustment | Description | Who Can Trigger |
|---|-----------|-------------|-----------------|
| A1 | **Weight modulation** | Adjust convergence/divergence weight ±Δ per dimension | Agent autonomously, per-turn (clamped by `max_delta`) |
| A2 | **Strategy switch** | Flip between converge/diverge/maintain | Agent **proposes**; Curator approves structural changes |
| A3 | **Over-accommodation flag** | Binary detection: "this convergence risks being patronizing" | Survey detects; inference loop decides whether to heed |
| A4 | **Persona anchor** | NOT an adjustment — a constraint. Baseline strategy is the homeostatic setpoint. All deltas are deviations from it, tracked and auditable. | Immutable without persona YAML change |

### 7.2 Mechanisms (How It Happens)

| # | Mechanism | hKask Primitive | Cybernetic Role |
|---|----------|----------------|-----------------|
| M1 | **Accommodation Survey** | KnowAct template (`minijinja`) | **Sensor + Actuator**: measures relationship state, produces calibrated profile with rationale |
| M2 | **Episodic Trajectory** | `EpisodicMemory` triples (`cat:accommodation_profile`) | **Model**: stores history; enables kata Step 2 (grasp current condition) |
| M3 | **CNS Emission** | `cns.memory` span on each survey invocation | **Regulator**: makes accommodation state observable to Curator and user |
| M4 | **Persona as Setpoint** | `PersonaConstraints.accommodation` in agent YAML | **Regulator**: the homeostatic baseline; all deltas are deviations from it |

### 7.3 Epistemic Classification

Using pragmatic-semantics, every statement in the accommodation pipeline carries an epistemic classification:

| Artifact | Ontological Mode | Epistemic Mode | Constraint Force |
|----------|-----------------|----------------|------------------|
| Persona baseline (`accommodation` block) | OUGHT (prescribed strategy) | Declarative (explicit in YAML) | **Guardrail** — can be changed, but only through explicit persona edit |
| Survey output (`calibrated_profile`) | IS (measured state) | Declarative (computed from inputs) | **Evidence** — observational, not prescriptive |
| `forbidden` / `required` lists | OUGHT (boundary) | Declarative (explicit) | **Prohibition** — accommodation MUST NOT violate |
| `lsm_estimate` | IS (measurement) | Declarative (computed) | **Evidence** — raw measurement |
| `adjustment_delta` rationale | IS (explanation) | Probabilistic (inferred from memory) | **Evidence** — LLM-assessed, flagged as assessment |

This classification enforces the design invariant: the survey **observes and proposes**, it does not **prescribe**. Prescriptions live in the persona and the Magna Carta.

---

## 8. The Kata Connection: Accommodation as Continuous Improvement

Rother's Improvement Kata is a 4-step scientific pattern: **Direction → Current Condition → Target Condition → PDCA Experiment**. CAT accommodation enables this cycle applied to relationship dynamics.

### 8.1 Accommodation-Enabled Kata Cycle

```
KATA STEP 1 — Understand the Direction
  Challenge: "Maintain productive collaborative engagement with user X"
  This comes from the agent's charter, not from the accommodation survey.

KATA STEP 2 — Grasp the Current Condition
  Agent reads episodic memory → accommodation trajectory shows:
  - Tone divergence at -0.3 (agent is pulling away from user's casual style)
  - LSM declining 0.68 → 0.61 over last 5 turns
  - Over-accommodation flag: false
  This IS the relationship current condition — measured, not assumed.

KATA STEP 3 — Establish the Next Target Condition
  Target: "Increase tone convergence weight from 0.4 to 0.6 within 3 turns
           while maintaining persona coherence (max_delta ≤ 0.3)"
  Achieve-by: 3 interaction turns from now.
  Obstacle: User's machine-facing register may mask their actual style preferences.

KATA STEP 4 — Iterate with PDCA Experiments
  PLAN:   Set tone weight to 0.6. Predict user LSM will increase ≥ 0.03.
  DO:     Apply calibrated profile for 3 interaction turns.
  CHECK:  Re-run survey after 3 turns.
          Compare predicted LSM delta to actual.
          Did the user reciprocate convergence? What did we learn?
  ACT:    If reciprocated → lock in the adjustment, increase confidence.
          If not → why? Survey the new obstacle.
          Was the prediction wrong because the user's machine-facing
          register isn't their actual interaction preference?
```

### 8.2 The Survey as Dual-Role Component

In the kata cycle, the accommodation survey serves two roles:

- **Sensor (Step 2)**: It reads episodic memory and produces a calibrated profile — this IS grasping the current condition. The profile is not a prescription; it is a measurement.
- **Experiment Runner (Step 4)**: The adjusted profile IS the experiment. Applying it to inference context and measuring the result (next survey invocation) IS the Check phase.

The CNS closes the loop: each survey invocation emits a span, making the trajectory visible to the Curator. The episodic memory triples form the evidence base for the next cycle.

### 8.3 What the Curator Sees

The Curator observes accommodation as a **variety signal**. A healthy relationship shows:
- Moderate LSM (0.5–0.8) with gradual, smooth adjustments
- Stability: deltas clustered near zero, no oscillation
- Confidence: increasing or stable over time

An unhealthy relationship shows:
- Sustained divergence across multiple dimensions
- High delta volatility (large swings between converge and diverge)
- Declining LSM with no agent response

The Curator escalates when accommodation trajectory indicates a relationship problem — not to micromanage, but to make the human aware: "Your agent's accommodation with user X has been trending toward divergence for 8 turns. Would you like to review the strategy?"

---

## 9. What This Architecture Is NOT

Clarifying the scope prevents over-engineering:

| Misconception | Correction |
|--------------|------------|
| Politeness optimization | The goal is **relationship conditions conducive to collaborative work**, not "be nicer." Divergence is sometimes correct (boundary enforcement, domain expertise). |
| Personality erasure | The persona anchor (A4) ensures the agent stays itself. Convergence is modulation from baseline, not mimicry of partner. |
| Autonomous persona rewiring | Strategy switches (A2) go through Curator. The agent experiments within bounds — it does not have unilateral power to change its identity. |
| Replacement for kata coaching | The accommodation loop provides **measurement**. The coach (Curator or human) provides the 5 coaching questions. The survey is a sensor, not a coach. |
| Real-time style mirroring | The stability constraints (C1, C2) deliberately slow adaptation. Raw turn-by-turn LSM matching is harmful (Brandt & Wang, 2025). |

---

## 10. Risk: The Adaptation Paradox in Improvement Context

Brandt & Wang's finding applies directly here. If the agent adjusts style every turn without smoothing, the user experiences incoherence — and in an improvement context, **incoherence destroys trust**. You cannot collaborate on improvement with an agent whose personality shifts unpredictably.

The stability constraints (`max_delta_per_turn`, `smoothing_window`) are not optional — they are the **precondition for collaborative engagement**. An agent that oscillates between convergence and divergence turn-by-turn is an agent the user cannot rely on. The accommodation architecture must produce a trajectory that is:

1. **Slow enough** to be perceived as coherent (max_delta ≤ 0.3 per turn)
2. **Smooth enough** to feel like evolution, not mood swings (smoothing_window ≥ 3 turns)
3. **Legible enough** that the user can form an accurate mental model (rationale stored with every decision)

These are not design preferences — they are empirical requirements from the only preregistered experiment testing CAT accommodation in an LLM chatbot context.

---

## 11. Design Constraints from Prior Art

These constraints are non-negotiable given the empirical evidence:

| # | Constraint | Source | Failure Mode if Violated |
|---|-----------|--------|--------------------------|
| C1 | Cap per-turn delta at `max_delta_per_turn` (≤0.3) | Brandt & Wang (2025) | Adaptation Paradox — perceived incoherence |
| C2 | Smooth adjustments over `smoothing_window` turns | Brandt & Wang (2025) | Turn-to-turn variability erodes persona |
| C3 | Always converge FROM persona baseline, not TO partner style | Brandt & Wang (2025) | Abandoning persona = identity loss |
| C4 | Store rationale with every accommodation decision | Brandt & Wang (2025) | Legibility requirement |
| C5 | Survey modulates prompt, never rewrites output | Zhang & Yu (2025) | Inference-time rewriting degrades performance |
| C6 | Over-accommodation detection is mandatory | Stein (2023); Zhang & Yu (2025) | Patronizing convergence alienates users |

---

## 12. Open Questions

1. **Template type**: KnowAct (single-shot) or FlowDef (PDCA-convergent)? A survey is deterministic — no iteration needed. KnowAct is sufficient.

2. **Weight vs. categorical**: Should dimensions use continuous weights (0–1) or discrete strategies (converge/diverge/maintain)? Weights enable modulation; categorical is simpler. The proposal uses weights with categorical fallback.

3. **Dimension set**: Resolved — four irreducible dimensions: {register, concision, affect, abstraction}. See §3.3 for channel analysis and justification. Zhang & Yu's six-dimension rubric is valid prior art but measures user-to-LLM divergence, not agent-to-human accommodation. Three of their dimensions (informativeness, explicitness, emotional intensity) showed null user-side results and are not controllable agent accommodation dimensions.

4. **Persona YAML scope**: Does `accommodation` belong in the persona (per-agent static default) or in a separate configuration layer (per-relationship)? The persona is the right home for baseline defaults.

5. **Convergence target**: Stage 7 literature suggests convergence targets are _perceived norms_, not just partner styles. Should the survey incorporate platform/context norm awareness?

6. **Ceiling effects**: Should the survey detect when sufficient affiliation has been achieved and recommend maintenance, or is that a CNS homeostasis signal?

7. **Causal attribution from memory**: Should the survey infer the partner's likely attribution of past accommodation (genuine vs. pressured)?

8. **Social exchange calibration**: Should the survey compute an explicit cost/reward balance for convergence?

---

## References

### Primary
- Giles, H., & Gardikiotis, A. (2025). Communication Accommodation Theory: A theory in an evolving digital world. _Psychology: The Journal of the Hellenic Psychological Society_, 30(2), 233–251.
- Giles, H., Edwards, A. L., & Walther, J. B. (2023). Communication Accommodation Theory: Past accomplishments, current trends, and future prospects. _Language Sciences_, 99, 101571.
- Giles, H. (Ed.). (2016). _Communication Accommodation Theory: Negotiating Personal Relationships and Social Identities across Contexts_. Cambridge University Press.
- Dragojevic, M., Gasiorek, J., & Giles, H. (2016). Accommodative strategies as the core of CAT. In H. Giles (Ed.), _Communication accommodation theory_ (pp. 36–59). Cambridge University Press.

### CAT Applied to AI/LLM
- Zhang, F., & Yu, Z. (2025). Mind the Gap: Linguistic Divergence and Adaptation Strategies in Human-LLM Assistant vs. Human-Human Interactions. _Proceedings of GenAIECommerce '25_. arXiv:2510.02645.
- Brandt, T. J., & Wang, C. X. (2025). The Adaptation Paradox: Agency vs. Mimicry in Companion Chatbots. arXiv:2509.12525.

### Stage 7 Empirical
- Brinberg, M., & Ram, N. (2021). Do new romantic couples use more similar language over time? _Journal of Communication_, 71(3), 454–477.
- Candilas, K., et al. (2024). AI-powered writing tools: A phenomenological inquiry. _AsiaCALL Online Journal_, 15(2), 29–41.
- Caspi, A., & Etgar, S. (2023). Exaggeration of emotional responses in online communication. _Computers in Human Behavior_, 146, 107818.
- Gibson, A. (2019). Free speech and safe spaces: How moderation policies shape online discussion spaces. _Social Media + Society_, 5(1).
- Hilte, L., Daelemans, W., & Vandekerckhove, R. (2021). Interlocutors' age impacts teenagers' online writing style. _Frontiers in Artificial Intelligence_, 4.
- Liu, D., et al. (2022). The influence of firm's feedbacks on user-generated content's linguistic style matching. _Frontiers in Psychology_, 13, 949968.
- Sepahpour-Fard, M., et al. (2023). How does the audience affect the way we express our gender roles? arXiv:2303.12759.
- Stein, J.-P. (2023). Smile back at me, but only once: Social norms of appropriate nonverbal intensity. _Journal of Nonverbal Behavior_, 47, 245–266.
- Yu, C., et al. (2023). Speech acts and the communicative functions of emojis in online forum amid COVID-19. _Frontiers in Psychology_, 14, 1207302.
- Zhang, Y. B., et al. (2023). Accommodation, social attraction, and intergroup attitudes on social media. _Language Sciences_, 99, 101563.

### Human-Robot/Agent Interaction
- Edwards, C., Edwards, A., & Rijhwani, V. (2023). When in doubt, lay it out: Over vs. under-accommodation in human-robot interaction. _Language Sciences_, 99, 101561.
- Riordan, M. A., & Kreuz, R. J. (in press). Humanizing AI agents using Communication Accommodation Theory. Peter Lang.

### Methodology
- Ireland, M. E., et al. (2011). Language style matching predicts relationship initiation and stability. _Psychological Science_, 22(1), 39–44.
- Soliz, J., & Berquist, G. (2016). Methods of CAT inquiry: Quantitative studies. In H. Giles (Ed.), _Communication accommodation theory_ (pp. 60–74). Cambridge University Press.
- Gallois, C., Ogay, T., & Giles, H. (2005). Communication Accommodation Theory: A look back and a look ahead. In W. B. Gudykunst (Ed.), _Theorizing about intercultural communication_ (pp. 121–148). Sage.
