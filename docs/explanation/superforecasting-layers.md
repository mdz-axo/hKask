# Superforecasting: Layered Model

**Diataxis type:** Explanation
**Status:** Current (v0.31.0)
**Related:** `registry/templates/superforecasting/README.md` (skill pipeline), `crates/hkask-forecast/README.md` (library)

## Why this document exists

"Superforecasting" in hKask appears in four places — a natural-language skill, a
pure-math Rust library, and two domain MCP servers — and they describe the same
Tetlock methodology at different resolutions. Without an explicit layer model,
the surfaces drift: the skill calls its pipeline "8 stages", the library exposes
"4 functions", and the servers add domain logic that looks like more stages. This
document fixes the contract so reviewers and contributors can see which surface
owns what.

## The three layers

```
┌──────────────────────────────────────────────────────────────┐
│  Skill layer  — registry/templates/superforecasting/*.j2     │
│  Natural-language Tetlock pipeline (8 stages + gate +        │
│  convergence). LLM reasoning: triage judgment, hypothesis     │
│  generation, counterfactual analysis, dragonfly synthesis,   │
│  calibration, record, quality gate. PDCA loop + fusion panel.  │
└──────────────────────────────────────────────────────────────┘
                          │  documents the formulas
                          │  it relies on (conformance contract)
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  Canonical-math layer  — crates/hkask-forecast                │
│  Pure-math Tetlock primitives only. No domain types, no NLP,  │
│  no I/O. calibrate_from_fermi, outside_view_adjustment,        │
│  bayesian_update, brier_score, brier_score_multi,             │
│  brier_interpretation. The single source of truth for the     │
│  deterministic core.                                           │
└──────────────────────────────────────────────────────────────┘
                          ▲  consumed via hkask_forecast::*
                          │  (adapters convert domain types)
              ┌───────────┴────────────┐
              ▼                        ▼
┌────────────────────────────┐  ┌──────────────────────────────┐
│ hkask-mcp-scenarios         │  │ hkask-mcp-companies           │
│ Event-tree forecasting,     │  │ FIBO-anchored financial       │
│ ForecastStore journal,      │  │ forecasting, WeightedScenario │
│ calibration curve, triage   │  │ intrinsic-value distribution, │
│ heuristic, certainty tiers. │  │ FermiDefaults env loading.    │
└────────────────────────────┘  └──────────────────────────────┘
```

## What each layer owns

### Skill layer (`registry/templates/superforecasting/`)

Owns the **full Tetlock pipeline as LLM prompts**. This is where the methodology
lives as natural-language reasoning: triage into the Goldilocks zone, Fermi
decomposition into sub-questions, outside-view base-rate anchoring, inside-view
hypothesis generation + counterfactual analysis (delegated to `falsifiability`),
Bayesian evidence update, dragonfly-eye MCDA synthesis, forward-looking
calibration, structured record, independent quality gate, and convergence
check. These stages are not reducible to pure math — "steelman the strongest
opposing argument" is LLM judgment, not a formula.

The skill does **not** call Rust directly. It operates on natural language. But
its stage descriptions must stay consistent with the formulas the canonical-math
layer implements, which is why the conformance contract exists.

### Canonical-math layer (`crates/hkask-forecast/`)

Owns the **deterministic primitives** — the formulas Tetlock's methodology
reduces to: confidence-weighted averaging (Fermi), shrinkage estimation (outside
view), Bayes' theorem (evidence update), and Brier scoring (calibration
tracking). Pure math, no domain types, no NLP, no I/O. This is the single source
of truth for the deterministic core; both MCP servers consume it via
`hkask_forecast::*` and convert their domain types with thin adapters.

### Domain MCP servers (`hkask-mcp-scenarios`, `hkask-mcp-companies`)

Own the **domain applications** that compose the canonical primitives with
domain-specific types and I/O:

- `hkask-mcp-scenarios` — event-tree forecasting, a `ForecastStore` journal with
  compaction, a 10-bin backward-looking `compute_calibration_curve`, a
  `triage_question` heuristic (clocklike/goldilocks/cloudlike), `score_forecast`
  certainty tiers, `compute_marginal_probabilities`, `cross_validate`. Delegates
  the pure-math core to `hkask_forecast`.
- `hkask-mcp-companies` — FIBO-anchored financial forecasting, `WeightedScenario`
  with `intrinsic_per_share`, `distribute_scenario_probabilities`,
  `expected_intrinsic`, `FermiDefaults` env loading, `ForecastOutcome`,
  `forecast_record` tool. Delegates the pure-math core to `hkask_forecast` via
  adapters that convert its local `SubQuestion` to `hkask_forecast::FermiQuestion`.

Domain logic stays here, not in `hkask-forecast`, because it is entangled with
domain types and I/O — moving it up would violate the deep-module discipline
(the canonical library must remain a pure-math leaf with no domain dependencies).

## The conformance contract

The contract lives in `registry/templates/superforecasting/README.md` as the
"Deterministic Primitives" table. It maps each skill stage to the
`hkask-forecast` function that implements its deterministic core, or marks the
stage "natural-language only". The contract is mechanically verified by
`scripts/check-forecast-conformance.sh` (run in CI), which asserts:

1. Every `hkask-forecast` public function is referenced in the contract table
   (no orphan primitives).
2. Every primitive the contract table names actually exists in `hkask-forecast`
   (no dangling references).

This makes the skill ↔ Rust seam auditable at CI time rather than only at code
review, so the two surfaces cannot silently drift.

## Common drift and how this model prevents it

| Drift | How the model catches it |
|-------|---------------------------|
| A MCP server reimplements a canonical primitive instead of delegating. | The conformance test surfaces un-delegated math; the canonical layer is the only place the formulas live. |
| The skill describes a formula the Rust lib no longer implements. | The contract table's named functions are checked to exist; a removed function fails CI. |
| `hkask-forecast` grows a primitive the skill's pipeline doesn't use. | The conformance test flags orphan primitives (every public fn must be in the contract table). |
| Stage names diverge between the skill and the servers. | The contract table is the authoritative stage↔primitive map; servers reference it via doc comments. |
| The fusion summary compresses the pipeline to 4 stages, hiding triage/synthesis/record. | The `FusionSkill::Superforecasting` summary now names the full 8-stage pipeline. |

## Non-goals

- This model does **not** require `hkask-forecast` to implement every Tetlock
  stage. Stages that are inherently LLM judgment (triage, inside-view hypothesis
  generation, synthesis, forward calibration, record, quality gate, convergence)
  have no pure-math core and correctly have no Rust counterpart.
- This model does **not** make the skill call Rust. The skill remains a
  natural-language pipeline; the contract is about *consistency of formulas*,
  not runtime invocation.
- This model does **not** merge the two MCP servers. They serve different
  domains (event trees vs financial valuation) and share only the canonical-math
  layer, which is the correct amount of sharing.