---
title: "Kata User Guide — Toyota Kata for Agent Capability Development"
audience: [developers, operators, curators]
last_updated: 2026-06-14
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, lifecycle]
---

# Kata User Guide — Toyota Kata for Agent Capability Development

## 1. Research Background

### 1.1 Origins: Mike Rother and Toyota Kata (2004–2009)

Between 2004 and 2009, researcher Mike Rother studied Toyota's management system to understand the factors behind their sustained success with continuous improvement and adaptation. His finding was that Toyota's approach revolves around **daily practice of a practical form of scientific thinking across the organization, with managers taking the role of coaches**.

Rother observed a repeating pattern of thinking and behavior in Toyota managers that was different from prevailing Western command-and-control routines. He depicted this pattern as a four-step model, which he named the **Improvement Kata** — "kata" being a Japanese term for structured practice routines (from martial arts, where kata are used to train combatants in basic building-block moves).

The research was published in 2009 as the book *Toyota Kata: Managing People for Improvement, Adaptiveness, and Superior Results*. [^rother-2009]

### 1.2 Core Concepts

**Scientific thinking is not our default mode.** Our brains create feelings of certainty based on limited information. We jump to conclusions. We don't notice our knowledge threshold — where what we know ends and assumption begins. Scientific thinking means knowing that any idea should be tested, learning to compare what you think (theory) with what actually happens (evidence), and adjusting based on what you discover from the difference.

**Kata are practice routines.** The word comes from martial arts, where kata are used to train combatants in basic building-block moves. In Toyota Kata, kata are simple, structured routines that you practice deliberately, especially at the beginning, so their pattern becomes a habit and leaves you with new abilities.

**Starter Kata are training wheels.** Rother explicitly calls them "starter" (not "finishing") kata because learners are expected to graduate beyond them. They are deliberately simplified practice routines for each step of the Improvement Kata and for coaching. Once the pattern becomes automatic — a meta-habit — the learner develops their own way.

**Coaching is essential.** "Without coaching, a change in our brain's wiring is less likely to occur." [^rother-2017] The coach makes the learner's thinking visible, provides procedural guidance (not solutions), and reinforces the scientific pattern daily. Coaching itself is a skill that takes practice.

### 1.3 The Two Linked Behaviors

Toyota Kata has **two linked behaviors**, not three:

| Behavior | Who Practices | What They Do |
|----------|--------------|--------------|
| **Improvement Kata (IK)** | The learner | 4-step scientific pattern for achieving challenging goals |
| **Coaching Kata (CK)** | The coach (manager/supervisor) | 5-question dialogue to teach the IK pattern |

**Starter Kata are NOT a third behavior.** They are the practice routines for learning each step of the IK and CK. They are the "how" of practicing, not a separate "what." [^rother-medium]

### 1.4 The Improvement Kata — 4-Step Scientific Pattern

```
Step 1: UNDERSTAND THE DIRECTION
        Challenge from the level above. What are we trying to achieve?

Step 2: GRASP THE CURRENT CONDITION
        Facts and data. Go and see. Don't assume.
        This is your Current Knowledge Threshold.

Step 3: ESTABLISH THE NEXT TARGET CONDITION
        Specific, measurable, 1 week to 3 months out.
        Beyond current knowledge threshold.
        Obstacles become visible here → Obstacles Parking Lot.

Step 4: ITERATE TOWARD THE TARGET CONDITION
        Rapid PDCA experiments. One obstacle at a time.
        Each experiment moves the knowledge threshold.
        Coaching cycles happen here (daily, ~20 minutes).
```

**Critical insight:** Step 4 is where the real work happens. You will encounter obstacles you couldn't see from the planning phase. The path won't be straight — you're in a mode of rapid learning and discovery, adjusting course based on facts and data. The threshold of knowledge moves with each experiment.

### 1.5 The Coaching Kata — 5-Question Dialogue

The coach asks these five questions in sequence, daily, at the gemba (where the work happens):

| # | Question | Purpose |
|---|----------|---------|
| 1 | What is the **Target Condition**? | Ground the learner in the goal |
| 2 | What is the **Actual Condition** now? | Ground the learner in reality |
| 3 | What **Obstacles** do you think are preventing you from reaching the target condition? Which **ONE** are you addressing now? | Focus the learner |
| 4 | What is your **Next Step**? (Next experiment) What do you **expect**? | Drive action — this is the PDCA Plan step |
| 5 | How quickly can we go and see what we have **Learned** from taking that step? | Close the feedback loop |

The coach provides **procedural guidance, not solutions.** "Have you checked the metrics?" not "You should adjust the timeout to 5 seconds." The coach's role is to make the learner's thinking visible and reinforce the scientific pattern.

### 1.6 The Meta-Cognitive Loop

The daily coaching cycle is the meta-cognitive engine of Toyota Kata. It serves three purposes:

1. **Reinforces the pattern** of the Improvement Kata through daily repetition
2. **Makes the learner's thinking apparent** so the coach can give appropriate feedback
3. **Helps the learner see what they cannot see alone** — their own knowledge threshold

Over time, the learner internalizes the pattern. The Starter Kata routines are gradually replaced by meta-habits. The learner begins to approach every problem with the "skeleton" of the Kata routine, understanding that they are not experimenting TO the solution but experimenting to FIND obstacles. They become obstacle-driven rather than solution-driven.

### 1.7 Key Principles from the Research

- **20 minutes a day is better than two hours once a week.** If you practice only periodically and the rest of the time it's business as usual, what you're actually practicing is business as usual. [^rother-2017]
- **Knowing isn't the same as doing.** Benchmarking is not enough to make change happen. [^rother-medium]
- **You can really only work on one bad habit at a time.** [^rother-2017]
- **The path to a challenging goal can't be determined in advance.** You navigate with a compass, not a map. [^rother-2009]
- **Managers become the coaches by default.** Coaching is not a separate role — it's how managers develop their people. [^liker-2021]

---

## 2. Technical Build — hKask Kata Architecture

### 2.1 Four-Skill Architecture

The hKask kata system is implemented as **three independently usable skills plus one bundle** that composes them:

```
┌─────────────────────────────────────────────────────────────┐
│                     KATA BUNDLE                              │
│  Routes agents to the right kata based on context           │
│  Manages transitions: starter→improvement↔coaching          │
│  Monitors habits via CNS, triggers interventions            │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ KATA-STARTER │  │KATA-IMPROVE- │  │KATA-COACHING │     │
│  │              │  │    MENT      │  │              │     │
│  │ Build found- │  │ 4-step       │  │ 5-question   │     │
│  │ ational      │  │ scientific   │  │ dialogue     │     │
│  │ habits       │  │ pattern      │  │ for teaching │     │
│  │              │  │              │  │              │     │
│  │ Consent:     │  │ Consent:     │  │ Consent:     │     │
│  │ Self         │  │ Curator      │  │ Learner      │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

Each skill is **independently usable and adoptable.** An agent doesn't need the full system — they can start with `kata-starter` today.

### 2.2 Skill Details

| Skill | Zed Layer | Templates | Manifest | Purpose |
|-------|-----------|-----------|----------|---------|
| **kata-starter** | `.agents/skills/kata-starter/SKILL.md` | 5 in `registry/templates/kata-starter/` | `starter-kata.yaml` | Build foundational scientific thinking habits through deliberate practice |
| **kata-improvement** | `.agents/skills/kata-improvement/SKILL.md` | 5 in `registry/templates/kata-improvement/` | `improvement-kata.yaml` | 4-step scientific pattern for achieving challenging goals |
| **kata-coaching** | `.agents/skills/kata-coaching/SKILL.md` | 6 in `registry/templates/kata-coaching/` | `coaching-kata.yaml` | 5-question dialogue for teaching scientific thinking |
| **kata** (bundle) | `.agents/skills/kata/SKILL.md` | 7 in `registry/templates/kata/` | `kata-pattern.yaml` | Full system orchestration with routing, habit monitoring, and iteration |

### 2.3 Template Distribution (23 total)

| Skill | Templates | Types |
|-------|-----------|-------|
| **kata** (bundle) | `kata-selector.j2`, `consent-and-select.j2`, `outcome-and-habit.j2`, `habit-intervention.j2`, `iteration-check.j2`, `iteration-comparison.j2`, `kata-switch-check.j2` | 6 KnowAct, 1 WordAct |
| **kata-starter** | `starter-cycle.j2`, `starter-selector.j2`, `starter-five-questions.j2`, `starter-pdca-cycle.j2`, `starter-observation-drill.j2` | 4 FlowDef, 1 KnowAct |
| **kata-improvement** | `improvement-cycle.j2`, `improvement-step1-direction.j2`, `improvement-step2-current.j2`, `improvement-step3-target.j2`, `improvement-step4-experiment.j2` | 1 FlowDef, 4 WordAct |
| **kata-coaching** | `coaching-cycle.j2`, `coaching-q1-target.j2`, `coaching-q2-actual.j2`, `coaching-q3-obstacles.j2`, `coaching-q4-experiment.j2`, `coaching-q5-learn.j2` | 1 FlowDef, 5 WordAct |

### 2.4 Bundle Manifests (5 total)

| Manifest | Skill | Purpose |
|----------|-------|---------|
| `kata-pattern.yaml` | kata (bundle) | Unified orchestration — routes to starter/improvement/coaching |
| `kata-iteration.yaml` | kata (bundle) | Variance assessment sub-manifest (max 2 iterations) |
| `starter-kata.yaml` | kata-starter | 3-step practice flow (select → execute → record) |
| `improvement-kata.yaml` | kata-improvement | 4-step scientific pattern with gas, CNS, OCAP |
| `coaching-kata.yaml` | kata-coaching | 5-question dialogue flow with gas, CNS, OCAP |

### 2.5 Interfaces

The kata engine (`crates/hkask-services/src/kata.rs`) exposes:

| Interface | Purpose |
|-----------|--------|
| `KataEngine::execute()` | Execute a full kata cycle |
| `KataEngine::load_manifest()` | Load and parse a kata manifest |
| `KataEngine::with_consent()` | Set OCAP consent gate |
| `KataEngine::with_cns()` | Set CNS observer callback |
| `KataState::save()` / `KataState::load()` | Persist and resume kata state |

### 2.6 CNS Integration

All kata execution emits CNS spans under `cns.prompt.kata` with sub-spans for each skill:

| Counter | Baseline | Warning | Critical |
|---------|----------|---------|----------|
| `kata.practices.completed` | 5/week | < 3/week | < 2/week |
| `kata.habit.formation` | 1 per 21 days | — | < 1 per 30 days |
| `kata.automaticity.score` | +0.05/week | +0.03/week | +0.01/week |
| `kata.iterations.used` | 0.5/session | > 1.5/session | > 2.0/session |
| `kata.variance.score` | < 0.2 | > 0.4 | > 0.6 |

### 2.7 Bootstrap Registration

All 23 kata templates are registered in `registry/templates/bootstrap-registry.yaml` under four sections: Kata Bundle (7 entries), Kata-Starter (5), Kata-Improvement (5), Kata-Coaching (6). The R7.5 bot owns the "kata" domain per `hkask-types/src/r7.rs`. Bootstrap phase 7 (`KataReadiness`) verifies domain ownership.

---

## 3. User How-To

### 3.1 Adoption Path

Toyota Kata is adopted incrementally. You don't deploy the whole system at once:

```
Day 1:   kata-starter (Five Questions Drill)
         → Print the 5-question card. Read through it in order.
         → Goal: Internalize the questioning pattern.

Week 1:  kata-starter (PDCA Cycle, Observation Drill)
         → Practice PDCA on trivial processes.
         → Practice distinguishing facts from interpretations.
         → Goal: Build experimental mindset.

Week 2:  kata-improvement (first IK cycle)
         → Pick a real, measurable capability gap.
         → Run the 4-step pattern.
         → Goal: Apply scientific thinking to a real problem.

Week 3+: kata-improvement + kata-coaching
         → Schedule daily coaching sessions (20 min).
         → Coach asks 5 questions, learner shows IK storyboard.
         → Goal: Make scientific thinking a daily habit.

Month 2+: kata bundle
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
1. **Direction** — classification step, uses Gemma 4 26B classifier model
2. **Current Condition** — classification step, uses Gemma 4 26B classifier model
3. **Target Condition** — reasoning step, uses the configured generation model
4. **Experiment** — reasoning step, uses the configured generation model

Steps marked `classifier: true` in the manifest use the system classifier model (`google/gemma-4-26B-A4B-it` via DeepInfra). Reasoning steps use the default generation model.

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
kask kata start kata-pattern --bot Curator --ctx "learner=Alice"
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

*ℏKask - A Minimal Viable Container for Agents — Kata User Guide — v0.27.0*
