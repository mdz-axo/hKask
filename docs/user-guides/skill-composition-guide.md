---
title: "hKask Skill Composition Guide — Multi-Skill Workflow Chains"
audience: [developers, agents, curators, architects]
last_updated: 2026-06-19
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
| Manage the skill corpus | Chain 6: Skill Lifecycle |
| Improve agent capability | `kata-starter` → `kata-improvement` → `kata-coaching` |
| Create a logo | `logo-builder` |
| Update documentation | `document-update` |

---

## 4. Composition Anti-Patterns

| Anti-pattern | Why it fails | Fix |
|-------------|-------------|-----|
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
