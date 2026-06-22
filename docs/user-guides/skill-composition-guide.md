---
title: "hKask Skill Composition Guide — Multi-Skill Workflow Chains"
audience: [developers, agents, curators, architects]
last_updated: 2026-06-22
version: "0.30.0"
status: "Active"
domain: "Composition"
mds_categories: [composition, curation]
---

# hKask Skill Composition Guide

**Purpose:** Map end-to-end multi-skill workflows. While individual SKILL.mds declare composition affinities, this guide weaves them into complete chains — showing which skills compose, in what order, and why.

**Companion docs:** [`skill-user-guide.md`](skill-user-guide.md) (catalog and activation), [`skill-designer-guide.md`](../guides/skill-designer-guide.md) (creating skills), [`PRINCIPLES.md`](../architecture/core/PRINCIPLES.md)

---

## 1. Composition Principles

Skills compose because hKask's architecture places them in layers. Higher layers clarify perception; lower layers act on the clarified view.

### The Golden Rule

```
Never compose skills that operate at the same layer without a clear handoff contract.
Prefer vertical chains (perceptual → regulative → analytic → executive) over horizontal chains.
```

### The Iteration Mandate

**No real skill executed by a real expert is one-shot.** Writing is rewriting. Debugging is re-hypothesizing. Design is iterative deletion. A well-composed skill template is a FlowDef with an internal convergence loop, not a single-pass KnowAct prompt.

```
Skill quality = inner loop. A skill without internal iteration is a prompt, not a skill.
```

Every skill template must answer:

| Question | One-Shot Failure Mode | Recursive Fix |
|----------|----------------------|---------------|
| Does it converge? | Produces output and stops — quality is a single roll of the dice | Loop until quality threshold met OR max iterations exhausted. Exit with status (converged / maxed-out / escalated). |
| Does it narrow scope? | Re-processes the same input each pass — no learning | Each iteration narrows focus to the worst remaining violations, lowest-scoring dimension, most stale reference. |
| Does it admit failure? | Hallucinates confidence — every output is "complete" | Reports convergence status: `converged` (threshold met), `maxed_out` (iterations exhausted, best result returned), `escalated` (human needed). |

**Implementation pattern:** Use `FlowDef` template type with nested `choice` branching, not `KnowAct` prose that describes a loop it cannot execute. A FlowDef cascade can contain nested FlowDef/KnowAct/WordAct steps, bounded by the matryoshka limit of 7. The runtime's `ManifestExecutor` drives the cascade: render selector → LLM → parse JSON → follow chosen path. This is the machinery for real recursion.

| Template Type | Can It Loop? | Use For |
|--------------|-------------|---------|
| **FlowDef** | ✅ `choice` → branch → `escalate` → `abort` | Convergent quality loops (essentialist G1→G2→G3, gentle-lovelace score→rewrite→re-score) |
| **KnowAct** | ❌ Single-pass prompt | Metacognitive analysis within a FlowDef step ("evaluate this gate"), not the loop itself |
| **WordAct** | ❌ Single-pass prompt | Persona rendering, system prompts |

**Example — A recursive quality skill:**

```yaml
# FlowDef: gentle-lovelace-converge
select: quality_check
execute:
  - id: score
    type: KnowAct
    template: replica-report  # Score the document
  - id: check_threshold
    type: choice
    branches:
      - condition: "composite < 0.15"
        action: abort  # Quality met, exit
      - condition: "iteration >= max_iterations"
        action: escalate  # Maxed out, human review
      - condition: "default"
        action: continue  # Rewrite and re-score
  - id: rewrite
    type: KnowAct
    template: rewrite-weakest-dimension  # Fix the worst-scoring dimension
  - id: loop
    type: FlowDef  # Recursive: re-enter the score→check→rewrite→loop cascade
    select: quality_check
```

A skill whose template is a single KnowAct cannot converge — it can only produce output and stop. That is a prompt wearing a skill costume.

### The Skill-Kata Isomorphism

**Every skill is a specific application of the Improvement Kata with certain variables fixed to fit the domain.** The Improvement Kata is the general pattern: Understand Direction → Grasp Current Condition → Establish Target Condition → PDCA Iterate. A skill instantiates this pattern by fixing the variables:

| Kata Variable | General Form | Skill-Specific Form |
|--------------|-------------|-------------------|
| **Direction** | Challenge from level above | The skill's purpose (e.g., "eliminate pass-through abstractions") |
| **Current Condition** | Facts and data, not assumptions | Measurement of current state (e.g., cosine distance from exemplar) |
| **Target Condition** | Measurable, beyond current knowledge threshold | Quality threshold (e.g., composite < 0.15, zero Prohibition findings) |
| **PDCA Iterate** | Rapid experiments toward target | Inner loop: evaluate → narrow → improve → re-evaluate |
| **Exit** | Target reached OR obstacle blocks | `converged` (excellence) OR `maxed_out` (energy exhaustion) |

Skills bring mastery. Mastery enforces excellence. To be a valid skill in the hKask definition, a template MUST:

```
1. Be a FlowDef (not KnowAct, not WordAct) — only FlowDef has the choice/escalate/abort/loop machinery
2. Declare a measurable quality target (convergence.threshold) — what "done" means
3. Declare an energy budget (gas.cap) — the maximum resource expenditure
4. Contain a PDCA loop (evaluate → check → improve → loop) — the iterative improvement ratchet
5. Exit on excellence (threshold met → abort) OR energy exhaustion (max_iterations or gas exceeded → maxed_out/escalate)
6. Report which exit condition fired (convergence.converged | maxed_out | escalated) and what was achieved
```

A template that lacks any of these six properties is not a skill — it is a prompt (KnowAct), a persona (WordAct), or a tool invocation. It may be useful. It may be composed INTO a skill as a step within the PDCA loop. But it is not itself a valid hKask skill.

The two exit rails — quality threshold and energy budget — form the ratchet. The skill cannot exit until it has either achieved its quality goal or exhausted its energy allocation. Every iteration either improves quality (moving toward the threshold) or consumes budget (moving toward exhaustion). The skill converges on one or the other; it cannot run forever.

### Shared Infrastructure

`constraint-forces` runs across **all** layers and all compositions. Prohibitions and Guardrails are never relaxed, regardless of which skills are in the chain.

---

## 2. Primary Composition Chains

### Chain 1: Perception → Analysis → Action

```
dokkodo-mindset → pragmatic-laziness → essentialist → coding-guidelines
     ↑                                                    ↑
constraint-forces (runs across all, never relaxed)
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Perceive** | `dokkodo-mindset` | Clear attachment, preference, resentment, and fear from the perceptual field |
| **Analyze** | `pragmatic-laziness` | Decompose the situation, map feedback loops, find δS = 0 path |
| **Delete** | `essentialist` | Apply 3-gate eliminative interrogation — Exist → Surface → Contract |
| **Execute** | `coding-guidelines` | Enforce Karpathy principles: think first, simplicity, surgical, goal-driven |
| **Enforce** | `constraint-forces` | Running across all stages, classify every boundary |

**Use when:** Design decisions, architecture reviews, code simplification, "what should I delete?"

**Example session:**
```
> apply the Dokkodo — I'm attached to this module we built
> now be lazy about this module
> simplify what remains
> use coding-guidelines for the implementation
```

---

### Chain 2: Forecast → Decide → Record → Verify

```
superforecasting → mcda → decision-journal → goal-analysis
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Forecast** | `superforecasting` | 8-stage calibrated probability pipeline — triage, Fermi, outside/inside view, Bayesian, dragonfly-eye |
| **Decide** | `mcda` | Identify criteria, weight and score alternatives, detect compensation masking, sensitivity analysis |
| **Record** | `decision-journal` | Record reasoning, assumptions, alternatives, emotional state. Define expected outcomes with time horizons. |
| **Verify** | `goal-analysis` | Track whether the outcome matches the prediction. Judge completion with confidence scoring. |

**Use when:** Consequential decisions under uncertainty, strategic choices with multiple criteria, decisions you need to learn from

**Example session:**
```
> superforecast: will switching to Rust reduce our bug rate by 50% within 6 months?
> MCDA: compare staying on Python vs. partial Rust rewrite vs. full Rust rewrite
> journal this decision — record the reasoning and schedule a 6-month revisit
> create a goal: complete Rust migration with bug rate tracked weekly
```

---

### Chain 3: Diagnose → Extract → Fix → Harden

```
diagnose → structured-extraction → refactor-service-layer → adversarial-red-team
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Diagnose** | `diagnose` | Reproduce, anchor to spec, hypothesize, instrument, fix — the disciplined loop |
| **Extract** | `structured-extraction` | Map the incident narrative to a root cause schema — what failed, why, and how |
| **Fix** | `refactor-service-layer` | Strangler fig extraction — new implementation alongside old, incrementally replace |
| **Harden** | `adversarial-red-team` | Test the fix against injection, hijacking, exfiltration, and tool misuse |

**Use when:** Something broke, and you need to fix it AND prevent recurrence

**Example session:**
```
> diagnose this crash in the auth module
> extract structured data from the incident report into a root cause schema
> refactor the auth service layer to isolate the vulnerable path
> red-team the new auth module before deploying
```

---

### Chain 4: Explore → Summarize → Compress

```
zoom-out → chain-of-density → caveman
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Explore** | `zoom-out` | Get the bigger picture — how does this code fit into the architecture? |
| **Densify** | `chain-of-density` | Iterative density-increase summarization — pack maximum entities into fixed word count |
| **Compress** | `caveman` | Final stylistic compression — drop filler, articles, hedging, preserve substance |

**Use when:** Understanding a large unfamiliar codebase and communicating it concisely

**Example session:**
```
> zoom out on the condenser crate
> chain-of-density: summarize the condenser architecture at 200 words
> now caveman that summary
```

---

### Chain 5: Plan → Critique → Revise → Evaluate

```
scenario-builder → grill-me → self-critique-revision → gentle-lovelace
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Plan** | `scenario-builder` | Schwartz method — focal question, STEEP, 2×2 matrix, robust strategies |
| **Stress-test** | `grill-me` | Socratic interrogation of each scenario's assumptions and strategies |
| **Revise** | `self-critique-revision` | Iterative draft → critique → revise cycle on the strategy document |
| **Evaluate** | `gentle-lovelace` | Score the final strategy document against 4 dimensions of writing quality |

**Use when:** Strategic planning, futures work, decision documents that must withstand scrutiny

**Example session:**
```
> build scenarios for our product strategy over the next 5 years
> grill me on the Wild West scenario — what assumptions are weakest?
> self-critique the strategy document
> gentle lovelace: evaluate the final strategy doc
```

---

### Chain 6: Hunt Bugs — Perceive Semantics → Analyze Loops → Probe → Report

```
pragmatic-semantics → pragmatic-cybernetics → bug-hunt
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Classify** | `pragmatic-semantics` | Classify every finding: IS vs OUGHT, declarative vs probabilistic vs subjunctive. Trace provenance. Never present speculation as fact. |
| **Analyze loops** | `pragmatic-cybernetics` | Treat target code as a feedback system. Check polarity, delay, gain, closure, fidelity. Good Regulator check. Variety analysis. |
| **Hunt** | `bug-hunt` | Charter → Probe → Oracle → Taxonomize → Report. Weinberg quality, Beizer taxonomy, Hendrickson exploratory tours. |

**Use when:** Bug hunting expeditions, code quality audits, finding semantic errors and interaction bugs

**Example session:**
```
> hunt bugs in hkask-wallet — quality criteria: financial invariants
> (agent applies pragmatic-semantics to classify each finding)
> (agent applies pragmatic-cybernetics to analyze wallet as feedback system)
> (agent runs bug-hunt expedition with charter, probe, oracle, taxonomy, report)
```

---

### Chain 7: Skill Lifecycle — Discovery → Audit → Maintain

```
skill-discovery → skill-logic-audit → skill-maintenance
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Discover** | `skill-discovery` | Find skills matching a capability gap |
| **Audit** | `skill-logic-audit` | Check template logic against stated goals |
| **Maintain** | `skill-maintenance` | Score skill health, detect staleness, drift, and broken references |

**Use when:** Managing the skill corpus

---

### Chain 8: Resilience — Accept → Stabilize → Diagnose

```
dokkodo-mindset → diagnose → improve-codebase-architecture
```

| Stage | Skill | What happens |
|-------|-------|-------------|
| **Accept** | `dokkodo-mindset` | Accept things exactly as they are — the error happened, the system failed |
| **Diagnose** | `diagnose` | Systematic diagnosis without panic or resentment distorting the process |
| **Improve** | `improve-codebase-architecture` | Find deepening opportunities — was this failure caused by shallow architecture? |

**Use when:** Error states, constraint violations, adversarial input — the American Ronin resilience pattern

---

## 3. Composition by Problem Domain

| I need to... | Chain |
|-------------|-------|
| Simplify a design | Chain 1: Perception → Analysis → Action |
| Make a strategic decision | Chain 2: Forecast → Decide → Record → Verify |
| Fix a bug properly | Chain 3: Diagnose → Extract → Fix → Harden |
| Hunt bugs in code — semantic errors, interaction bugs | Chain 6: Hunt Bugs — Perceive Semantics → Analyze Loops → Probe → Report |
| Understand and explain code | Chain 4: Explore → Summarize → Compress |
| Write a strategy document | Chain 5: Plan → Critique → Revise → Evaluate |
| Respond to an error | Chain 8: Resilience — Accept → Stabilize → Diagnose |
| Manage the skill corpus | Chain 7: Skill Lifecycle |
| Improve agent capability | `kata-starter` → `kata-improvement` → `kata-coaching` |
| Create a logo | `logo-builder` |
| Update documentation | `document-update` |

---

## 4. Composition Anti-Patterns

| Anti-pattern | Why it fails | Fix |
|-------------|-------------|-----|
| **Single-shot KnowAct skill** | Skill template is a KnowAct with no internal loop. Describes recursion in prose but cannot execute it — the template type lacks `choice`/`escalate`/`abort` machinery. Produces output and stops; quality is a single dice roll. | Redesign as FlowDef with nested KnowAct steps inside a convergence loop. Set explicit quality threshold and max iterations. Exit with status. |
| Skipping perception before analysis | Pragmatic-laziness evaluates a distorted landscape | Always run `dokkodo-mindset` first for consequential decisions |
| MCDA without superforecasting criteria | "Cost" and "risk" become uncalibrated guesses | Run `superforecasting` first on risk-related criteria |
| Caveman before chain-of-density | Caveman drops entities for style; chain-of-density preserves them | Always densify first, compress second |
| Decision journal without revisit scheduling | The journal becomes a log, not a calibration tool | Always define time horizons and resolution criteria |
| Red-team without diagnose | You know it's vulnerable but not why | Diagnose the vulnerability before hardening |

---

## 5. Maintaining This Guide

When new skills are created or composition affinities change:

1. Update the relevant chain(s) in §2
2. Add new problem→chain entries in §3
3. Check for new anti-patterns in §4
4. Verify that every skill in a chain has declared its composition affinities in its SKILL.md `## Composition` section
5. CNS composition tracking (§6) may reveal actual composition patterns that differ from designed patterns — update accordingly

---

## 6. CNS Composition Tracking (v0.31+ Design)

When CNS spans include a `composed_with` field on skill invocation events, this guide can become data-driven:

```bash
kask skill compose-stats --period 30d
```

Would surface:
- **Actual composition frequency** — which skills are invoked together most often
- **Composition gaps** — skills that declare composition affinities but are never composed
- **Emergent chains** — compositions that appear in practice but aren't documented

The "Essential Five" in `skill-user-guide.md` should be derived from composition frequency, not curated manually.

---

## References

- [`skill-user-guide.md`](skill-user-guide.md) — Skill catalog and activation
- [`skill-designer-guide.md`](../guides/skill-designer-guide.md) — Creating and maintaining skills
- [`PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) — P1–P12 architecture principles
- [`dokkodo-user-guide.md`](dokkodo-user-guide.md) — Perceptual layer deep dive
