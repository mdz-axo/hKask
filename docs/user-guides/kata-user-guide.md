---
title: "Kata User Guide — Toyota Kata for Agent Capability Development"
audience: [developers, operators, curators]
last_updated: 2026-06-18
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, lifecycle]
---

# Kata User Guide — Toyota Kata for Agent Capability Development

## 1. Research Background

### 1.1 Origins: Mike Rother and Toyota Kata (2004–2009)

Between 2004 and 2009, researcher Mike Rother studied Toyota's management system. His finding: Toyota's approach revolves around **daily practice of a practical form of scientific thinking, with managers as coaches**. He named the pattern the **Improvement Kata** — "kata" from martial arts practice routines. Published in 2009 as *Toyota Kata*. [^rother-2009]

### 1.2 Core Concepts

**Scientific thinking is not our default mode.** We jump to conclusions. We don't notice our knowledge threshold — where facts end and assumptions begin. Scientific thinking means testing ideas, comparing theory to evidence, and adjusting based on what you discover.

**Kata are practice routines.** Structured routines practiced deliberately until the pattern becomes habitual.

**Starter Kata are training wheels.** Deliberately simplified. Learners graduate beyond them once the pattern becomes automatic.

**Coaching is essential.** "Without coaching, a change in our brain's wiring is less likely to occur." [^rother-2017] The coach provides procedural guidance, never solutions.

### 1.3 The Two Linked Behaviors

| Behavior | Who Practices | What They Do |
|----------|--------------|--------------|
| **Improvement Kata (IK)** | The learner | 4-step PDCA scientific pattern |
| **Coaching Kata (CK)** | The coach | 5-question dialogue grounded in IK data |

**Starter Kata are NOT a third behavior.** They are practice routines for learning IK and CK steps.

### 1.4 The Improvement Kata — 4-Step PDCA Pattern

```
Step 1: UNDERSTAND THE DIRECTION — Challenge from above. Knowledge threshold.
Step 2: GRASP THE CURRENT CONDITION — Facts and data. Go and see. metric_before.
Step 3: ESTABLISH THE NEXT TARGET CONDITION — Specific, measurable, 1wk–3mo.
        Obstacles visible here → Obstacles Parking Lot.
Step 4: ITERATE (PDCA) — Plan→Do→Check→Act. One obstacle at a time.
        Each experiment moves the knowledge threshold.
        After cycle: metric_after → improvement signal → Act on findings.
```

**The cybernetic loop:** Before/after measurement closes the PDCA cycle. The engine captures metrics before the cycle (from CNS counters or declared baselines), runs all 4 steps, captures metrics after, and computes the improvement signal — IS evidence of change.

### 1.5 The Coaching Kata — 5-Question Dialogue

| # | Question | Purpose |
|---|----------|---------|
| 1 | What is the **Target Condition**? | Ground learner in the goal |
| 2 | What is the **Actual Condition** now? | Ground learner in reality (IS, not assumptions) |
| 3 | What **Obstacles**? Which **ONE** now? | Focus — one obstacle at a time |
| 4 | What is your **Next Step**? What do you **expect**? | PDCA Plan with prediction |
| 5 | How quickly can we go and see what we **Learned**? | Close the feedback loop |

**The coach never gives solutions.** "Have you checked the metrics?" not "You should adjust the timeout." Coaching questions are grounded in the learner's actual IK storyboard data via `ik_state_ref`.

### 1.6 Key Principles

- **20 minutes a day > 2 hours once a week.** [^rother-2017]
- **You can only work on one bad habit at a time.** [^rother-2017]
- **The path can't be determined in advance.** Navigate by compass, not map. [^rother-2009]

---

## 2. Technical Build — hKask Kata Architecture

### 2.1 Four-Skill Architecture


```
┌─────────────────────────────────────────────────────────────┐
         → Full system with CNS monitoring, habit tracking, interventions.
         → Multiple agents at different stages.
         → Goal: Self-sustaining improvement culture.
```

### 3.2 Using kata-starter (Day 1)

**When:** New agent, low automaticity score, or returning after a gap.

**How:**
```bash
kask kata start starter-kata --bot Alice
```

This executes all 3 practice routines (daily_cns_report, span_emission_practice, energy_awareness) with no LLM calls — pure habit formation. Gas cost is zero.

**What to expect:** The agent will practice the routine, record the session to memory, and emit a CNS span. No real problems are solved — this is pure practice.

**Graduation criteria:** When the routine feels automatic. CNS automaticity score > 0.5.

### 3.3 Using kata-improvement (Week 2+)

**When:** Specific, measurable capability gap exists. Automaticity > 0.5.

**Prerequisites:** Completed kata-starter.

**How:**
```bash
kask kata start improvement-kata --bot Alice --ctx "capability=span_emission"
```

The engine walks the 4-step PDCA cycle:
1. **Direction** — classification step, uses classifier model (Qwen3 MoE, configured via HKASK_CLASSIFIER_MODEL)
2. **Current Condition** — classification step, uses classifier model
3. **Target Condition** — reasoning step, uses the configured generation model
4. **Experiment** — reasoning step, uses the configured generation model

Steps marked `classifier: true` in the manifest use the system classifier model (Qwen3-235B-A22B MoE on KiloCode). Reasoning steps use the default generation model.

**Save and resume:**
```bash
# Run and save state
kask kata start improvement-kata --bot Alice --save /tmp/kata-state.json

# Resume from saved state (skips completed steps)
kask kata start improvement-kata --bot Alice --resume /tmp/kata-state.json
```

**What to expect:** The agent will define a target condition, identify obstacles, and begin running PDCA experiments. Each experiment produces learning. The knowledge threshold moves forward.

**Consent required:** Curator must consent to start an Improvement Kata cycle.

### 3.4 Using kata-coaching (Week 3+)

**When:** Active Improvement Kata cycle exists. Coach is available.

**Prerequisites:** Learner has an active IK cycle with a target condition. Coach has practiced the Five Questions Drill.

**How:**
```bash
kask kata start coaching-kata --bot Alice --ctx "learner=Bob"
```

The coach asks the 5 questions in sequence. Each question builds on accumulated context from previous responses. The learner responds with their IK storyboard data.

**What to expect:** Daily 20-minute sessions. The coach reinforces the scientific pattern. The learner's thinking becomes visible. Over time, the learner internalizes the pattern.

**Consent required:** Learner must explicitly consent to being coached. Consent is revocable at any time.

### 3.5 CNS Observability

All kata execution emits tracing spans under the `hkask.kata` target with the manifest's namespace as a field:

| Span | When | Fields |
|------|------|--------|
| `kata.cycle.start` | Cycle begins | kata_type, bot, namespace |
| `kata.step.start` | Each step begins | step, action, bot, namespace |
| `kata.step.complete` | Each step completes | step, gas, namespace |
| `kata.coaching.question` | Each coaching question | question, bot, namespace |
| `kata.cycle.complete` | Cycle ends | steps/questions/practices, gas, namespace |

### 3.6 Using the kata Bundle (Month 2+)

**When:** Multiple agents at different stages. Full system monitoring desired.

**How:**
```bash
```

The bundle's `kata-selector.j2` routes agents based on context:
- New agent → kata-starter
- Low automaticity → kata-starter (Observation Drill)
- Capability gap + auto > 0.5 → kata-improvement
- Active IK + coach available → kata-coaching
- Habit decay (3+ days) → Habit intervention

**What to expect:** CNS monitors all practices. Algedonic alerts fire if practices drop below baseline. Habit interventions trigger automatically.

### 3.7 Composition Rules

| Transition | Allowed? | Condition | Consent |
|------------|----------|-----------|---------|
| Starter → Improvement | Yes | Automaticity > 0.5 + capability gap exists | Curator |
| Improvement → Coaching | Yes | Coach available during Step 4 | Learner |
| Coaching → Improvement | Yes | Specific capability gap identified | Curator |
| Starter → Coaching | No | Starter is self-contained | — |
| Any → Starter | Yes | Low automaticity or 7+ day gap | Self |

### 3.8 Anti-Patterns

1. **Deploying the full bundle on day one.** Adopt incrementally. Start with kata-starter.
2. **Skipping Starter Kata.** Agents without foundational habits will struggle with improvement.
3. **Coaching without an active IK cycle.** The 5 questions need a target condition to reference.
4. **Giving solutions instead of asking questions.** "You should..." defeats the purpose of coaching.
5. **Vague target conditions.** "Get better" is not a target condition. "Reduce latency from 2.3s to 1.5s by Friday" is.
6. **Working on multiple obstacles simultaneously.** One at a time.
7. **Staying in Starter Kata forever.** These are "starter" kata, not "finishing" kata. Graduate.
8. **Ignoring CNS alerts.** Variety deficits and habit decay require intervention, not silence.

### 3.9 Quick Reference Card

```
┌─────────────────────────────────────────────────────────┐
│                 TOYOTA KATA QUICK REFERENCE               │
├─────────────────────────────────────────────────────────┤
│ IMPROVEMENT KATA (4 steps)                               │
│   1. Direction — What challenge?                         │
│   2. Current Condition — What are the facts?             │
│   3. Target Condition — Where to by when?                │
│   4. Iterate — PDCA experiments, one obstacle at a time  │
├─────────────────────────────────────────────────────────┤
│ COACHING KATA (5 questions)                              │
│   1. What is the Target Condition?                       │
│   2. What is the Actual Condition now?                   │
│   3. What Obstacles? Which ONE now?                      │
│   4. What is your Next Step? What do you expect?         │
│   5. How quickly can we go and see What we Learned?      │
├─────────────────────────────────────────────────────────┤
│ PRACTICE                                                 │
│   • 20 minutes daily, not 2 hours weekly                 │
│   • Coach asks questions, never gives solutions          │
│   • One obstacle at a time, one bad habit at a time      │
│   • Each experiment moves the knowledge threshold        │
└─────────────────────────────────────────────────────────┘
```

---

## References

[^rother-2009]: Rother, M. (2009). *Toyota Kata: Managing People for Improvement, Adaptiveness, and Superior Results.* McGraw-Hill.
[^rother-2017]: Rother, M. (2017). *The Toyota Kata Practice Guide: Developing Scientific Thinking Skills for Superior Results — in 20 Minutes a Day.* McGraw-Hill.
[^rother-culture]: Rother, M. & Aulinger, G. (2017). *Toyota Kata Culture: Building Organizational Capability and Mindset through Kata Coaching.* McGraw-Hill.
[^lean-kata]: Lean Enterprise Institute. *Kata — A Resource Guide.* https://www.lean.org/lexicon-terms/kata/
[^rother-website]: Rother, M. *The Toyota Kata Website.* https://public.websites.umich.edu/~jmondisa/TK/Homepage.html
[^liker-2021]: Liker, J. (2021). *The Toyota Way, 2nd Edition.* McGraw-Hill.
[^rother-medium]: Rother, M. (2023). "Learning to Think Scientifically." Medium. https://medium.com/@734mike/thinking-scientifically-407fa7e0db27

---

*ℏKask - A Minimal Viable Container for Agents — Kata User Guide — v0.28.0*
