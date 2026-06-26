---
name: essentialist
visibility: public
description: "General-purpose recursive eliminative interrogation. Enforces 'always take away, never add' through a 3-gate challenge loop (Exist → Surface → Contract) the actor must survive before any artifact can be committed. Use when the user says 'simplify this,' 'what can be deleted,' 'audit complexity,' 'strip this module,' 'is this necessary,' or 'essentialist review.'"
composes_skills: [deep-module, coding-guidelines, pragmatic-semantics]
---

# Essentialist Skill — Eliminative Interrogation

A general-purpose skill enforcing *eliminative minimalism*: every artifact (module, function, trait, type, parameter, dependency) is assumed guilty until proven necessary through a 3-gate recursive challenge loop. The governing principle is **"always take away, never add"** — reduction is the default, preservation must be earned.

## Philosophy

Essentialism inverts the usual design question. Instead of asking "what should I add?", it asks "what happens if I delete this?" Every artifact must survive three escalating gates of interrogation. Artifacts that fail any gate must be reduced and resubmitted. Only artifacts that survive all three gates — twice, with zero deltas on the second pass — may be committed.

## Trigger Conditions

Activate this skill when the user says:
- "simplify this" / "simplify this module" / "make this simpler"
- "what can be deleted" / "what can I delete" / "what is unnecessary"
- "audit complexity" / "audit this for complexity"
- "strip this module" / "strip this down"
- "is this necessary" / "is this abstraction needed"
- "essentialist review" / "run the essentialist"
- "what can go" / "what can be removed"

## Operating Modes

Choose the mode based on the user's phrasing:

| User says | Mode | Behavior |
|-----------|------|----------|
| "simplify this", "strip this", "run the essentialist" | **autonomous** | Agent evaluates, reduces, and loops without pause. Human only sees final report and escalations. |
| "review this", "audit this", "suggest reductions", "what should I delete" | **advisory** | Agent evaluates, presents findings per gate, and pauses for human approval before acting. Human can accept, reject, or override each recommendation. |
| Unclear | Default to **advisory** (safer — no autonomous deletions) |

In advisory mode, each gate produces a recommendation set with constraint-force labels. The human reviews and responds `accept` / `reject` / `override` per item. The loop only continues after human signoff on the current gate's reductions.

## The Three Gates

### G1 — Exist (Deletion Test)

**Apply Ousterhout's deletion test.** Ask two questions about the artifact:

**Direction 1 — Caller's perspective:**
> Delete the code that uses this artifact. If the *complexity* reappears in the callers, the artifact earns its keep. If callers can inline a trivial replacement, the artifact is a pass-through.

**Direction 2 — Artifact's perspective:**
> Delete the artifact entirely. If the *complexity* vanishes (a direct call to the dependency does the same thing), the artifact was just wiring — don't create it.

**Verdict:** If no behavior vanishes on deletion, the artifact must be pruned. Don't create or preserve what doesn't encode behavior.

Delegate to: `deep-module/deep-module-delete` (hKask) or manual deletion test reasoning.

### G2 — Surface (Interface Count)

**Count the public surface.** List every public function, type, trait, constant. If the count exceeds 7, each additional public item must carry a written justification of why it cannot be merged with an existing public item.

**Challenge question:** "What if this had exactly one public function? What would it be? Why do the others exist as separate public items?"

**Verdict:** Every public item is a liability (it must be tested, documented, maintained, and preserved across versions). If > 7 without justification, reduce the surface.

Delegate to: `deep-module/deep-module-assess` (hKask) or manual surface count.

### G3 — Contract (Abstraction Trace)

**Trace every abstraction boundary.** For every port, adapter, trait, wrapper, or interface:

> What behavior is lost if I replace this abstraction with a direct call to its single dependency?

If the answer is "nothing" — delete the port. If the abstraction wraps exactly one dependency with no added behavior, value, or information hiding, it is a pass-through abstraction.

**Also audit:** Single-use traits, traits with one implementor, generic parameters with one concrete type, config structs passed through untouched, error types that just wrap one inner error.

**Verdict:** Delete every abstraction that doesn't encode behavior beyond what a direct call would provide.

Delegate to: `coding-guidelines/guidelines-verify` (hKask) with focus on single-use abstraction violations (Simplicity First, anti-pattern #2).

## The Recursive Loop

```
┌─────────────────────────────────────────────┐
│ Submit artifact for eliminative review       │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ Gate 1 — EXIST: Deletion test               │
│ Does behavior vanish if deleted?            │
├─────────────────────────────────────────────┤
│ PASS → Proceed to G2                        │
│ FAIL → Reduce (prune pass-throughs) → G1    │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ Gate 2 — SURFACE: Public item count         │
│ ≤ 7? Every extra justified?                 │
├─────────────────────────────────────────────┤
│ PASS → Proceed to G3                        │
│ FAIL → Merge/justify → G2                   │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ Gate 3 — CONTRACT: Abstraction trace         │
│ Every port traced to dependency.            │
│ Behavior lost if replaced with direct call? │
├─────────────────────────────────────────────┤
│ PASS → Narrow scope, repeat G1→G2→G3        │
│ FAIL → Delete pass-through abstractions     │
└────────────────────┬────────────────────────┘
                     ▼
┌─────────────────────────────────────────────┐
│ Repeat with narrowed scope.                 │
│ If pass again with ZERO deltas → DONE.      │
│ If changes made → continue looping.         │
│ Max 3 full rounds; escalate to human after. │
└─────────────────────────────────────────────┘
```

## Recursion Rules

1. Apply G1 → G2 → G3 to the artifact in strict order.
2. If any gate fails, the actor (human or agent) must reduce the artifact and resubmit from G1.
3. On first complete pass (all three gates pass), narrow the scope and repeat the full G1→G2→G3 sequence.
4. On second pass with **zero deltas** (no further reductions found), stop — the artifact is essential.
5. If the second pass finds further reductions, apply them and repeat once more (max 3 agent-level retries per gate).
6. After 3 agent-level retries on any single gate, escalate to human with a report of what survived, what was pruned, and what remains contested.

## Required vs Suggested — Constraint-Force Classification

Every finding carries a **constraint force** from the `pragmatic-semantics` hierarchy. This determines whether the recommendation is required or merely suggested:

| Force | Label | Meaning | Example |
|-------|-------|---------|---------|
| Prohibition | **REQUIRED** | Inviolable. Must be fixed. | "Artifact encodes zero behavior — pass-through with no encapsulation." |
| Guardrail | **REQUIRED (overridable)** | Must be fixed unless human explicitly overrides. | "12 public items — 4 lack justification. Reduce or justify." |
| Guideline | **SUGGESTED** | Best practice. Actor should apply unless a stated reason not to. | "Consider merging these two handlers with overlapping domains." |
| Evidence | **INFO** | Observation. Not a directive. | "Depth score: 14 — shallow by Ousterhout metric." |
| Hypothesis | **SPECULATIVE** | Tentative. Needs verification. | "This trait may be single-use if AdapterV2 is complete." |

**In autonomous mode**: Prohibitions and Guardrails cause gate failure (require reduction). Guidelines, Evidence, Hypotheses are reported but don't cause gate failure — they appear in the elimination report as informational findings.

**In advisory mode**: All forces are presented to the human with recommendation strength. The human decides action per item.

## Paired Skills

This skill pairs naturally with:
- **deep-module** — For G1 (deletion test) and G2 (surface count, depth score)
- **coding-guidelines** — For G3 (single-use abstraction audit, Simplicity First violations)
- **pragmatic-semantics** — For classifying findings by required vs suggested force

## Output Format

On completion, produce an **elimination report**:

```markdown
## Essentialist Review — [Artifact Name]

### Gate 1 — Exist
- Items deleted: N
- Items preserved: M
- Required reductions (Prohibition + Guardrail): N
- Suggested reductions (Guideline): N
- Deletion test results per item: [summary]

### Gate 2 — Surface
- Public items before: X, after: Y
- Items merged/deleted: [list]
- Justifications accepted: [list]

### Gate 3 — Contract
- Abstractions traced: N
- Pass-through deletions: K
- Retained with justification: J

### Essentialism Score
- Items removed: R / total items: T → (R/T * 100)%
- Required reductions: N_REQUIRED (Prohibition + Guardrail)
- Suggested reductions: N_SUGGESTED (Guideline only)
- Rounds required: N
- Escalated: yes/no
```

In **advisory mode**, the report also includes:
- Recommendations accepted by human: N
- Recommendations rejected by human: N (with human's stated reason)
- Recommendations overridden by human: N (human chose different action)

## hKask Runtime Integration

When running within hKask, use the `essentialist/essentialist-flow` KnowAct template, which orchestrates:
- Iteration over `[G1, G2, G3]` gates
- Delegation to `deep-module/deep-module-delete` (G1), `deep-module/deep-module-assess` (G2), `coding-guidelines/guidelines-verify` (G3)
- `choice` on pass/fail with gate-specific branching
- `escalate` on retry exhaustion (3 max)
- `abort` on zero-delta completion

Outside hKask, follow the same 3-gate process manually with the same recursion rules.

## Mode Decision Tree

```
User says "review/audit/suggest/what should I delete?"
└── advisory mode
    ├── Gate 1: Present EXIST findings → await human accept/reject/override
    ├── Gate 2: Present SURFACE findings → await human accept/reject/override
    ├── Gate 3: Present CONTRACT findings → await human accept/reject/override
    └── On pass: Narrow scope, repeat with human in loop

User says "simplify/strip/run the essentialist"
└── autonomous mode
    ├── Gate 1: Evaluate → reduce → retry (max 3) → escalate if stuck
    ├── Gate 2: Evaluate → reduce → retry (max 3) → escalate if stuck
    ├── Gate 3: Evaluate → reduce → retry (max 3) → escalate if stuck
    └── On pass: Narrow scope, repeat. Zero deltas → done.
```

## Quick Reference

Before committing any artifact, ask:
1. **Mode:** Agentic (autonomous loop) or advisory (human-in-the-loop)? Default advisory.
2. **Exist:** If I delete this, does any behavior vanish? If no → prune.
3. **Surface:** More than 7 public items? If yes → merge or justify each.
4. **Contract:** Can I replace this abstraction with a direct call? If yes → delete the port.
5. **Loop:** Passed all three? Narrow scope and repeat. Zero deltas on repeat? Done.
6. **Force:** Is each finding required (Prohibition/Guardrail) or suggested (Guideline/Evidence/Hypothesis)?

"Always take away, never add" — the default answer to "should this exist?" is **no**.

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `essentialist-flow.j2` | KnowAct | Run the 3-gate eliminative interrogation loop |


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/essentialist.yaml`

### PDCA Convergence
- **Threshold:** 0.25 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = no material eliminations remain

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 16000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
