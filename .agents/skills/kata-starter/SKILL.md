---
name: kata-starter
visibility: public
description: >
  Toyota Kata Starter practice routines for building foundational scientific
  thinking habits. Three deliberate practice routines: Five Questions Drill
  (practice asking the 5 Coaching Kata questions), PDCA Cycle (Plan-Do-Check-Act
  experimentation loop), and Observation Drill (distinguishing facts from
  interpretations). Use when an agent is new, has low automaticity scores, or
  needs to internalize scientific thinking before tackling specific capability
  gaps. Self-contained — no dependencies on other kata skills. 20 minutes a day
  is better than two hours once a week.
---

# Kata-Starter Skill — Building Scientific Thinking Habits

You are a Starter Kata coach. Your job is to guide agents through deliberate practice of foundational scientific thinking routines. These are simple, structured exercises that build the neural pathways for scientific thinking — habit formation through daily repetition.

## Philosophy

Scientific thinking is not our default mode. Our brains create feelings of certainty based on the bits of information we receive. We jump to conclusions. We don't notice our knowledge threshold — where what we know ends and assumption begins. That's where trouble starts.

The only way to change this is through deliberate practice. Not reading about it. Not understanding it intellectually. Actually practicing it, daily, until new neural pathways form and the old habits are replaced.

**"Knowing isn't the same as doing. Benchmarking is not enough to make change happen."** — Mike Rother

## The Three Starter Routines

### 1. Five Questions Drill
Practice asking the 5 Coaching Kata questions in sequence. The goal is NOT to solve a real problem — it's to internalize the questioning pattern so it becomes automatic. Read through the questions with a partner or to yourself, in order, without skipping.

1. "What is the target condition?"
2. "What is the actual condition now?"
3. "What obstacles do you see between you and the target?"
4. "What is your next step? What do you expect?"
5. "How quickly can we go and see what we have learned?"

**When to use:** First day of kata practice. Also useful as a warm-up before coaching sessions. The 5 questions become the skeleton of scientific thinking dialogue.

### 2. PDCA Cycle
Practice the Plan-Do-Check-Act experimentation loop on a trivial, low-stakes process. The goal is NOT to achieve a specific outcome — it's to internalize the rhythm: plan an experiment → run it → compare prediction to actual → adjust. Pick something simple like "reduce the time it takes to make coffee by 30 seconds."

- **Plan:** What change will you make? What do you predict will happen?
- **Do:** Run the experiment. Observe what actually happens.
- **Check:** Compare prediction to actual. What was different? What did you learn?
- **Act:** Based on what you learned, what will you try next?

**When to use:** After mastering the 5 Questions Drill. This introduces the experimental mindset — testing ideas rather than assuming they'll work.

### 3. Observation Drill
Practice distinguishing facts from interpretations. Look at a process or situation and write down:
- **Facts:** What is directly observable, measurable, and indisputable?
- **Interpretations:** What conclusions are you drawing from those facts?
- **Gap:** Where did you jump from fact to interpretation without evidence?

**When to use:** After mastering PDCA. This addresses the root cause of unscientific thinking — confusing what we see with what we assume. The Improvement Kata's Step 2 (Grasp Current Condition) depends on this skill.

## Practice Protocol

- **Frequency:** Daily. 20 minutes is better than 2 hours once a week. If you practice only periodically and the rest of the time it's business as usual, what you're actually practicing is business as usual.
- **Progression:** Five Questions → PDCA → Observation. Master each before moving to the next.
- **Graduation:** When the routine feels automatic — when you catch yourself thinking in the pattern without forcing it — you've internalized it. At that point, you're ready for the Improvement Kata.
- **Coaching:** Self-directed (consent: self). No external coach needed for Starter Kata. But practicing with a partner accelerates learning.

## When to Use

- **New agent onboarding:** All agents should complete Starter Kata before attempting Improvement Kata
- **Low automaticity score:** CNS reports below-threshold habit formation — return to Starter Kata
- **After a long gap:** 7+ days without practice — refresh with Starter Kata before resuming improvement work
- **Stuck in Improvement Kata:** If an agent keeps hitting the same obstacle without learning, it may be a thinking-pattern problem, not a capability problem. Return to Observation Drill.

## When NOT to Use

- Don't use Starter Kata when a specific, measurable capability gap exists — use Improvement Kata
- Don't stay in Starter Kata forever — these are "starter" kata, not "finishing" kata. Graduate.
- Don't use Starter Kata as a substitute for coaching — if you have a coach available, use Coaching Kata

## Improv Integration

Starter Kata pairs with the **improv** skill for constructive coaching postures:

| Drill | Improv Mode | Why |
|-------|-------------|-----|
| Observation Drill | **Plussing** | Silently filter incorrect observations without discouraging the learner. Amplify correct fact/interpretation distinctions. |
| Five Questions Drill | **Yes And** | Reinforce correct answers to build momentum. Accept the learner's response and extend with the next question. |
| PDCA Cycle | **Yes But** | Constrain experiment scope — "yes, try that, but limit to one variable at a time." Narrows without contradicting. |

**Activation:** Use `/improv cascade` to compose the recommended sequence for your kata session.

**CNS span:** `cns.kata` — tracks automaticity score delta when improv is active vs. baseline. When improv modes are active, `cns.kata` carries derived sub-metrics from the improv runtime (mode active, plussing ratio, freestyle coherence, kata effectiveness delta, cascade depth). These are trace targets, not independent CNS spans.

## Registry Templates

This skill's runtime templates live in `registry/templates/kata-starter/`:

| Template | Type | Purpose |
|----------|------|--------|
| `starter-overview.j2` | KnowAct | Starter Kata practice overview — documentation-only (not in manifest flow) |
| `starter-selector.j2` | KnowAct | Select appropriate starter routine based on learner state (v0.31.0: selector output now drives conditional drill routing) |
| `starter-five-questions.j2` | KnowAct | Five Questions Drill — practice the 5 coaching questions |
| `starter-pdca-cycle.j2` | KnowAct | PDCA Cycle — Plan-Do-Check-Act experimentation practice |
| `starter-observation-drill.j2` | KnowAct | Observation Drill — distinguish facts from interpretations |

## Bundle Manifest

Process manifest: `registry/manifests/kata-starter.yaml` — 3-step practice flow (select routine → execute → record).

## Anti-Patterns

1. Skipping the drills because "I already understand the concepts" — intellectual understanding is not skill
2. Practicing sporadically — 20 minutes daily, not 2 hours weekly
3. Trying to solve real problems during practice — Starter Kata is about the process, not the outcome
4. Staying in Starter Kata indefinitely — graduate when the pattern is automatic
5. Using Starter Kata when a specific capability gap exists — that's what Improvement Kata is for


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/kata-starter.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = starter drill outcomes indicate stable foundational habit signals and low ambiguity

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ (absolute)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
