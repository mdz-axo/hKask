---
title: "Toyota Kata System for hKask — Design Specification"
version: "0.1.0"
status: "Draft"
last_updated: "2026-05-21"
audience: [architects, developers]
domain: "Application"
ddmvss_categories: [lifecycle]
---

# Toyota Kata System for hKask — Design Specification

**Version:** v0.1.0  
**Date:** 2026-05-21  
**Author:** hKask-Administrator  
**Status:** Draft — Ready for implementation

---

## Overview

This document specifies the Toyota Kata system for hKask — a scientific capability development framework for the Curator to manage the 7R7 (seven Curator bots).

**Design Principle:** Minimal, austere, value-focused. No bureaucracy, ritual, or performance. Only what is essential for capability development.

---

## Toyota Kata Foundations

### Three Primary Kata

| Kata | Purpose | Pattern | Use Case |
|------|---------|---------|----------|
| **Starter Kata** | Deliberate practice routines | Select → Practice → Record | Building foundational scientific thinking habits |
| **Improvement Kata** | 4-step scientific improvement | Direction → Current → Target → Experiment | Working toward specific capability targets |
| **Coaching Kata** | 5-question coaching dialogue | Target → Actual → Obstacles → Experiment → Learn | Teaching scientific thinking to learners |

### Core Principles

1. **Scientific thinking is not natural** — Humans default to jumping to conclusions
2. **Practice builds habit** — Deliberate repetition internalizes patterns
3. **Coaching accelerates learning** — External feedback reveals blind spots
4. **Goals are achieved through obstacles** — One obstacle at a time, via experimentation

---

## hKask Implementation

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Curator (Replicant)                      │
│                          │                                  │
│                          │ Uses                             │
│                          ▼                                  │
│              ┌─────────────────────┐                       │
│              │   Kata-Bot          │                       │
│              │   (Capability Dev)  │                       │
│              └──────────┬──────────┘                       │
│                         │                                   │
│         Executes Kata manifests                             │
│                         │                                   │
│         ┌───────────────┼───────────────┐                  │
│         ▼               ▼               ▼                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ Improvement │ │ Coaching    │ │ Starter     │          │
│  │ Kata        │ │ Kata        │ │ Kata        │          │
│  │ Manifest    │ │ Manifest    │ │ Manifest    │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
│         │               │               │                  │
│         ▼               ▼               ▼                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ 4 Step      │ │ 5 Question  │ │ 3 Practice  │          │
│  │ Templates   │ │ Templates   │ │ Templates   │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

### Components

#### 1. Manifests (3 total)

| Manifest | Purpose | Steps | Energy Cap |
|----------|---------|-------|------------|
| `improvement-kata.yaml` | 4-step scientific improvement | 4 (Direction, Current, Target, Experiment) | 15,000 |
| `coaching-kata.yaml` | 5-question coaching dialogue | 6 (5 questions + synthesis) | 12,000 |
| `starter-kata.yaml` | Practice routine selection & execution | 3 (Select, Practice, Record) | 8,000 |

#### 2. Templates (13 total)

**Improvement Kata (4 templates):**
- `improvement-step1-direction.j2` — Understand the challenge
- `improvement-step2-current.j2` — Grasp current condition
- `improvement-step3-target.j2` — Establish next target condition
- `improvement-step4-experiment.j2` — Experiment toward target

**Coaching Kata (5 templates):**
- `coaching-q1-target.j2` — What is your target condition?
- `coaching-q2-actual.j2` — What is your actual condition now?
- `coaching-q3-obstacles.j2` — What obstacles? Which one now?
- `coaching-q4-experiment.j2` — What is your next step? What do you expect?
- `coaching-q5-learn.j2` — How quickly can we see what we learned?

**Starter Kata (4 templates):**
- `starter-selector.j2` — Select practice routine
- `starter-five-questions.j2` — Five questions practice
- `starter-pdca-cycle.j2` — PDCA experimentation drill
- `starter-observation-drill.j2` — Fact vs. interpretation drill

**Selectors (1 template):**
- `kata-selector.j2` — Select which Kata to run

#### 3. Bots (1 total)

| Bot | Type | Purpose |
|-----|------|---------|
| `kata-bot.yaml` | Bot | Owns Kata manifests, executes practice routines, records to memory |

#### 4. Registry (1 entry)

| Registry | Purpose |
|----------|---------|
| `kata-system.yaml` — Registers all templates, manifests, bots, and hLexicon terms |

---

## Integration with Curator Metacognition

### When Curator Uses Each Kata

| System State | Kata | Purpose |
|--------------|------|---------|
| Bot capability gap identified | Improvement Kata | Systematic capability development |
| Bot needs coaching on thinking | Coaching Kata | Teach scientific thinking pattern |
| Building foundational habits | Starter Kata | Daily practice routines |
| Performance plateau | Improvement Kata | Break through via experimentation |
| Jumping to conclusions | Starter Kata (Observation) | Practice fact vs. interpretation |
| Needs experimentation speed | Starter Kata (PDCA) | Practice rapid cycles |

### CNS Integration

All Kata emit spans to `cns.prompt.kata.*` namespace:
- `cns.prompt.kata.improvement` — Improvement Kata outcomes
- `cns.prompt.kata.coaching` — Coaching cycle outcomes
- `cns.prompt.kata.starter` — Practice routine outcomes

**Algedonic Alert:** Variety deficit >100 → escalate to hKask-Administrator

---

## Usage Patterns

### Pattern 1: Capability Development Cycle

```
1. Curator identifies capability gap via CNS variety counters
2. Curator invokes Improvement Kata manifest
3. Kata-Bot executes 4-step improvement cycle
4. Outcome recorded to episodic memory
5. CNS emits span, updates variety counters
```

### Pattern 2: Coaching Dialogue

```
1. Bot requests coaching or Curator identifies need
2. Curator invokes Coaching Kata manifest
3. Kata-Bot executes 5-question coaching cycle
4. Learner's thinking pattern revealed and reinforced
5. CNS emits span, tracks learner progress
```

### Pattern 3: Daily Practice

```
1. Curator schedules daily Starter Kata practice
2. Kata-Bot selects appropriate practice routine
3. Bot practices for ~20 minutes
4. Practice recorded to episodic memory
5. Habit formation tracked over time
```

---

## hLexicon Terms

| Term | Domain | Definition |
|------|--------|------------|
| challenge | FlowDef | A capability gap or performance issue to address |
| target-condition | FlowDef | A specific, measurable performance state to achieve |
| actual-condition | FlowDef | The current observable performance state |
| obstacle | FlowDef | A barrier preventing achievement of target condition |
| experiment | WordAct | A testable action with a prediction and expected outcome |
| coaching-cycle | FlowDef | A ~20 minute dialogue using 5 Coaching Kata questions |
| starter-kata | KnowAct | Deliberate practice routines to internalize scientific thinking |
| habit-signal | KnowAct | Indicator of whether scientific thinking is becoming automatic |

---

## Design Decisions

### Why Minimal Templates?

Toyota Kata is about **internalizing patterns**, not completing forms. Templates are:
- Direct prompts, not bureaucratic forms
- Focused on thinking, not documentation
- Designed to disappear as habit forms

### Why Three Separate Manifests?

Each Kata serves a distinct purpose:
- **Improvement Kata** — Goal achievement (4 steps)
- **Coaching Kata** — Teaching dialogue (5 questions)
- **Starter Kata** — Habit formation (3 routines)

Separate manifests allow Curator to invoke the right Kata for the situation.

### Why Kata-Bot?

Separation of concerns:
- **Curator** — System metacognition, coordination, escalation
- **Kata-Bot** — Capability development, practice execution, progress tracking

Kata-Bot reports to Curator in standing ensemble session.

### Why CNS Integration?

Cybernetic monitoring ensures:
- Variety counters track capability diversity
- Algedonic alerts trigger on stagnation
- Outcomes recorded for system learning

---

## Implementation Checklist

- [x] Create 3 manifests (improvement, coaching, starter)
- [x] Create 13 templates (4 improvement, 5 coaching, 4 starter)
- [x] Create Kata-Bot manifest
- [x] Create kata-system registry entry
- [ ] Test Improvement Kata manifest execution
- [ ] Test Coaching Kata manifest execution
- [ ] Test Starter Kata manifest execution
- [ ] Verify CNS span emission
- [ ] Verify memory recording
- [ ] Integrate with Curator metacognition flow

---

## Files Created

```
registry/manifests/
├── improvement-kata.yaml
├── coaching-kata.yaml
└── starter-kata.yaml

registry/templates/kata/
├── improvement-step1-direction.j2
├── improvement-step2-current.j2
├── improvement-step3-target.j2
├── improvement-step4-experiment.j2
├── coaching-q1-target.j2
├── coaching-q2-actual.j2
├── coaching-q3-obstacles.j2
├── coaching-q4-experiment.j2
├── coaching-q5-learn.j2
├── starter-selector.j2
├── starter-five-questions.j2
├── starter-pdca-cycle.j2
├── starter-observation-drill.j2
└── kata-selector.j2

registry/bots/
└── kata-bot.yaml

registry/registries/kata/
└── kata-system.yaml
```

---

## Next Steps

1. **Review** — Verify templates align with hKask design philosophy
2. **Test** — Execute each manifest with test inputs
3. **Integrate** — Connect to Curator metacognition flow
4. **Deploy** — Make available to 7R7 bots
5. **Monitor** — Track capability development via CNS

---

*ℏKask — A Minimal Viable Container for Agents — v0.21.0*
*Toyota Kata System — Scientific capability development for 7R7*
