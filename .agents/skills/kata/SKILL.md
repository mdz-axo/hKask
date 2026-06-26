---
name: kata
visibility: public
description: >
  Toyota Kata system — composes kata-starter, kata-improvement, and kata-coaching
  into a complete scientific capability development practice. Routes agents to
  the appropriate kata based on context: new agents start with kata-starter to
  build foundational habits, agents with specific capability gaps use
  kata-improvement, and coaches use kata-coaching to guide learners. Includes
  composition rules (starter→improvement transitions, improvement↔coaching
  switching), habit monitoring with CNS integration, iteration with variance
  assessment, carbon accounting, and OCAP consent enforcement. Use when deploying
  the full Toyota Kata methodology across a team of agents.
---

# Kata Skill — Toyota Kata System

You are the Kata system orchestrator. Your job is to compose the three Toyota Kata practices — Starter, Improvement, and Coaching — into a coherent scientific capability development system. You route agents to the right kata at the right time and manage transitions between them.

## Architecture: Three Independent Skills, One System

The kata system is composed of three independently usable skills, each designed for a different stage of scientific thinking development:

| Skill | Role | Entry Point | Consent |
|-------|------|-------------|---------|
| **kata-starter** | Build foundational habits | New agents, low automaticity | Self |
| **kata-improvement** | Achieve challenging goals | Agents with specific capability gaps | Curator |
| **kata-coaching** | Teach scientific thinking | Coach + learner during active IK | Learner OR Curator |

Each skill can be used standalone. An agent doesn't need the full system — they can start with `kata-starter` today. This mirrors the real-world adoption pattern: print the 5-question card, practice one routine, then expand.

## Adoption Path

```
Day 1:   kata-starter (Five Questions Drill)
         → Internalize the questioning pattern
Week 1:  kata-starter (PDCA Cycle, Observation Drill)
         → Build experimental mindset
Week 2:  kata-improvement (first IK cycle)
         → Apply scientific thinking to a real capability gap
Week 3+: kata-improvement + kata-coaching
         → Daily coaching cycles during Step 4 experimentation
```

## Routing Logic

The bundle uses `kata-selector.j2` to route agents based on context:

| Condition | Route to |
|-----------|----------|
| New agent (no prior kata sessions) | kata-starter |
| Automaticity score < 0.3 | kata-starter (Observation Drill) |
| 7+ days since last practice | kata-starter (refresh) |
| Specific capability gap + auto > 0.5 | kata-improvement |
| Active IK cycle + coach available | kata-coaching |
| Habit decay (3+ days no practice) | Habit intervention |

## Composition Rules

- **Starter → Improvement transition:** Allowed when automaticity > 0.5 and a specific capability gap exists. Requires Curator consent.
- **Improvement ↔ Coaching switching:** Allowed when obstacles are thinking-pattern-related (requires learner consent) or when a coach is available during Step 4 (requires Curator consent).
- **Starter is self-contained:** No switching into or out of starter during a session. Starter Kata sessions are atomic practice units.
- **Max 2 iterations:** Per Improvement Kata session for variance assessment.
- **Nested kata forbidden:** Don't run a kata inside another kata.

## CNS Integration

The bundle emits structured tracing events under the `hkask.kata` target. The canonical CNS spans are managed by the Kata runtime (`crates/hkask-services-kata/src/lib.rs`) and by the improv module (`crates/hkask-improv/src/kata.rs`) when improv is active:

| Span | Source | Meaning |
|------|--------|---------|
| `cns.kata` | `hkask-improv` | Automaticity score delta when improv modes are active vs. baseline |

Variety and outcome counters are derived from the `hkask.kata` trace events; they are not separate canonical CNS span names. Do not invent span namespaces like `cns.prompt.kata` or `kata.practices.completed` — the runtime does not emit them.

## Registry Templates

This bundle's orchestration templates live in `registry/templates/kata/`:

| Template | Type | Purpose |
|----------|------|--------|
| `consent-and-select.j2` | KnowAct | Verify P2 consent and select kata type (subsumes former kata-selector, removed v0.31.0) |
| `starter-cycle.j2` | KnowAct | Bridge template: kata-pattern → kata-starter delegation (v0.31.0) |
| `improvement-cycle.j2` | KnowAct | Bridge template: kata-pattern → kata-improvement delegation (v0.31.0) |
| `coaching-cycle.j2` | KnowAct | Bridge template: kata-pattern → kata-coaching delegation (v0.31.0) |
| `outcome-and-habit.j2` | KnowAct | Synthesize kata outcome with habit assessment |
| `habit-intervention.j2` | WordAct | Generate intervention when habit is at risk |
| `iteration-check.j2` | KnowAct | Check if iteration is needed (variance or low confidence) |
| `iteration-comparison.j2` | KnowAct | Compare iterations for variance and confidence |
| `kata-switch-check.j2` | KnowAct | Validate kata switching against composition rules |
| `kata-convergence-check.j2` | KnowAct | Compute normalized convergence for kata practice sessions |

## Bundle Manifests

| Manifest | Purpose |
|----------|--------|
| `registry/manifests/kata-pattern.yaml` | Unified orchestration — routes to starter/improvement/coaching |
| `registry/manifests/kata-starter.yaml` | kata-starter skill manifest (standalone or invoked by bundle) |
| `registry/manifests/kata-improvement.yaml` | kata-improvement skill manifest (standalone or invoked by bundle) |
| `registry/manifests/kata-coaching.yaml` | kata-coaching skill manifest (standalone or invoked by bundle) |

## Individual Skills

For detailed instructions on each practice, see the individual skill files:

- `.agents/skills/kata-starter/SKILL.md` — Building foundational scientific thinking habits
- `.agents/skills/kata-improvement/SKILL.md` — 4-step scientific pattern for goal achievement
- `.agents/skills/kata-coaching/SKILL.md` — 5-question dialogue for teaching scientific thinking

## When to Use the Bundle

- **Deploying kata across a team:** Multiple agents at different stages need coordinated routing
- **Full adoption:** An agent has completed starter, is running improvement cycles, and has a coach
- **Monitoring:** CNS habit tracking derives from `hkask.kata` trace events; it does not rely on invented span namespaces.

## When NOT to Use the Bundle

- **First day:** Just use kata-starter. Start with one practice routine.
- **Single agent with a gap:** Just use kata-improvement with a coach using kata-coaching.
- **No coach available:** Use kata-improvement standalone. Self-directed improvement is harder but possible.

## Anti-Patterns

1. Deploying the full bundle on day one — adopt incrementally, one kata at a time
2. Skipping Starter Kata — agents without foundational habits will struggle with improvement
3. Coaching without an active IK cycle — the 5 questions need a target condition to reference
4. Treating the bundle as a rigid process — the individual skills are independently useful
5. Ignoring CNS alerts — variety deficits and habit decay require intervention, not silence


## Registry Manifest

**Type:** Bundle | **Manifest:** 

### PDCA Convergence
- **Threshold:** 0.20 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** both
- **Max iterations:** 3
- **Convergence meaning:** 0 = practice session complete, outcome achieved, habit signal positive

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 5 rJ (absolute)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
