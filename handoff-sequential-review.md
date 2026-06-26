# Handoff: Sequential Thinking / Sequential Inquiry — Adversarial Review

## Session Summary

Built two skills to replicate the sequentialthinking MCP server as agent-owned PDCA flowdefs:

**sequential-thinking** — Pure decomposition + sorting. 3-step PDCA (engine → converge → loop). KnowAct engine template runs full thought chain with branching, revision, hypothesis/verify in one LLM invocation.

**sequential-inquiry** — Compound skill. 6-step PDCA (think → delegate-h-f → delegate-mcda → delegate-diagnose → converge → loop). Engine emits `delegation_requests`; flowdef dispatches to sub-skill methodology delegates; results feed back via `prior_delegation_results` on next cycle.

The session agent lost calibration on rJoule economics and needs adversarial review of all work product.

## Files

| File | Status |
|------|--------|
| `registry/manifests/sequential-thinking.yaml` | Exists — rJoule 24k is unsane (see below) |
| `registry/manifests/sequential-inquiry.yaml` | Exists — rJoule/gas flailed repeatedly, final values unsane |
| `registry/templates/sequential-thinking/sequential-thinking-engine.j2` | Clean |
| `registry/templates/sequential-thinking/sequential-thinking-convergence-check.j2` | Clean |
| `registry/templates/sequential-inquiry/sequential-inquiry-engine.j2` | Template logic sound; contracts need review |
| `registry/templates/sequential-inquiry/sequential-inquiry-convergence-check.j2` | 10-criterion; delegation resolution criteria need scrutiny |
| `registry/templates/sequential-inquiry/sequential-inquiry-delegate-hypothesis-framer.j2` | FINER+PICO inline methodology |
| `registry/templates/sequential-inquiry/sequential-inquiry-delegate-mcda.j2` | MCDA inline methodology |
| `registry/templates/sequential-inquiry/sequential-inquiry-delegate-diagnose.j2` | Diagnose inline methodology |
| `.agents/skills/sequential-thinking/SKILL.md` | Companion docs |
| `.agents/skills/sequential-inquiry/SKILL.md` | Companion docs — rJoule table values unsane |
| `AGENTS.md` | sequential-thinking added to Reasoning & Analysis; sequential-inquiry NOT added yet |

## Review Framework — Five Anchoring Skills

Activate these skills in order. Each provides a distinct lens. Do not proceed to the next until the current lens has produced actionable findings.

### Lens 1: coding-guidelines

**Activate first.** Before any code changes, surface assumptions and verify goal-driven execution.

Questions to answer:
- What assumptions did the session agent make about how `rjoule.cap`, `cost_per_token`, and `gas.cap` interact?
- Were changes surgical or did they sprawl (the rJoule value flailing from 48k → 32k → 28k → 5)?
- What is the success criterion: "both manifests have economically sane budgets consistent with system convention"?
- Was there speculative complexity added that the problem didn't require?

### Lens 2: pragmatic-semantics

Classify every claim in the manifests and templates by certainty level.

Questions to answer:
- IS vs OUGHT: Which template contract fields are declarative (enforced by the renderer) vs aspirational?
- The delegate template contract says `output.result: object` — what actual structure does the convergence check expect? Is the contract precise enough?
- `convergence_field: step_5_result.convergence_metric` — is this IS (the executor enforces this path) or OUGHT (the template should produce it)?
- The engine template instructs the LLM to "emit delegation_requests" — is this probabilistic (LLM may or may not) or deterministic? How does the convergence check handle the case where no delegation requests are emitted?

### Lens 3: pragmatic-cybernetics

Map the feedback loops and check for variety deficits and homeostatic failure modes.

Questions to answer:
- The delegation feedback loop: engine → delegate steps → convergence → loop → engine. What is the latency (1 full cycle)? Is this acceptable for the problem domain?
- What happens when the engine emits a delegation request but the delegate returns `invoked: false` (mismatch between engine expectation and delegate's request matching)?
- What happens when the engine emits 3 delegation requests but only 2 delegate steps match (e.g., two hypothesis-framer requests, no mcda request)?
- The convergence check has 10 criteria subtracting from 1.0. Can the scoring saturate at 0.0 without all criteria being met? Is there a homeostatic failure where the metric stays stuck at a non-converging value?
- Variety engineering: the engine has 3 delegation targets. Is this sufficient variety for the problem space, or does it create a coupling where every deep analysis must fit one of these three molds?

### Lens 4: grill-me

Stress-test every architectural decision with escalating Socratic probing.

Questions to probe:
- Why does sequential-inquiry exist as a separate skill rather than extending sequential-thinking with optional delegation?
- Why are the delegate templates inline methodology replicas rather than invoking the actual hypothesis-framer/mcda/diagnose flowdefs?
- What prevents the engine from emitting delegation requests for skills that don't have delegate steps in the manifest?
- At max 3 PDCA iterations and 1-cycle delegation latency, can the engine ever incorporate delegation results more than twice? Is this sufficient?
- If the answer is "the agent should use sequential-thinking for simple problems and sequential-inquiry for complex ones" — who makes that routing decision? On what signal?

### Lens 5: essentialist

Apply the 3-gate elimination loop: Exist → Surface → Contract.

**G1 — Exist (deletion test):**
- Delete sequential-thinking. Does any behavior vanish that sequential-inquiry cannot replace? (sequential-inquiry without delegation requests is functionally equivalent.)
- Delete sequential-inquiry. Does any behavior vanish that sequential-thinking + manual agent orchestration cannot replace?
- Delete each delegate template. Does the methodology it provides already exist in the target skill's own templates?

**G2 — Surface (≤7 public items):**
- sequential-thinking: 2 templates + manifest = 3 items. Pass.
- sequential-inquiry: 5 templates + manifest = 6 items. Pass, but at the threshold.
- Can any delegate templates be merged? (They share identical contracts — `invoked: boolean, skill: string, result: object`.)

**G3 — Contract (abstraction trace):**
- The delegate templates are pass-through abstractions: they receive `delegation_requests` and return `{invoked, skill, result}`. The actual methodology (FINER+PICO, MCDA, diagnosis) is executed by the LLM within the template prompt, not by the referenced skill's flowdef. Is this a pass-through that should be replaced with direct invocation?

## Known Critical Issue: rJoule Economics

The session agent set rJoule caps at values ($28,000-$48,000 equivalent) that are economically absurd for a thinking prompt. The flailing correction attempts (48k → 32k → 28k → 5, cost_per_token 0.25 → 1.0 → 0.01 → 0.00002) demonstrate fundamental confusion about the gas↔rJoule relationship.

**Ground truth (from user):** 250,000 gas = 1 rJoule = $1.

**Questions the review must answer:**
- What is the correct `rjoule.cap` for sequential-thinking (currently 24,000)?
- What is the correct `rjoule.cap` and `cost_per_token` for sequential-inquiry?
- Are the existing skill manifests (diagnose: 14k, essentialist: 16k, superforecasting: 32k) also inflated, or is there a system convention that makes these values correct and the session agent's panic unwarranted?
- Derive the formula: given a 120,000 gas cap, 3 max iterations, and 6 steps per cycle, what rJoule cap maintains proportionality with the gas constraint?

## Pending: Conceptual Merge Analysis

User requested audit of semantically similar skills for potential merging. Flagged candidates: `review` and `diagnose`. This analysis was not started. The grill-me and essentialist lenses above partially cover this, but a systematic similarity analysis across the full skill corpus is a separate task.

## What NOT to Touch

- The engine templates' core reasoning protocols (branching, revision, hypothesis/verify) — these are structurally correct
- The PDCA loop architecture (3-step and 6-step) — the step count and loop_target values are correct
- The delegate templates' methodology descriptions (FINER+PICO, MCDA pipeline, diagnosis loop) — these are accurate to their source skills
