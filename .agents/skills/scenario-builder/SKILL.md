---
name: scenario-builder
visibility: public
description: Scenario planning methodology following Schwartz's framework. Refines focal questions, maps key forces and macro-level driving forces through STEEP analysis, generates divergent 2x2 scenario narratives, and derives implications with early-warning indicators. Use when exploring strategic uncertainty, preparing for multiple futures, or stress-testing decisions against alternative worlds.
activation: "build scenarios"
---

# Scenario Builder

Multi-stage scenario planning following the Schwartz Method — the methodology developed at Royal Dutch Shell that anticipated the 1973 oil crisis and transformed strategic planning from prediction to preparation. Generates multiple divergent, internally consistent futures and derives strategies that work across them.

## Why Scenario Building?

Prediction is brittle. When you predict one future, you prepare for one future — and when a different future arrives, you're unprepared. Scenario building inverts this: instead of asking "what will happen?", it asks "what could happen, and what would we do in each case?"

The key output is not a prediction. It's **robust strategies** — actions that work across all plausible futures — and **early-warning indicators** — signals that tell you which future is unfolding so you can switch to contingent strategies before it's too late.

Shell used scenario planning to navigate the 1973 oil embargo, the 1986 price collapse, and the collapse of the Soviet Union — not by predicting any of them, but by having strategies ready for worlds where they could happen.

## The Stages

```
┌─────────────────────────────────────────────────────────────┐
│ STAGE 1: FOCAL QUESTION                                      │
│                                                              │
│ Refine the strategic question. It must be:                   │
│  • Decision-relevant — tied to a real choice                 │
│  • Time-bounded — has a planning horizon (e.g., 5 years)     │
│  • Scope-bounded — not everything, everywhere                │
│                                                              │
│ Bad:  "Will AI take over?"                                   │
│ Good: "How should we position our AI strategy over the       │
│        next 5 years given regulatory uncertainty?"           │
│                                                              │
│ Output: refined_question, time_horizon, scope, current_state │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 2: KEY FORCES (Micro-Level)                            │
│                                                              │
│ Identify proximate factors shaping the focal domain:         │
│  • Market dynamics and competitor actions                    │
│  • Demand shifts and customer behavior                       │
│  • Regulatory changes and policy signals                     │
│  • Technology adoption curves                                │
│                                                              │
│ Cluster identified forces into thematic groups.              │
│                                                              │
│ Output: key_forces[], thematic_clusters[]                    │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 3: DRIVING FORCES (Macro-Level — STEEP)                │
│                                                              │
│ Map macro forces against importance × uncertainty:           │
│  • Society — demographics, values, cultural shifts           │
│  • Technology — breakthroughs, adoption, disruption          │
│  • Economy — growth, inflation, trade, investment            │
│  • Environment — climate, resources, sustainability          │
│  • Politics — regulation, geopolitics, governance            │
│                                                              │
│ Forces that are HIGH importance AND HIGH uncertainty         │
│ become your scenario axes.                                   │
│                                                              │
│ Output: driving_forces[], importance_uncertainty_matrix       │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 4: 2×2 SCENARIO MATRIX                                 │
│                                                              │
│ Select TWO critical uncertainties as axes.                   │
│ Cross them to create FOUR scenario quadrants.                │
│                                                              │
│ Example axes for AI regulation:                              │
│   X-axis: Regulation — strict ↔ laissez-faire                │
│   Y-axis: Adoption — rapid uptake ↔ slow/blocked             │
│                                                              │
│ Quadrants:                                                   │
│   • Regulated + Fast = "Managed Acceleration"                │
│   • Regulated + Slow = "Fortress AI"                         │
│   • Laissez-faire + Fast = "Wild West"                       │
│   • Laissez-faire + Slow = "Winter"                          │
│                                                              │
│ Each quadrant gets an internally consistent narrative.       │
│                                                              │
│ Output: scenario_axes, scenario_narratives[4]                │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 5: INDEPENDENT QUALITY GATE                            │
│                                                              │
│ An independent evaluator (separate from the narrative        │
│ generator) assesses the scenario set across three dimensions:│
│                                                              │
│  • Divergence (0–1) — are all 4 quadrants genuinely          │
│    distinct? Flags parametric variation (dial-level          │
│    differences masquerading as different worlds).            │
│  • Consistency (0–1) — is each narrative internally          │
│    coherent with no contradictory elements?                  │
│  • Coverage (0–1) — do the 4 scenarios collectively span    │
│    the full uncertainty space? No major blind spots?         │
│                                                              │
│ Gate passes when all three scores ≥ 0.60.                   │
│ If the gate fails, return to Stage 4 and refine axes or      │
│ narratives before proceeding.                                │
│                                                              │
│ Output: gate_pass, divergence_score, consistency_score,      │
│         coverage_score, parametric_variation_flag,           │
│         gate_findings[]                                      │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ STAGE 6: IMPLICATIONS & INDICATORS                           │
│                                                              │
│ For each scenario:                                           │
│  • What would we need to do? (contingent strategies)         │
│  • What holds across all four? (robust strategies)           │
│  • What early signals would tell us this scenario            │
│    is unfolding? (tripwires/indicators)                      │
│                                                              │
│ Output: robust_strategies[], contingent_strategies[],        │
│         early_warning_indicators[]                           │
│                                                              │
│ Each strategy classified by constraint force:                │
│  • Prohibition — must-do in all scenarios (existential)      │
│  • Guardrail — should-do, overridable with rationale         │
│  • Guideline — preferred approach, advisory                  │
│                                                              │
│ Note: Constraint-force labels (Prohibition/Guardrail/        │
│ Guideline) follow the pragmatic-semantics methodology —      │
│ OUGHT-classified by constraint force, not IS-declarative.    │
│ This ensures strategies carry enforceable prescriptive       │
│ weight consistent with the Magna Carta principle hierarchy.  │
└─────────────────────────────────────────────────────────────┘
```

## Robust vs. Contingent Strategies

| Type | Definition | Example |
|------|-----------|---------|
| **Robust strategy** | Works across ALL scenarios | "Invest in AI literacy regardless of regulatory outcome" |
| **Contingent strategy** | Works in SPECIFIC scenarios | "If strict regulation arrives, pivot to compliance consulting" |

Robust strategies are your no-regret moves. Contingent strategies are your hedges — prepare them, trigger them when early-warning indicators fire.

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "scenario plan this" / "build scenarios" / "explore futures" | Full 7-stage pipeline (includes quality gate) |
| "what are the driving forces?" / "STEEP analysis" | Stage 3 only — macro force mapping |
| "build a 2x2 matrix" / "scenario axes" | Stage 4 only — scenario construction |
| "verify scenarios" / "are these divergent?" | Stage 5 only — independent quality gate |
| "what are the early signals?" / "tripwires" | Stage 6 only — implications and indicators |
| "what's robust across scenarios?" | Robust strategy extraction |

## Do NOT Activate For

- Short-term tactical decisions with a horizon of < 6 months
- Fully deterministic domains where uncertainty is negligible
- Decisions where the cost of scenario planning exceeds the cost of being wrong
- When a simple SWOT or PEST analysis suffices

## Downstream Skills

These skills are not composed into the scenario-builder pipeline but are natural next steps after scenario construction. Activate each at the indicated stage.

- **Superforecasting** (activate after Stage 6): Assign calibrated probabilities to each of the 4 scenarios using Tetlock's methodology. Scenario builder explores the futures; superforecasting quantifies their likelihood.
- **MCDA** (activate after Stage 6): Evaluate robust and contingent strategies across all 4 scenarios. Which strategy scores highest in the most futures? Use multi-criteria decision analysis to rank alternatives.
- **Decision-journal** (activate on convergence): When the scenario planning cycle converges, offer the user activation of `decision-journal` to record the strategic decision with full context — which scenarios were considered, which strategies were selected, and the rationale. Schedule a revisit when early-warning indicators fire. This closes the loop from exploration to committed action.

## Registry Templates

| Template | Type | Stage | Purpose |
|----------|------|-------|---------|
| `focal-question.j2` | KnowAct | 1 | Refine and bound the focal question |
| `key-forces.j2` | KnowAct | 2 | Identify micro-level proximate forces |
| `driving-forces.j2` | KnowAct | 3 | STEEP macro force mapping with importance-uncertainty matrix |
| `axes-and-narratives.j2` | KnowAct | 4 | Construct 2×2 matrix with scenario narratives |
| `scenario-quality-gate.j2` | KnowAct | 5 | Independent quality gate — divergence, consistency, coverage |
| `implications-indicators.j2` | KnowAct | 6 | Derive strategies and early-warning indicators |
| `scenario-convergence-check.j2` | KnowAct | 7 | Compute convergence from quality gate + heuristic checks |

## Quick Reference

1. **Focus** — refine the question: decision-relevant, time-bounded, scope-bounded
2. **Key forces** — what proximate factors shape this domain?
3. **Driving forces** — STEEP macro forces mapped by importance × uncertainty
4. **2×2 matrix** — cross two critical uncertainties → four scenarios with narratives
5. **Quality gate** — independent evaluation of divergence, consistency, and coverage
6. **Implications** — robust strategies (all scenarios), contingent (specific), tripwires (early signals)
7. **Classify** — each strategy carries a constraint-force label (Prohibition/Guardrail/Guideline), applied using pragmatic-semantics methodology
8. **Converge** — heuristic divergence check + quality gate scores → normalized metric. On convergence, offer decision-journal to record the strategic decision.

*"The goal of scenario planning is not to predict the future but to make better decisions in the face of uncertainty."* — Peter Schwartz, *The Art of the Long View*


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/scenario-planning.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = scenarios are sufficiently divergent/coherent and implications+indicators are actionable

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
