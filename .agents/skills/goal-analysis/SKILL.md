---
name: goal-analysis
visibility: public
description: Goal specification and verification. Extracts structured goals from user intent, judges completion via semantic evaluation or command execution, and produces calibrated verdicts with confidence scoring. Use when you need to track whether a specific objective has been achieved, when you want lightweight goal tracking without kanban boards, or when an agent's work output needs structured completion verification.
activation: "create a goal"
---

# Goal Analysis

Lightweight goal specification and verification. Extracts structured goals from user intent, tracks them through activation, and judges completion via semantic evaluation of outcomes against original criteria. A minimal alternative to kanban for single-goal tracking — no boards, no swimlanes, just "what are we trying to achieve and have we achieved it?"

## Why Goal Analysis?

Between "I should probably do X" (informal intention) and full kanban task management sits a gap. Goal analysis fills it: extract a structured goal with explicit completion criteria, track it through activation, and verify completion with calibrated confidence.

The key differentiator from kanban:
- **Kanban** manages *workflow* — columns, swimlanes, WIP limits, delegation
- **Goal analysis** manages *objectives* — "what are we trying to achieve?" and "did we?"

A goal is not a task. A task is something you do. A goal is something you achieve. Goal analysis bridges intention and outcome.

## The Goal Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│ 1. CREATE                                                    │
│                                                              │
│ Extract structured goal from user intent:                    │
│  • Goal text — what are we trying to achieve?               │
│  • Completion criteria — how will we know it's done?         │
│  • Visibility — private, shared, or public                   │
│  • Priority — low, medium, or high                           │
│                                                              │
│ Output: goal (id, text, criteria[], visibility, priority)    │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. ACTIVATE → EXECUTE                                        │
│                                                              │
│ Goal is stored and activated. Agent works toward it.         │
│ Goal context (id, text, criteria) is available to the        │
│ agent throughout execution.                                  │
│                                                              │
│ CNS span emitted: cns.goal.create                            │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. VERIFY                                                    │
│                                                              │
│ Judge completion via one of three methods:                   │
│                                                              │
│ JUDGE (semantic): LLM evaluates outcome summary and          │
│   produced artifacts against original criteria.              │
│   Produces: done/continue/blocked + confidence.              │
│                                                              │
│ JUDGE COMMAND: Execute a command and check output against    │
│   acceptance criteria. Produces: done/continue/blocked.      │
│                                                              │
│ JUDGE SIMPLE (fallback): Minimal evaluation when full        │
│   semantic judgment is unavailable. Returns "continue"       │
│   with default confidence for lightweight checks.            │
│                                                              │
│ Output: verdict, reason, confidence (0.0–1.0)               │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. RESOLVE                                                   │
│                                                              │
│ • done → mark complete, emit CNS span                        │
│ • continue → loop back to execution                          │
│ • blocked → escalate to human                                │
│                                                              │
│ If confidence < 0.7 → request human review regardless        │
│ of verdict. Low-confidence "done" may still be wrong.        │
└─────────────────────────────────────────────────────────────┘
```

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "create a goal to..." / "track this objective" / "goal analysis" | Full lifecycle — create → activate → execute → verify |
| "did I achieve..." / "verify goal X" / "is this done?" | Verify only — judge completion against criteria |
| "what are my active goals?" / "goal status" | List goals with current state |
| "what were the criteria for goal X?" / "goal details" | Retrieve goal specification |

## Composition

- **Decision-journal:** Record the decision to pursue a goal — what was the reasoning, what alternatives were considered? Goal analysis tracks the outcome; the journal tracks the judgment quality.
- **Essentialist:** Before creating a goal, ask: does this goal earn its existence against the deletion test? Will achieving it reduce total system action?
- **Structured-extraction:** Extract structured goal definitions from narrative descriptions of desired outcomes.
- **MCDA:** When choosing between competing goals, MCDA ranks them on weighted criteria (impact, feasibility, urgency).

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `create.j2` | WordAct | Extract structured goal from user intent |
| `judge.j2` | KnowAct | Semantic completion verification with confidence scoring |
| `judge_command.j2` | KnowAct | Command-based completion verification |
| `judge_simple.j2` | KnowAct | Fallback minimal verification |

## Quick Reference

1. **Create** — extract goal with explicit completion criteria from intent
2. **Activate** — store and begin tracking
3. **Execute** — agent works with goal context available
4. **Verify** — semantic judge, command judge, or simple fallback
5. **Resolve** — done / continue / blocked, with confidence-gated human review

*"Shared language + shared goals = productive cooperation."* — Scott Page, *The Difference*


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/goal-analysis.yaml`

### PDCA Convergence
- **Threshold:** 0.25 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = verdict is confidently resolved for current cycle

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 14000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
