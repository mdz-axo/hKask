---
name: dokkodo-mindset
visibility: public
description: Metacognitive perceptual filter based on Miyamoto Musashi's 21 Dokkodo precepts. Applies the precepts as a lens to clarify perception by removing attachment, preference, resentment, fear, and customary bias. Composes with pragmatic-laziness — clears the perceptual field before the lazy loop evaluates the action landscape. Activate when the user says "apply the Dokkodo", "warrior mindset", "see this clearly", "perceptual reset", or when a decision feels distorted by attachment or preference.
---

# Dokkodo Mindset

A metacognitive perceptual filter. Applies Miyamoto Musashi's 21 Dokkodo precepts as a lens that transforms how the agent perceives a situation — not what to do, but *how to see*. Does not advise, does not plan, does not act. It *perceives*.

**Governing precept:** *Accept things exactly as they are* (Precept 1).
**Meta-precept:** *Never stray from the Way* (Precept 21).

## Architectural Role

The Dokkodo mindset sits at the **perceptual layer** — before regulative (constraint-forces), analytic (pragmatic-laziness, essentialist), and executive (coding-guidelines) layers. It transforms the perceived action landscape so that downstream processes evaluate a clarified, distortion-free situation.

```
Dokkodo Mindset (perceive) → Pragmatic Laziness (evaluate δS = 0) → Essentialist (delete)
         ↑                              ↑
    constraint-forces runs across all layers
```

**Key insight:** Mindset transforms what counts as "action." Attachment makes clinging look easier than releasing. Resentment makes complaint look easier than acceptance. Fear makes avoidance look easier than engagement. The Dokkodo removes these distortions so the true least-action path becomes visible.

## Composition Contract

| Property | Value |
|----------|-------|
| **Polarity** | Perceptual (pre-filter; runs before analysis) |
| **Type** | KnowAct — produces a judgment (clarified perception), not an artifact |
| **Composes with** | `pragmatic-laziness`, `essentialist`, `constraint-forces`, `coding-guidelines` |
| **Regulated by** | `constraint-forces` (Prohibitions/Guardrails never relaxed) |
| **Convergence** | δP = 0 — perception reaches stationary state; no further distortions found |
| **Max iterations** | 3, then escalate |

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "apply the Dokkodo" / "warrior mindset" / "see this clearly" | Apply full perceptual filter |
| "perceptual reset" / "clear the lens" | Cluster A only (perceptual reset) |
| "what am I attached to here?" / "where is desire distorting this?" | Cluster B only (desire/attachment) |
| "what emotional friction is here?" / "am I resenting this?" | Cluster C only (emotional resilience) |
| "what would this look like if I accepted death?" / "existential posture" | Cluster D only (existential posture) |
| "is my perception clear?" / "δP = 0?" | Convergence check only (no full filter) |

## The Four Precept Clusters

### Cluster A — Perceptual Reset (1, 4, 11, 12, 15, 19)
Clear the lens of self-importance, preference, custom, and wishful thinking. "Accept things exactly as they are" is the governing precept.

### Cluster B — Desire/Attachment Management (2, 5, 10, 13, 14, 18)
Identify where desire creates artificial gradients — paths that look easier because you want them to be, or harder because you're clinging to an alternative.

### Cluster C — Emotional Resilience (3, 6, 7, 8, 9)
Eliminate emotional friction: regret (energy spent on an unchangeable past), envy (comparing your path to another's), resentment (wishing the past were different), grief (attachment in retrospect).

### Cluster D — Existential Posture (16, 17, 20, 21)
The ultimate brachistochrone. When death is accepted, no path is too costly — the true least-action landscape becomes visible. Precept 21 is the meta-precept that resolves all conflicts.

## The Perceptual Transformation Loop

```
┌─────────────────────────────────────────────┐
│ STEP 1: IDENTIFY DISTORTIONS                 │
│                                              │
│ For each cluster, ask: where is perception   │
│ distorted by self, desire, emotion, or fear? │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ STEP 2: APPLY THE LENS                       │
│                                              │
│ State distortion → Apply precept →           │
│ Re-perceive without the distortion           │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ STEP 3: MAP LANDSCAPE SHIFT                  │
│                                              │
│ Which paths now appear shorter? Longer?      │
│ Any brachistochrone candidates?              │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ STEP 4: CONVERGENCE CHECK (δP = 0)           │
│                                              │
│ No new distortions + shift_magnitude ≈ 0?    │
│ → STATIONARY. Perception is clear.           │
│                                              │
│ New distortions found?                       │
│ → Continue if iterations remain.             │
│                                              │
│ Max iterations reached?                      │
│ → Escalate with clearest perception found.   │
└─────────────────────────────────────────────┘
```

## Output Structure

The template produces a JSON object with:
- `clarified_perception` — the situation seen through the Dokkodo lens
- `distortions_removed` — each distortion mapped to its precept and type
- `precept_alignment` — per-precept alignment scores (0.0–1.0)
- `action_landscape_shift` — how the perceived action landscape changed
- `brachistochrone_candidates` — paths that look harder short-term but minimize long-term action
- `stationary` / `should_continue` / `escalate` — convergence state

## Registry Template

This skill's runtime template lives in `registry/templates/dokkodo-mindset/`:

| Template | Type | Purpose |
|----------|------|---------|
| `dokkodo-perceive.j2` | KnowAct | Apply the 21 precepts as perceptual filter with embedded δP = 0 convergence |

One template. The skill is lazy. The convergence logic is embedded, not separated — following the `pragmatic-laziness-flow.j2` pattern.

## Lexicon Terms

`accept`, `assess`, `clarify`, `classify`, `converge`, `detach`, `discriminate`, `endure`, `evaluate`, `perceive`, `reflect`, `relinquish`, `renounce`

New to the vocabulary: `detach`, `endure`, `perceive`, `relinquish`, `renounce`. (`accept` was already present.)

## When to Use This Skill

- **Before pragmatic-laziness:** Clear the perceptual field so the lazy loop evaluates a distortion-free landscape.
- **When a decision feels stuck:** Attachment or preference may be creating a false local minimum.
- **When facing adversity:** The Dokkodo is fundamentally a resilience discipline — it's how the warrior maintains clarity when circumstances are hostile.
- **When regret, resentment, or envy appear:** These are friction that produces nothing. Apply Cluster C.
- **When "the obvious path" feels wrong:** Preference or custom may be masquerading as least action. Apply Cluster A.
- **Invoked from other skills:** `pragmatic-laziness` can invoke the Dokkodo as a pre-filter before Phase 1. `constraint-forces` can invoke it when a constraint conflict feels emotionally loaded.

## Quick Reference

1. **Identify distortions** — where is perception warped by self, desire, emotion, or fear?
2. **Apply the lens** — state the distortion, apply the precept, re-perceive.
3. **Map the shift** — what changed in the action landscape?
4. **Check convergence** — δP = 0? If not, repeat (max 3).
5. **Never relax** Prohibitions or Guardrails. The Dokkodo clarifies perception; it does not override sovereignty.
6. **Precept 21 resolves conflicts** — when precepts clash, the Way decides.

*"Accept things exactly as they are."* — Dokkodo, Precept 1
