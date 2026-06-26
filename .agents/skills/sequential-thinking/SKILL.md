---
name: sequential-thinking
visibility: public
description: "Dynamic, reflective sequential thinking with branching, revision, hypothesis generation, and verification. Replicates the sequentialthinking MCP server as an agent-owned PDCA skill. Use when analyzing complex problems, debugging, planning, or any task requiring structured chain-of-thought reasoning."
---

# Sequential Thinking Skill

Dynamic, reflective sequential thinking engine that replicates the functionality of the sequentialthinking MCP server — but as an **agent-owned skill** rather than an external MCP tool call.

The agent manages its own thought process: numbering thoughts, branching to explore alternatives, revising earlier thoughts when new insights emerge, generating and verifying hypotheses, and dynamically adjusting the total thought count as understanding deepens.

## Architecture: PDCA Skill vs. MCP Tool

| Dimension | MCP Server Tool | hKask Skill |
|-----------|----------------|-------------|
| **Invocation** | Agent calls external tool per thought | Agent invokes skill once; template runs full chain internally |
| **State** | State lives in MCP server (external) | State lives in LLM context (agent-owned) |
| **Iteration** | Agent decides when to call next | PDCA loop in manifest drives convergence |
| **Convergence** | Manual (agent decides when done) | Automatic via convergence check template |
| **Branching** | Agent passes `branchFromThought` + `branchId` | LLM self-manages branching within the chain |
| **Gas/RJoule** | Unmetered external call | Metered via `gas.cap` + `rjoule.cap` in manifest |
| **OCAP** | No capability delegation | Full OCAP delegation chain with Ed25519 signatures |

## When to Activate

Activate this skill when the agent encounters:
- Complex multi-step problems requiring structured reasoning
- Debugging scenarios where root cause is unclear
- Planning or design tasks with trade-offs to evaluate
- Any situation where "think step by step" would help but needs more rigor than ad-hoc CoT
- Problems where the answer is non-obvious and requires hypothesis testing

**Trigger phrases:** "think through this", "analyze this step by step", "I need to reason about", "help me understand this problem"

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `sequential-thinking-engine.j2` | KnowAct | Execute the full sequential thinking chain — generate thoughts, branch, revise, hypothesize, verify, and synthesize a final answer |
| `sequential-thinking-convergence-check.j2` | KnowAct | Compute convergence metric from hypothesis verification status, chain completeness, and confidence calibration |

## PDCA Flow

```
┌─────────────────────────────────────┐
│  Step 1: sequential-thinking-engine │  ← KnowAct: full chain-of-thought
│  (gas: 9000, timeout: 90s)          │
├─────────────────────────────────────┤
│  Step 2: convergence-check          │  ← KnowAct: evaluate convergence
│  (gas: 2000, timeout: 30s)          │
├─────────────────────────────────────┤
│  Step 3: loop → Step 1 if not met   │  ← PDCA re-entry
└─────────────────────────────────────┘
```

## Inputs

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `problem` | string | yes | The problem or question to analyze |
| `domain` | string | no | Domain context (e.g., "Rust type system", "distributed systems") |
| `constraints` | array | no | Constraints that bound the solution space |
| `max_thoughts` | integer | no | Maximum thoughts per cycle (default: 12) |
| `thinking_budget` | string | no | LLM thinking budget: "full", "medium", "minimal", "off" (default: "full") |

## Outputs

The engine produces a structured JSON response:

| Field | Type | Description |
|-------|------|-------------|
| `thought_chain` | array | Ordered list of thought objects with numbering, branching, and revision metadata |
| `final_answer` | string | The synthesized, complete answer |
| `hypothesis` | object | The verified hypothesis with statement, method, and confidence |
| `hypothesis_verified` | boolean | Whether the hypothesis survived verification |
| `total_thoughts_used` | integer | Actual number of thoughts in the chain |
| `branches_explored` | array | All branches with outcome (dead-end, merged, etc.) |
| `revisions_made` | array | All revisions with reason |
| `solution_confidence` | number | Calibrated confidence in [0, 1] |

## Thinking Protocol

### Core Loop
1. Start with `thoughtNumber = 1`, estimate `totalThoughts`
2. For each thought: reason substantively, not meta-cognitively
3. **Branch** when alternative approaches need comparison
4. **Revise** when new insight contradicts prior reasoning
5. **Hypothesize** when sufficient evidence accumulates
6. **Verify** hypothesis against all evidence and edge cases
7. If verified → `needsMoreThoughts: false` → produce final answer
8. If not → adjust `totalThoughts` upward, continue

### Convergence Criteria
The convergence check scores [0,1] by subtracting from 1.0 for each satisfied condition:
- Hypothesis exists (-0.15)
- Hypothesis verified (-0.25)
- Chain complete (`needsMoreThoughts: false`) (-0.15)
- No unresolved branches (-0.10)
- No pending revisions (-0.10)
- Confidence calibrated (≥ 0.7) (-0.10)
- Answer synthesized (-0.10)
- Chain stable between iterations (-0.05)

Convergence threshold: **0.15** — achieved when hypothesis is verified AND chain is complete.

## Gas & Energy Budget

| Resource | Cap | Per Iteration |
|----------|-----|---------------|
| Gas | 100,000 | 100 |
| rJoule | 1 |
| Max iterations | 3 | — |
| Engine timeout | 90s | — |
| Check timeout | 30s | — |

## Comparison with MCP sequentialthinking

The MCP server's parameters map to the template's internal protocol:

| MCP Parameter | Template Equivalent |
|---------------|-------------------|
| `thought` | `thought_chain[i].thought` |
| `thoughtNumber` | `thought_chain[i].thoughtNumber` |
| `totalThoughts` | `thought_chain[i].totalThoughts` (auto-adjusted) |
| `nextThoughtNeeded` | `thought_chain[i].needsMoreThoughts` |
| `isRevision` | `thought_chain[i].isRevision` |
| `revisesThought` | `thought_chain[i].revisesThought` |
| `branchFromThought` | `thought_chain[i].branchFromThought` |
| `branchId` | `thought_chain[i].branchId` |
| `needsMoreThoughts` | `thought_chain[i].needsMoreThoughts` |

The key difference: in the MCP server, the **agent** calls the tool repeatedly with each parameter. In this skill, the **template prompts the LLM** to produce the entire chain in one structured output, with the PDCA loop handling re-entry for refinement.
