---
name: pragmatic-cybernetics
visibility: public
description: "Cybernetic reasoning framework for analyzing hKask's CNS, feedback loops, variety engineering, and system homeostasis. Use when diagnosing CNS alerts, analyzing feedback loop failures, evaluating variety deficits, or reasoning about the system's self-regulation architecture. Pairs with constraint-forces for enforcement-level decisions."
---

# Pragmatic Cybernetics

A framework for reasoning cybernetically about hKask's homeostatic self-regulation system. Cybernetics isn't abstract theory here — the CNS *is* a cybernetic system, and understanding its structure helps diagnose failures before they scream.

## hKask as a Cybernetic System

Every cybernetic system has five components. Here is hKask's:

| Component | hKask Implementation | What It Does |
|-----------|---------------------|-------------|
| **Sensor** | MCP tool dispatch (`cns.tool.<subsystem>` spans) | Collects observations — tool calls, inference results, prompt outcomes |
| **Model** | hkask-storage (SQLite + SQLCipher) + CNS variety counters | Remembers what was seen. ν-events are the canonical observations. Variety counters track diversity. |
| **Regulator** | CNS homeostatic loop + Curator Agent | Compares current variety to threshold. Algedonic alerts when deficit exceeds limits. |
| **Actuator** | OCAP-governed MCP dispatch | Actions gated by capability tokens. The regulator recommends; OCAP enforces boundaries. |
| **Observer-of-observer** | CNS algedonic signals (`cns.cybernetics.backpressure`) | "Is the system regulating itself?" Second-order cybernetics. |

The feedback loop:

```
Agent activity → MCP tools (sensor) → ν-events (model) → CNS variety counter (comparator)
    → Curator (regulator) → OCAP-gated actions (actuator) → Agent activity
```

The Curator sits in the regulator box. It compares variety against thresholds and escalates. It never bypasses OCAP — that's the Magna Carta contract.

## The Viable System Model of hKask

| VSM System | hKask Component | Function |
|------------|-----------------|----------|
| **S1 (Operations)** | MCP tool dispatch, agent pods | Primary activity: agents do work |
| **S2 (Coordination)** | Communication loop + `Arc<AtomicU64>` backpressure | Anti-oscillation: queue depth monitoring, backpressure at threshold |
| **S3 (Control)** | CNS variety counters + algedonic thresholds | "Is this normal?" Threshold comparison (50 Warning, 100 Critical) |
| **S3\* (Audit)** | `kask sovereignty verify` + ad-hoc CNS queries | Sporadic direct probe, bypassing cached state |
| **S4 (Intelligence)** | Curator Agent + LLM inference | "What could this mean? What's coming?" |
| **S5 (Policy)** | Magna Carta P1–P4 + OCAP constraints | Identity, constraints, refusal posture |

The recursion principle: every component should be viable at its own level. The Curator is viable if it can observe, compare, and escalate. An agent pod is viable if it can execute within its OCAP boundary. If any component lacks its own feedback loop, it is not viable — flag it.

## Feedback Loop Analysis

When diagnosing a CNS alert or system issue, analyze the relevant feedback loop on five properties:

| Property | Question | hKask Diagnostic |
|----------|----------|------------------|
| **Polarity** | Negative (stabilizing) or positive (amplifying)? | CNS is negative feedback by design. Positive feedback = runaway — critical. |
| **Delay** | How long between action and feedback? | Inference latency, tool dispatch time, variety counter refresh interval |
| **Gain** | How strongly does feedback affect the system? | Algedonic threshold sensitivity. Too high = missed alerts. Too low = alert fatigue. |
| **Closure** | Is the loop actually closed? | Algedonic alert emitted but never consumed by Curator = broken closure |
| **Fidelity** | Does the signal accurately represent reality? | Variety counter only counts what it measures. Unmeasured failure modes = blind spots |

### Spotting Broken Feedback Loops

| Symptom | Cybernetic Diagnosis | What to Check |
|---------|---------------------|---------------|
| Variety deficit exceeds 100 with no Curator response | Broken feedback closure — signal emitted, never consumed | Communication loop connectivity, Curator inbox |
| Variety deficit never exceeds 50 despite known problems | Sensor stall — observation loop broken | MCP dispatch health, ν-event persistence |
| Algedonic alerts fire repeatedly with no change | Positive feedback or gain too high | Dampener `override_cooldown` (120s), backpressure threshold |
| CNS spans missing for expected operations | Model-reality divergence | `cns.<domain>.<operation>` span registration, tracing pipeline |
| Communication queue depth exceeds backpressure threshold | S2 coordination failure | `Arc<AtomicU64>` counter, `communication_backpressure_threshold` |

## Variety Engineering

Ashby's Law of Requisite Variety: the regulator's variety must match the system's disturbance variety. If the system can fail in 100 ways but the CNS only monitors 10, that is a variety deficit.

### hKask's Variety Architecture

- **Raw variety:** Agent sessions produce many tool calls, inference requests, and state changes per interval
- **Attenuation layer:** CNS variety counters aggregate raw activity into diversity metrics (distinct tool types, prompt patterns, inference models used)
- **Amplification layer:** When variety deficit exceeds threshold, the Curator amplifies by ranking, presenting reasoned options, and escalating

### Variety Analysis Checklist

1. Enumerate system variety: What failure modes, behavioral patterns, and edge cases exist?
2. Enumerate regulator variety: What CNS spans, variety counters, and algedonic thresholds cover them?
3. Is `regulator_variety >= system_variety`? If not, attenuate (add more spans/counters) or amplify (add more Curator escalation paths).
4. Check `cns.variety` spans for gaps — unmeasured dimensions of agent behavior.

### The Context Window as Channel Capacity

The agent's context window is a finite Shannon channel. Every token spent on one thing is a token not spent on another.

- **System prompt:** Fixed cost (persona + skill instructions)
- **Conversation history:** Growing cost (each turn adds tokens)
- **CNS observations:** Variable cost (variety reports, algedonic alerts)
- **Available for reasoning:** Whatever remains

When approaching the limit, attenuate — summarize history, drop stale context, focus on high-signal observations. Never silently lose critical context because low-priority information filled the window.

## The Good Regulator (Conant-Ashby)

The Good Regulator theorem states: every good regulator of a system must be a model of that system. Applied to hKask:

1. The CNS variety counter is the regulator's model of agent behavior diversity.
2. Where does the model diverge from reality? Check: are there failure modes the variety counter doesn't capture?
3. Is the model updated when the system changes? Stale variety baselines are worse than no baselines.
4. Does the model include failure modes, or only success modes? A model that only tracks happy paths is not a Good Regulator.

## Spec Drift as Cybernetic Signal

`LoopPayload::SpecDriftAlert` from `DefaultSpecCurator` is a cybernetic signal — it means the system's regulatory model (specifications) has diverged from the implemented system. The alert flows through the Communication Loop to the Curation inbox alongside ν-event persistence.

When spec drift exceeds threshold:
1. The spec no longer accurately models the system (Conant-Ashby violation)
2. The Curator cannot regulate based on accurate information
3. The correct response is to revise the spec, not to suppress the alert

## Composes With

- **pragmatic-laziness** — Phase 2 (Identify Loops) of the 3-phase lazy loop. Activates pragmatic-cybernetics to map feedback loops and locate effort hotspots after semantic classification.

## Registry Templates

This skill's runtime templates live in `registry/templates/pragmatic-cybernetics/`:

| Template | Type | Purpose |
|----------|------|--------|
| `cybernetics-analyze-loop.j2` | KnowAct | Analyze a feedback loop on 5 properties (polarity, delay, gain, closure, fidelity) |
| `cybernetics-variety-check.j2` | KnowAct | Evaluate variety balance using Ashby's Law of Requisite Variety |
| `cybernetics-vsm-map.j2` | KnowAct | Map hKask components to VSM S1–S5 and assess viability |

The SKILL.md (this file) teaches the Zed coding agent the cybernetic reasoning framework. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When to Use This Skill

- **CNS alert fires:** Which cybernetic function failed? (Usually sensor, model, or feedback closure.)
- **"Is hKask healthy?":** Check each VSM system. Are all five present and functioning?
- **Algedonic alerts fire repeatedly with no change:** Broken feedback loop or positive feedback. Check dampener cooldown and backpressure.
- **Variety deficit is chronic:** The system is in a rut. The Curator should propose novel approaches or the user should introduce new task patterns.
- **New feature proposed:** Variety analysis. Does this add regulatory burden? Is there requisite variety to handle the new disturbance path?
- **Agent pod seems stuck:** Check its OCAP boundary. Is the pod viable within its capability scope, or does it need additional capabilities to close its feedback loop?

## Quick Reference Cards

### Feedback Loop Analysis
1. **Polarity:** Negative (stabilizing) or positive (amplifying)?
2. **Delay:** How long between action and feedback?
3. **Gain:** How strongly does feedback affect the system?
4. **Closure:** Is the loop actually closed?
5. **Fidelity:** Does the signal accurately represent what it claims?

### Variety Analysis
1. Enumerate system variety (failure modes, behavioral patterns)
2. Enumerate regulator variety (CNS spans, variety counters, Curator escalation paths)
3. `regulator_variety >= system_variety`? If not, attenuate or amplify.

### Good Regulator Check
1. What is the regulator's model of the system?
2. Where does the model diverge from reality?
3. Is the model updated when the system changes?
4. Does the model include failure modes, or only success modes?

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/pragmatic-cybernetics.yaml`

### PDCA Convergence
- **Threshold:** 0.25 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = loop diagnostics are stable for intervention

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 12000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
