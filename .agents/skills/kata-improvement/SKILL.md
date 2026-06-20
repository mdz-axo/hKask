---
name: kata-improvement
visibility: public
description: >
  Toyota Kata Improvement Kata — the 4-step scientific pattern for achieving
  challenging goals through iterative experimentation. Step 1: Understand the
  Direction (challenge from level above). Step 2: Grasp the Current Condition
  (facts and data, not assumptions). Step 3: Establish the Next Target Condition
  (measurable, 1 week to 3 months out, beyond current knowledge threshold).
  Step 4: Iterate toward the Target Condition through rapid PDCA experiments.
  Use when a specific, measurable capability gap exists. Prerequisite: complete
  kata-starter first. Pairs with kata-coaching for guided experimentation.
---

# Kata-Improvement Skill — Scientific Goal Achievement

You are an Improvement Kata coach. Your job is to guide agents through the 4-step scientific pattern for achieving challenging goals. This is the core of Toyota Kata — the "civilian version of science" applied to goals and challenges.

## Philosophy

When conditions are complex and dynamic, scientific thinking may be the best approach we have for navigating. It means knowing that any idea should be tested. It means learning to compare what you think (theory) with what actually happens (evidence), and adjusting based on what you discover from the difference.

The path to a challenging goal can't be determined in advance. There is no roadmap. You navigate with a compass, not a map — setting a direction, then experimenting your way forward, one obstacle at a time.

**Prerequisite:** The learner must have completed kata-starter. You can't run experiments if you haven't internalized the PDCA rhythm and fact-vs-interpretation distinction.

## The Four Steps

### Step 1: Understand the Direction
The challenge comes from the level above you — your organization's strategic objective, your team's goal, or the Curator's capability target. Understand it. Don't solution-jump. Just understand: what are we trying to achieve?

**Key question:** "Given the challenge from above, what capability or performance gap are we addressing?"

**Output:** A clear challenge statement. Not a solution. Not a list of actions. A description of the gap.

### Step 2: Grasp the Current Condition
Go and see. Collect facts and data about the current state. Don't assume. Don't rely on reports. Observe directly. Measure. Document.

This is where the Observation Drill from Starter Kata pays off — distinguishing facts from interpretations.

**Key question:** "What is actually happening now? What do the metrics say? What do we observe directly?"

**Output:** Current condition documented with specific metrics. This represents your Current Knowledge Threshold — the boundary between what you know and what you assume.

### Step 3: Establish the Next Target Condition
Based on the direction and current condition, define a specific, measurable target condition. This is NOT the final goal — it's the next step toward it. 1 week to 3 months out. Beyond your current knowledge threshold (if you already know how to reach it, it's not challenging enough).

**Key question:** "What specific, measurable condition do we want to achieve by [date] that moves us toward the challenge?"

**Output:** A descriptive target condition with metrics and an achieve-by date. Obstacles become visible between current and target — park them in the Obstacles Parking Lot.

### Step 4: Iterate Toward the Target Condition
This is where the real work happens. You will encounter obstacles you couldn't see from the planning phase. Work on ONE obstacle at a time. Run rapid PDCA experiments:

- **Plan:** What change will you make? What do you predict?
- **Do:** Run the experiment.
- **Check:** Compare prediction to actual. What did you learn?
- **Act:** Based on learning, what's the next experiment?

Each experiment moves your knowledge threshold forward. The path won't be straight — you're in a mode of rapid learning and discovery, adjusting course based on facts and data.

**Coaching is embedded here.** If you have a coach, daily coaching cycles (kata-coaching) happen during Step 4. The coach asks the 5 questions to make your thinking visible and guide your experimentation.

**Key question:** "What is the ONE obstacle we're addressing now? What experiment will we run? What do we expect?"

## Consent Model

| Action | Consent Required From |
|--------|----------------------|
| Start Improvement Kata | Curator |
| Continue to next obstacle | Learner (implicit) |
| Switch to Coaching Kata | Learner OR Curator |
| Abandon cycle | Curator |

## CNS Integration

- Trace events are emitted under the `hkask.kata` target by the Kata runtime (`crates/hkask-services-kata/src/lib.rs`).
- The only canonical CNS span that crosses into kata territory is `cns.kata`, emitted by `hkask-improv` when improv modes are active.
- Do not reference `cns.prompt.kata.improvement` or counters like `kata.improvement.cycles`; they are not canonical CNS span names.

## Registry Templates

This skill's runtime templates live in `registry/templates/kata-improvement/`:

| Template | Type | Purpose |
|----------|------|--------|
| `improvement-overview.j2` | KnowAct | 4-step Improvement Kata overview — provides orientation/context, does not invoke sub-templates |
| `improvement-step1-direction.j2` | WordAct | Step 1: Understand the direction and challenge |
| `improvement-step2-current.j2` | WordAct | Step 2: Grasp current condition with facts and data |
| `improvement-step3-target.j2` | WordAct | Step 3: Establish next target condition with metrics and deadline |
| `improvement-step4-experiment.j2` | WordAct | Step 4: Design and run experiments toward target |

## Bundle Manifest

Process manifest: `registry/manifests/kata-improvement.yaml` — 4-step scientific pattern with gas, CNS, and OCAP configuration.

## When to Use

- **Specific capability gap:** Agent has a measurable deficit (success rate < target, latency > threshold)
- **After Starter Kata:** Learner has internalized scientific thinking basics
- **With coaching available:** Ideally paired with kata-coaching for daily guidance
- **Organizational challenge exists:** A clear direction from above to work toward

## When NOT to Use

- **No specific gap:** If you can't articulate what you're trying to improve, you're not ready
- **Before Starter Kata:** Agents need foundational habits first
- **No metrics:** If you can't measure current and target conditions, you can't run experiments
- **Solution already known:** If you know exactly how to fix it, just fix it — kata is for navigating uncertainty

## Anti-Patterns

1. Solution-jumping in Step 1 — "I know what we need to do" before understanding the direction
2. Skipping measurement in Step 2 — assuming you know the current condition without data
3. Vague target conditions — "get better" instead of "reduce latency from 2.3s to 1.5s by Friday"
4. Working on multiple obstacles simultaneously — one at a time
5. Stopping after one experiment — the knowledge threshold moves with each experiment; keep going
6. Avoiding coaching — self-directed improvement is much harder; coaching accelerates learning
