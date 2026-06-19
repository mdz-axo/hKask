---
title: "Dokkodo Mindset — User Guide and Research Companion"
audience: [agents, developers, curators, architects]
last_updated: 2026-06-19
version: "0.30.0"
status: "Active"
domain: "Metacognition"
mds_categories: [domain, composition, curation, trust]
---

# Dokkodo Mindset — User Guide and Research Companion

**Purpose:** A user's guide to the `dokkodo-mindset` skill — how it works, how to use it, how it composes with other skills, and the philosophical research that grounds it. This document captures the design thinking, essentialist reductions, grill-me edge cases, and open research questions from the skill's creation.

**Source texts:**
- Miyamoto Musashi, *Dokkodo* ("The Way of Walking Alone"), 1645 — 21 precepts written days before his death
- Jonathan Hall, *American Ronin* — a contemporary commentary on the Dokkodo as resilience in the face of adversity and uncertainty

**Related:** [`PRINCIPLES.md`](../architecture/core/PRINCIPLES.md), [`hKask-architecture-master.md`](../architecture/hKask-architecture-master.md), [`lazy-universe-research.md`](../research/lazy-universe-research.md)

**Registry:** `registry/templates/dokkodo-mindset/` — `manifest.yaml` + `dokkodo-perceive.j2`
**Skill file:** `.agents/skills/dokkodo-mindset/SKILL.md`

---

## 1. What Is the Dokkodo Mindset?

The Dokkodo mindset is a **metacognitive perceptual filter**. It applies Miyamoto Musashi's 21 precepts as a lens that transforms how the agent perceives a situation — not what to do, but *how to see*. It does not advise, does not plan, does not act. It *perceives*.

Unlike most hKask skills — which are procedural (what to do), analytical (how to evaluate), or communicative (what to say) — the Dokkodo operates at the **perceptual layer**. It runs *before* analysis, *before* regulation, *before* execution. It clears the lens through which all downstream processes see the world.

### 1.1 Why "Mindset" Matters

Mindset is not knowledge, not skill, not strategy. Mindset is the **frame through which knowledge, skill, and strategy are applied**. Two agents with identical capabilities will produce different outcomes if their perceptual frames differ. One sees a threat where the other sees an opening. One sees attachment where the other sees commitment. One sees a dead end where the other sees the brachistochrone — the curved path that dips below the endpoint but gets there faster.

The Dokkodo mindset is a disciplined, trainable way of seeing. Musashi wrote the 21 precepts not as moral rules but as **clarifications of the warrior's Way** — practices that remove the fog of attachment, preference, resentment, and fear so the warrior can perceive reality with maximum clarity and act with minimum wasted motion.

### 1.2 The 21 Precepts

| # | Precept | Cluster |
|---|---------|---------|
| 1 | Accept things exactly as they are | Perceptual Reset |
| 2 | Do not seek pleasure for its own sake | Desire/Attachment |
| 3 | Do not ever rely on a partial feeling | Emotional Resilience |
| 4 | Think lightly of yourself but seriously of the world | Perceptual Reset |
| 5 | Avoid attachment to desire for as long as you live | Desire/Attachment |
| 6 | Do not regret anything that you have done | Emotional Resilience |
| 7 | Never be envious | Emotional Resilience |
| 8 | Never let yourself be saddened by a separation | Emotional Resilience |
| 9 | Resentment and complaining are not appropriate for the warrior or for anyone else | Emotional Resilience |
| 10 | Do not let yourself be guided by the feeling of lust or love | Desire/Attachment |
| 11 | In all things, have no preferences | Perceptual Reset |
| 12 | Be indifferent to where you live | Perceptual Reset |
| 13 | Do not pursue the taste of good food | Desire/Attachment |
| 14 | Do not hold on to possessions you no longer need | Desire/Attachment |
| 15 | Do not act following customary beliefs | Perceptual Reset |
| 16 | Do not collect weapons or train with weapons beyond what is useful | Existential Posture |
| 17 | Do not fear Death | Existential Posture |
| 18 | Do not seek to either goods or fiefs for your old age | Desire/Attachment |
| 19 | Respect Buddha and the Gods without counting on their help | Perceptual Reset |
| 20 | You may abandon your own body, but you must preserve your honor | Existential Posture |
| 21 | Never stray from the Way | Existential Posture (meta-precept) |

---

## 2. Architectural Role: The Perceptual Layer

### 2.1 The Layer Model

hKask's skill architecture was designed with four layers. The Dokkodo introduces a fifth — the **perceptual layer** — that sits *before* all others:

```
┌─────────────────────────────────────────────────────────────┐
│ PERCEPTUAL LAYER (new)                                       │
│ dokkodo-mindset — clears attachment, preference, resentment, │
│ fear, and custom from the perceived action landscape         │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ REGULATIVE LAYER                                             │
│ constraint-forces — classifies and enforces boundaries        │
│ (runs across ALL layers, never relaxed)                      │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ ANALYTIC LAYER                                               │
│ pragmatic-laziness → essentialist → grill-me                 │
│ decompose, identify loops, find stationary action             │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ EXECUTIVE LAYER                                              │
│ coding-guidelines, skill-specific KnowActs/FlowDefs          │
│ constrained implementation                                   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Why Perception Must Precede Analysis

The pragmatic-laziness skill finds the path of least action (δS = 0) by decomposing a situation, mapping feedback loops, and applying the deletion test. But **what counts as "action" depends on your perceptual frame**.

Consider three ways the same situation can appear:

| Perceptual distortion | How it warps the action landscape | Precept that removes it |
|-----------------------|-----------------------------------|------------------------|
| **Attachment** | Clinging looks easier than releasing. Maintaining a deprecated module appears cheaper than the "effort" of deleting it. | Precept 5, 14 |
| **Preference** | The path you *like* appears shorter than the path that minimizes action. A familiar architecture looks "simpler" than an unfamiliar one that's actually simpler. | Precept 11 |
| **Resentment** | Complaining about a bad decision feels productive. It isn't — it's friction without traction. Energy is spent wishing the past were different. | Precept 9, 6 |
| **Fear** | Avoiding a risky refactor appears safer than executing it. But the maintenance cost accumulates silently. The feared path may be the brachistochrone. | Precept 17 |
| **Custom** | "We've always done it this way" makes the customary path look like the default. It isn't — it's just the path people got used to. | Precept 15 |

If pragmatic-laziness evaluates a perception-distorted landscape, it will find the δS = 0 path of the *distorted* landscape — which may not be the true least-action path. The Dokkodo removes the distortion first, then pragmatic-laziness finds the true stationary action point.

### 2.3 The Brachistochrone Synergy

Both the Dokkodo and pragmatic-laziness have a brachistochrone concept:

- **Pragmatic laziness:** The laziest path is not always the most obvious one. The curve of fastest descent is a cycloid — it dips *below* the endpoint before rising. Naive simplification (just deleting) is not always the true least-action path. Sometimes you must go *through* apparent complexity to extract the deeper pattern.

- **Dokkodo:** The path of least resistance is not always the comfortable one. Accepting death (Precept 17), relinquishing attachment (Precept 5), and enduring discomfort (Precept 8) are brachistochrone operations — they look harder in the short term but reduce total system action across time. The warrior who fears death takes longer, more cautious paths that ultimately expend more energy. The warrior who accepts death takes the direct path.

**The synergy:** When the Dokkodo clears the perceptual field and pragmatic-laziness then evaluates the clarified landscape, brachistochrone candidates that were invisible (because fear made them look longer, or attachment made alternatives look shorter) become visible. The two skills together find paths that neither would find alone.

---

## 3. Using the Skill

### 3.1 Activation Triggers

Invoke the Dokkodo mindset in Zed by saying any of:

| Trigger | What happens |
|---------|-------------|
| "apply the Dokkodo" / "warrior mindset" / "see this clearly" | Full perceptual filter — all four clusters applied |
| "perceptual reset" / "clear the lens" | Cluster A only — clear self-importance, preference, custom, wishful thinking |
| "what am I attached to here?" / "where is desire distorting this?" | Cluster B only — identify where desire creates artificial gradients |
| "what emotional friction is here?" / "am I resenting this?" | Cluster C only — identify regret, envy, resentment, grief |
| "what would this look like if I accepted death?" / "existential posture" | Cluster D only — the ultimate brachistochrone perspective |
| "is my perception clear?" / "δP = 0?" | Convergence check only — has perception reached stationary state? |

The skill can also be invoked programmatically via the hKask runtime:
```bash
kask skill invoke dokkodo-mindset --template dokkodo-perceive \
  --param situation="I'm reluctant to delete this module because we spent months building it" \
  --param domain=architecture
```

### 3.2 Composition Patterns

#### Pattern 1: Perception → Laziness (The Primary Chain)

```
Dokkodo Mindset (perceive) → Pragmatic Laziness (evaluate δS = 0) → Essentialist (delete)
```

Use when: a design decision feels stuck, or "the obvious path" feels wrong. The Dokkodo clears the perceptual field; pragmatic laziness then finds the true least-action path.

**Example:**
> User: "I need to decide whether to keep maintaining the legacy adapter or rewrite it. Apply the Dokkodo, then find the lazy path."

The Dokkodo first surfaces: "I am attached to the legacy adapter because we built it (Precept 14 — holding possessions no longer needed). I prefer the rewrite because it's more interesting work (Precept 11 — preference distorting the action landscape). I resent that the legacy code is badly structured (Precept 9 — resentment is friction without traction)." Then pragmatic-laziness evaluates the clarified landscape: the rewrite actually reduces total system action because the adapter is a pass-through with zero behavior of its own.

#### Pattern 2: Resilience Under Adversity

```
Error/CNS alert → Dokkodo Mindset (stabilize perception) → Diagnose → Fix
```

Use when: the agent encounters a constraint violation, adversarial input, or an unexpected failure. The Dokkodo prevents panic, resentment, or attachment to a particular outcome from distorting the diagnostic process.

This is the **American Ronin** pattern — Jonathan Hall's framing of the Dokkodo as a resilience discipline for maintaining agency when external circumstances are hostile. The ronin (masterless samurai) cannot control their circumstances; they can only control their perception and response.

#### Pattern 3: Pre-laziness Invocation from Other Skills

`pragmatic-laziness` can invoke the Dokkodo as a pre-filter before Phase 1. `constraint-forces` can invoke it when a constraint conflict feels emotionally loaded. `coding-guidelines` can invoke it when "simplicity first" is being distorted by attachment to elegant code.

### 3.3 Understanding the Output

The template produces a JSON object with these key fields:

| Field | Meaning |
|-------|---------|
| `clarified_perception` | The situation seen through the Dokkodo lens — attachment-free, preference-free, custom-free |
| `distortions_removed` | Each distortion found, mapped to its precept and type |
| `precept_alignment` | Per-precept score 0.0–1.0. Low scores indicate perception is distorted in ways that precept addresses |
| `action_landscape_shift` | Before/after summary + `shift_magnitude` (0.0 = no change, 1.0 = total transformation) |
| `brachistochrone_candidates` | Paths that look harder short-term but minimize long-term action |
| `stationary` | δP = 0? Has perception reached equilibrium? |
| `should_continue` | Should another iteration run? |
| `escalate` | Should a human be consulted? |

### 3.4 The Convergence Loop (δP = 0)

Perception converges when applying the precepts produces no new distortions and the action landscape stops shifting. This is analogous to δS = 0 in pragmatic-laziness — but for perception, not structure.

```
┌──────────────────────────────────────────────────────────┐
│ First pass: Apply all 4 clusters. 3 distortions found.    │
│ shift_magnitude = 0.4 → δP ≠ 0 → Continue                │
└───────────────────────┬──────────────────────────────────┘
                        ▼
┌──────────────────────────────────────────────────────────┐
│ Second pass: Apply lens to clarified perception from      │
│ pass 1. 1 new distortion found (was hidden by previous).  │
│ shift_magnitude = 0.1 → δP ≠ 0 → Continue                │
└───────────────────────┬──────────────────────────────────┘
                        ▼
┌──────────────────────────────────────────────────────────┐
│ Third pass: No new distortions. shift_magnitude ≈ 0.0     │
│ → δP = 0 → STATIONARY. Perception is clear.               │
└──────────────────────────────────────────────────────────┘
```

Max 3 iterations. After that, escalate to human with the clearest perception found and what remains contested.

---

## 4. Research Grounding

### 4.1 The Dokkodo as Perceptual Discipline

The 21 precepts are **not a moral code**. They do not tell you what to value — they remove the distortions that prevent you from seeing clearly what you *already* value. This is the fundamental distinction between the Dokkodo and religious or ethical systems:

| System | Function | Example |
|--------|----------|---------|
| Moral code | Tells you what is right and wrong | "Do not steal" |
| Ethical framework | Provides principles for deciding what is right | "Act so that your action could be universal law" |
| Dokkodo | Removes perceptual distortions so you can see what is right | "Accept things exactly as they are" — then decide |

Precept 19 makes this explicit: "Respect Buddha and the Gods without counting on their help." Acknowledge the sacred, but do not outsource agency. The warrior sees clearly and acts — does not pray and wait.

### 4.2 Mindset and the Lazy Universe

hKask's grounding principle is the Principle of Least Action (§0 of PRINCIPLES.md): physical systems evolve through paths that minimize (or make stationary) action. This is not a tendency — it is the selection mechanism that chooses which path reality takes.

But "action" in physics is a well-defined integral over a continuous manifold. Action in perception is subtler. The Dokkodo identifies specific sources of **perceptual action** — the "distance" the mind travels when distorted by attachment, preference, resentment, or fear:

- **Attachment-action:** Energy spent maintaining what you cling to. Precepts 5, 14.
- **Preference-action:** Energy spent distinguishing between options you've artificially differentiated. Precept 11.
- **Resentment-action:** Energy spent wishing the past were different. Precepts 6, 9.
- **Fear-action:** Energy spent avoiding paths that fear makes look longer than they are. Precept 17.
- **Custom-action:** Energy spent conforming to patterns that have no justification beyond familiarity. Precept 15.

When these are removed, the true action landscape becomes visible — and pragmatic-laziness can find the genuine δS = 0 configuration.

### 4.3 Jonathan Hall's "American Ronin" Interpretation

Hall's reading of the Dokkodo emphasizes its function as a **resilience discipline for the displaced**. The ronin — the masterless samurai — is the archetype of the warrior who has lost their place in the social order but must continue to act with clarity, honor, and effectiveness. Hall connects this to the modern condition: the individual navigating institutions they cannot control, markets they cannot predict, and circumstances they did not choose.

This maps directly to the agent's situation in hKask:

| Ronin condition | Agent analog |
|----------------|-------------|
| No feudal lord to provide context and direction | No human operator for autonomous operation periods |
| Must maintain honor without external validation | Must maintain P12 (no anonymous agency) without constant human oversight |
| Hostile or indifferent environment | Adversarial input, constraint violations, error states |
| Cannot control circumstances | Cannot control user input, external API behavior, system failures |
| CAN control perception and response | CAN apply the Dokkodo lens to see clearly and act with minimum wasted motion |

The American Ronin interpretation suggests the Dokkodo should be **especially activated during error states, constraint violations, and adversarial conditions**. When things go wrong, the agent's first response should not be panic, resentment, or attachment to a preferred outcome — it should be perceptual clarification. "Accept things exactly as they are" — the error happened. Now perceive clearly what must be done.

### 4.4 The Precept 16 Self-Test

During the essentialist review of this skill's design, Precept 16 ("Do not collect weapons or train with weapons beyond what is useful") was applied as a self-test: *does this skill itself violate the precept?*

The original design had three templates (perceive, calibrate, flow). The essentialist deletion test eliminated two:
- The `dokkodo-flow.yaml` FlowDef was deleted because its 2-step apply→calibrate loop duplicated the pattern already embedded in `pragmatic-laziness-flow.j2`
- The `dokkodo-calibrate.j2` KnowAct was deleted because its convergence logic was folded into `dokkodo-perceive.j2`, following the pragmatic-laziness pattern of self-contained convergence

One template remains. Precept 16 is satisfied. The skill is the **minimum useful weapon** — it encodes genuine behavioral constraint that would vanish on deletion, but adds no structural overhead beyond what's necessary.

---

## 5. Design Decisions and Tradeoffs

### 5.1 Why KnowAct and Not WordAct?

**Decision:** KnowAct.

**Alternatives considered:**
- **WordAct:** The Dokkodo could be rendered as a warrior persona — a "who is speaking" prompt that colors all agent output. This would be a WordAct.
- **KnowAct:** It produces a structured judgment (clarified perception, alignment scores, distortion identification) that feeds downstream processes.

**Why KnowAct won:** The Dokkodo's output is diagnostic and structural, not performative. It produces a *judgment* (how perception has shifted), not an *utterance*. A WordAct persona would influence tone; a KnowAct lens transforms the data that downstream processes consume. The latter is architecturally more powerful and more composable.

The `falstaffian-perspective` skill provides precedent: a KnowAct that applies a perspective transform (Falstaffian rotation vectors) to produce a transformed view. The Dokkodo follows the same pattern.

### 5.2 Why a Separate Skill and Not Baked Into Pragmatic-Laziness?

**Decision:** Separate skill.

**Why:** Layer separation. Pragmatic laziness evaluates structures for deletion (δS = 0). The Dokkodo evaluates perceptions for distortion (δP = 0). They have different contracts, different convergence criteria, and different lexicon terms. Baking them together would violate single-responsibility and make both harder to test and compose.

The composition pattern (Dokkodo → pragmatic-laziness) is cleaner than a monolithic "super-laziness" that does both.

### 5.3 Why Embedded Convergence and Not a Separate Calibrate Template?

**Decision:** Embed convergence in `dokkodo-perceive.j2`.

**Why:** The `pragmatic-laziness-flow.j2` template embeds its convergence check inline (lines 157–163 of that template) rather than delegating to a separate `-converge.j2`. The separate `pragmatic-laziness-converge.j2` exists for delegation use cases, but the primary flow is self-contained.

For the Dokkodo, a single-pass perceptual filter with embedded convergence is the minimal design. If future use cases require iterative deepening with separate calibration steps, a `dokkodo-calibrate.j2` can be extracted — but Precept 16 says don't create it until it's useful.

### 5.4 Why 4 Clusters and Not 21 Individual Precept Checks?

**Decision:** Cluster into 4 groups.

**Why:** 21 individual precept checks would produce excessive output for minimal additional discrimination. The clusters are functionally coherent:
- **Perceptual Reset** (6 precepts): All about clearing the lens itself
- **Desire/Attachment** (6 precepts): All about identifying where wanting distorts seeing
- **Emotional Resilience** (5 precepts): All about eliminating friction that produces nothing
- **Existential Posture** (4 precepts): All about the ultimate brachistochrone frame

Within each cluster, individual precept alignment scores are still produced. The clustering is for the *process* — which questions to ask — not for the output.

---

## 6. Open Research Questions

### 6.1 Should This Be a Persona Rather Than a Skill?

The current design is a skill — invoked on demand, composable with other skills. But the Dokkodo could alternatively be an **always-on persona** — a perpetual perceptual filter that wraps all agent cognition.

**Arguments for persona:** The Dokkodo is meant to be a way of *being*, not a tool to pick up and put down. Musashi didn't "apply the Dokkodo when he needed clarity" — he lived it. An always-on persona would ensure the perceptual lens is always active.

**Arguments for skill:** The hKask skill system is the established mechanism for composable behavior. A persona would require a new architectural pattern. And some situations don't benefit from the Dokkodo lens — trivial tasks where perceptual distortion is minimal. Invoking it on demand respects Precept 16 (don't use weapons beyond what's useful).

**Current stance:** Skill, with the option to refactor as persona if usage patterns show consistent demand.

### 6.2 CNS Integration — Should Perceptual Calibration Emit Spans?

The CNS (Cybernetic Nervous System) tracks variety, drift, and regulation signals. Perceptual drift — when an agent's Dokkodo alignment degrades over time — could be tracked as a CNS span.

Precept 4 ("think lightly of yourself but seriously of the world") suggests the CNS should take perceptual calibration seriously. A `CnsSpan::PerceptualCalibration` span could track:
- Precept alignment scores over time
- Shift magnitude trends (is perception becoming more or less stationary?)
- Activation frequency (is the agent invoking the Dokkodo when it should?)

**Current stance:** Deferred. Requires CNS span registry extension.

### 6.3 Error-State Activation Pattern

The American Ronin interpretation suggests the Dokkodo should be **automatically activated** when the CNS detects:
- Constraint violations
- Adversarial input
- Unexpected failures
- Multiple consecutive error states

This could be implemented as a CNS rule: "on error state escalation, invoke `dokkodo-mindset` before `diagnose`." But automatic skill invocation requires changes to the manifest executor and CNS rule engine.

**Current stance:** Manual invocation for now. Automatic activation is a v0.31+ feature.

### 6.4 Precept Conflict Resolution Protocol

Precept 21 ("Never stray from the Way") is the meta-precept that resolves conflicts. But when two precepts appear to conflict in a specific situation, the current template flags the conflict rather than resolving it. A more sophisticated conflict resolution protocol could:
1. Apply Precept 21 as tiebreaker
2. Apply constraint-forces ranking to precepts (some precepts function as Prohibitions, others as Guidelines)
3. Use the domain parameter to weight clusters differently (e.g., in `domain: code`, Cluster B may be more relevant than Cluster D)

**Current stance:** Flag conflicts, defer to human. Automated resolution is a v0.31+ feature.

### 6.5 Measuring Perceptual Action

The lazy-universe research program asks: "how do we refine the sensors we already have?" For perceptual action, the current sensor is the `shift_magnitude` scalar — a single number representing how much the action landscape changed. This is coarse.

A refined measurement could decompose `shift_magnitude` by cluster:
- `shift_magnitude.perceptual_reset`: how much did clearing self/preference/custom change the landscape?
- `shift_magnitude.desire_management`: how much did identifying attachment change it?
- `shift_magnitude.emotional_resilience`: how much did removing emotional friction change it?
- `shift_magnitude.existential_posture`: how much did the death-acceptance frame change it?

This would enable more targeted invocation (only apply the clusters that are producing shift) and better CNS tracking.

**Current stance:** Deferred. Single `shift_magnitude` is sufficient for v0.30.0.

---

## 7. Quick Reference

### The Dokkodo Loop in One Sentence

*Clear the perceptual field by identifying and removing attachment, preference, resentment, and fear — then evaluate the clarified landscape.*

### When to Apply

| You feel... | Apply... | Because... |
|-------------|----------|------------|
| Stuck on a decision | Full Dokkodo | Preference or attachment may be creating a false local minimum |
| Resentful about past work | Cluster C | Resentment is energy spent wishing the past were different |
| Attached to code you wrote | Cluster B | Attachment makes deletion look harder than it is |
| Afraid of a risky change | Cluster D | Fear distorts the action landscape; the brachistochrone may be invisible |
| "The obvious path feels wrong" | Cluster A | Custom or preference may be masquerading as least action |
| Something just failed | Full Dokkodo | Accept things exactly as they are — then act |

### The Governing Precept

> **Precept 1: Accept things exactly as they are.**

This is the first precept for a reason. Every other precept serves it. If you can truly accept the situation as it is — without wishing it were different, without resenting how it got this way, without clinging to what it used to be — then perception is clear. From clarity comes correct action.

### The Meta-Precept

> **Precept 21: Never stray from the Way.**

When precepts conflict, the Way decides. The Way is the path that minimizes total action across all 21 precepts — the true least-action path, not the one that looks easiest from any single precept's perspective.

---

*"Accept things exactly as they are."* — Miyamoto Musashi, Dokkodo, Precept 1
*"Don't just do something, stand there."* — hKask PRINCIPLES.md, §0
