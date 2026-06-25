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
| `starter-kata.yaml` | kata-starter | 3-step practice flow (select → execute → record) |
| `improvement-kata.yaml` | kata-improvement | 4-step scientific pattern with gas, CNS, OCAP |
| `coaching-kata.yaml` | kata-coaching | 5-question dialogue flow with gas, CNS, OCAP |
| `kata-iteration.yaml` | kata-improvement | Standalone iteration manifest — 2-step variance assessment (loadable, engine-compatible) |

### 2.5 Engine Interfaces

The kata engine (`crates/hkask-services/src/kata.rs`) exposes:

| Interface | Purpose |
|-----------|--------|
| `KataEngine::execute()` | Execute a full kata cycle with before/after metrics |
| `KataEngine::load_manifest()` | Load and parse a kata manifest |
| `KataEngine::with_consent()` | Set OCAP consent gate (P2 Affirmative Consent) |
| `KataEngine::with_cns()` | Set CNS observer callback |
| `KataEngine::with_history()` | Inject practice history for streak/automaticity tracking |
| `KataEngine::with_history_store()` | Inject SQLite-backed kata history store for concurrent persistence |
| `KataEngine::with_metrics()` | Inject metric collector for before/after measurement |
| `KataEngine::with_cns_runtime()` | Inject CnsRuntime for variety counter integration |
| `KataEngine::record_history_entry()` | Persist a practice entry to the SQLite store when available |
| `KataState::save()` / `KataState::load()` | Persist and resume kata state |

### 2.6 CNS Integration

The kata engine integrates with the Cybernetic Nervous System at two levels:

**Variety Counters:** After each step, practice, or question, the engine increments CNS variety counters via `CnsRuntime::increment_variety()`:

| Counter | When Incremented | Baseline |
|---------|-----------------|----------|
| `kata.practices.completed` | Every step/question/practice | 5/week |
| `kata.automaticity.score` | After starter cycle (auto > 0) | +0.05/week |
| `kata.habit.formation` | After starter cycle (auto > 0.5) | 1 per 21 days |

**Algedonic Alerts:** After each cycle, the engine checks variety thresholds via `CnsRuntime::check_variety()` and emits `kata.algedonic` warnings when deficits exceed the configured threshold (default 100).

**Tracing Spans:** All execution emits structured spans under `hkask.kata`:

| Span | When | Key Fields |
|------|------|-----------|
| `kata.cycle.start` | Cycle begins | kata_type, bot, namespace, automaticity_before |
| `kata.step.start` | Each step begins | step, action, bot |
| `kata.step.checked` | PDCA Check phase | step, passed_check |
| `kata.step.complete` | Each step completes | step, gas |
| `kata.coaching.question` | Each coaching question | question, bot, has_ik_state |
| `kata.starter.practice` | Each starter practice | practice, bot |
| `kata.starter.habit_check` | Before starter cycle | automaticity, streak_days, needs_intervention |
| `kata.starter.habit_decay_alert` | When 3+ day gap detected | days_since_last |
| `kata.cycle.complete` | Cycle ends | steps, gas, has_signal, automaticity_delta |
| `kata.algedonic` | Variety deficit detected | severity, deficit, threshold |

### 2.7 Automaticity & Habit Tracking

Kata practice history is stored in `data/kata-history.json` with per-agent practice entries. The engine also supports SQLite-based persistence via `KataHistoryStore` when a database is available (default path: `data/hkask.db`, overridable via `HKASK_DB_PATH`). When both stores are available, entries are persisted to both — JSON for backward compatibility, SQLite for queryability and concurrent access. The daemon reads `kata_history` rows to power CNS queries and memory narratives.

- **Streak**: Consecutive days with at least one practice
- **Automaticity**: `min(1.0, streak_days / 21.0)` — linearly approaches 1.0 over 21 consecutive days
- **Decay**: When 3+ days elapse without practice, automaticity decays by `0.8^(days_since / 3)`
- **Graduation**: Automaticity > 0.5 qualifies for graduation from starter to improvement kata
- **Habit Intervention**: 3+ days without practice triggers a `kata.starter.habit_decay_alert` CNS warning

### 2.8 Improvement Signal (Cybernetic Feedback)

The Improvement Kata captures metrics before and after each cycle. The engine:

1. Before cycle: calls `MetricCollector` for each metric declared in the manifest (`metric_before`, `metric_after` spans)
2. After cycle: captures same metrics again
3. Computes `ImprovementSignal` with delta and direction (Positive/Negative/Stalled/NotMeasured)
4. Stores signal in `KataResult` and emits it in CNS spans

### 2.9 Memory Integration

Every step produces a `StepExperience` struct recorded to the agent's episodic memory via the daemon's dual-encoding pipeline. Step experiences include: agent name, kata type, step label, action, output summary, gas used, and timestamp. The CLI records each step individually (`kata_step`) plus an overall cycle completion (`kata_execute`).

### 2.10 Bootstrap Registration

All 23 kata templates are registered in `registry/templates/bootstrap-registry.yaml` under four sections: Kata Bundle (7 entries), Kata-Starter (5), Kata-Improvement (5), Kata-Coaching (6). Each template directory has a `manifest.yaml` describing template IDs, types (FlowDef/WordAct/KnowAct), and purposes.

### 2.11 Kanban Integration

Kata cycles execute as kanban tasks through the `hkask-mcp-kanban` MCP surface. This connects scientific capability development (kata) with headless task coordination (kanban), with CNS observing the full feedback path.

**PDCA → Kanban mapping:**

| PDCA Step | Kanban Action | Task State Transition | CNS Span |
|-----------|--------------|----------------------|----------|
| Plan | `/kanban improve <task-id>` | Backlog → Ready | `TaskCreated` |
| Do | Agent executes experiment | Ready → InProgress | `kata.step.start` |
| Check | Verify results against prediction | InProgress → Review | `kata.step.checked` |
| Act | Apply findings, update knowledge threshold | Review → Done | `kata.cycle.complete` |

**Coaching 5 Questions → Kanban Task Fields:**

| Question | Kanban Task Field |
|----------|-------------------|
| 1. What is the Target Condition? | `task.goal` |
| 2. What is the Actual Condition now? | `task.evidence_before` |
| 3. What Obstacles? Which ONE? | `task.blockers` |
| 4. Next Step? What do you expect? | `task.next_action` + `task.prediction` |
| 5. How quickly can we go see? | `task.review_interval` |

**Full feedback path:** KataEngine → CNS spans → KanbanService (task state transitions) → CNS variety counters → algedonic alerts → Curator escalation.

See also: `docs/user-guides/kanban-user-guide.md`, `docs/architecture/hKask-architecture-master.md` (Kata section)

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

*ℏKask - A Minimal Viable Container for Agents — Kata User Guide — v0.28.0*
