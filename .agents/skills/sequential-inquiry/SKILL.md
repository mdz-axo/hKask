---
name: sequential-inquiry
visibility: public
description: "Structured chain-of-thought reasoning with branching, revision, hypothesis testing, and automatic deep-dive delegation to hypothesis-framer, mcda, and diagnose. The engine decides at runtime whether delegation is needed — no pre-selection. Subsumes the deprecated sequential-thinking skill."
---

# Sequential Inquiry Skill

**The primary structured reasoning skill for hKask.** Provides dynamic chain-of-thought with branching, revision, and hypothesis verification. When the engine identifies a thought needing deeper analysis, it emits delegation requests; the flowdef dispatches to hypothesis-framer (FINER+PICO), mcda (weighted comparison), or diagnose (root-cause analysis). Results feed back into the next PDCA cycle.

**No pre-selection needed.** The engine decides at runtime whether to delegate. If no delegation requests are emitted, the delegate steps return `invoked: false` and the skill functions as pure sequential thinking. Use for any reasoning task — from simple decomposition to compound inquiry.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│ Step 1: sequential-inquiry-engine (Think)            │
│   • decompose, branch, revise, hypothesize, verify   │
│   • emit delegation_requests when deep-dive needed   │
│   • on re-entry: weave prior_delegation_results into │
│     new thought chain                                │
├──────────────────────────────────────────────────────┤
│ Step 2: delegate-hypothesis-framer                   │
│   • no-op if no h-f request, else FINER + PICO       │
├──────────────────────────────────────────────────────┤
│ Step 3: delegate-mcda                                │
│   • no-op if no mcda request, else criteria weighting│
│     + scoring + compensation masking + sensitivity   │
├──────────────────────────────────────────────────────┤
│ Step 4: delegate-diagnose                            │
│   • no-op if no diagnose request, else repro strategy│
│     + ranked hypotheses + instrumentation plan       │
├──────────────────────────────────────────────────────┤
│ Step 5: convergence-check (10-criterion, 0.15)       │
├──────────────────────────────────────────────────────┤
│ Step 6: loop → Step 1                                │
└──────────────────────────────────────────────────────┘
```

## vs. Sequential Thinking

| Dimension | sequential-thinking | sequential-inquiry |
|-----------|--------------------|--------------------|
| **Role** | Decomposition + sorting | Decomposition + sorting + deep-dive delegation |
| **Sub-skills** | None | hypothesis-framer, mcda, diagnose |
| **Delegation** | N/A | Engine emits `delegation_requests`, flowdef dispatches |
| **Convergence criteria** | 8 (hypothesis + chain) | 10 (+ delegation resolution) |
| **Gas cap** | 100,000 | 150,000 |
| **rJoule cap** | 24,000 | 32,000 |
| **Steps** | 3 | 6 |

## When to Use

Use **sequential-thinking** when the problem can be solved through pure reasoning — no structured methodology beyond CoT is needed.

Use **sequential-inquiry** when the problem likely requires:
- Formal hypothesis validation (hypothesis-framer)
- Weighted comparison of alternatives (mcda)
- Structured root-cause diagnosis (diagnose)

The engine decides which to invoke at runtime based on the thought content. You don't pre-select — the flowdef handles it.

## Delegation Flow

```mermaid
sequenceDiagram
    participant E as Engine
    participant HF as Hypothesis-Framer
    participant MC as MCDA
    participant DI as Diagnose
    participant CV as Convergence

    E->>E: Think (cycle 1)
    E-->>HF: delegation_request: hypothesis-framer
    E-->>DI: delegation_request: diagnose
    HF-->>E: FINER+PICO result (cycle 2)
    DI-->>E: Root cause hypotheses (cycle 2)
    E->>E: Think (cycle 2, weaves results)
    CV->>CV: All delegations resolved? Chain complete?
    CV-->>E: metric ≤ 0.15 → converged
```

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `sequential-inquiry-engine.j2` | KnowAct | Inquiry engine with delegation awareness |
| `sequential-inquiry-delegate-hypothesis-framer.j2` | KnowAct | FINER + PICO delegate (no-op if not requested) |
| `sequential-inquiry-delegate-mcda.j2` | KnowAct | MCDA delegate (no-op if not requested) |
| `sequential-inquiry-delegate-diagnose.j2` | KnowAct | Diagnose delegate (no-op if not requested) |
| `sequential-inquiry-convergence-check.j2` | KnowAct | 10-criterion convergence with delegation resolution |

## Gas & Energy

| Resource | Cap | Per Iteration |
|----------|-----|---------------|
| Gas | 120,000 | 100 |
| rJoule | 2 |
| Max iterations | 3 | — |
| Engine timeout | 90s | — |
| Delegate timeout | 60s each | — |
| Check timeout | 30s | — |
