# Scenario Forecasting: Methodology

**Diataxis type:** Explanation
**Status:** Current (v0.31.0)
**Related:** `mcp-servers/hkask-mcp-scenarios/README.md` (tool reference), [Superforecasting: Layered Model](superforecasting-layers.md) (architecture)

## Why this document exists

The scenarios MCP server implements three forecasting methodologies — Tetlock's superforecasting, Schwartz's scenario planning, and Chermack's performance-based scenario system — as an integrated pipeline. This document explains how they combine and why, so reviewers can evaluate whether the implementation matches the methodology.

## Three methodologies, one pipeline

### Tetlock — Forecast accuracy

The superforecasting methodology (Tetlock & Gardner, 2015) provides the calibration engine:
- **Triage** — classify questions as clocklike, Goldilocks, or cloudlike
- **Fermi decomposition** — break forecasts into sub-questions with confidence-weighted estimates
- **Outside view** — blend with base rates using a shrinkage estimator
- **Bayesian updating** — revise probabilities as evidence arrives
- **Dragonfly-eye synthesis** — aggregate multiple perspectives with inverse-Brier weighting
- **Brier scoring** — measure forecast accuracy against outcomes
- **Calibration tracking** — detect systematic over/underconfidence

### Schwartz — Scenario imagination

The Art of the Long View (Schwartz, 1991) provides the scenario construction approach:
- **Focal question** — what decision does this inform?
- **Driving forces** — STEEP analysis (Social, Technological, Economic, Environmental, Political)
- **2×2 axis matrix** — two key uncertainties define four scenarios (implemented in the companies server for financial modeling)
- **Implications** — what strategies work across scenarios?

In the scenarios server, Schwartz provides the framing and brainstorming tools (`scenario_frame`, `scenario_frame_document`, `scenario_brainstorm`).

### Chermack — Project assessment

Chermack's Performance-Based Scenario System (2011) provides the evaluation framework:
- **Phase 1: Preparation** — stakeholder engagement, scope clarity
- **Phase 2: Exploration** — driving forces, diversity of views
- **Phase 3: Development** — causal structure, internal consistency
- **Phase 4: Implementation** — strategies applied, early warning indicators
- **Phase 5: Project Assessment** — learning outcomes, calibration evidence

The `scenario_assess` tool evaluates a project across all five phases.

## How they connect

```
Schwartz (framing)     → Tetlock (calibration)    → Chermack (assessment)
scenario_frame         scenario_calibrate         scenario_assess
scenario_brainstorm    scenario_quantify
scenario_build         scenario_update
                       scenario_synthesize
                       scenario_score
                       scenario_calibration
```

The pipeline flows from imagination (Schwartz) through computation (Tetlock) to evaluation (Chermack). The `scenario_full` tool compresses the Tetlock stages into a single call.

## Event-tree model (MAIA)

The scenarios server uses a binomial event-tree model (MAIA methodology):
- Each event is a yes/no question with a deadline
- Events can depend on other events via conditional probability tables
- Marginal probabilities are computed via full joint-table marginalization under parent independence
- The "all events occur" path probability is the product of all-node-occur conditionals

## Cross-links

- [Scenario Forecasting Pipeline Diagram](../reference/mcp-servers/scenarios.md) — tool flow diagram (DIAG-RF-005, inline)
- [Scenarios Adversarial Review](../status/scenarios-adversarial-review.md) — code smell inventory
- [Scenarios ↔ Companies Bridge](../architecture/core/scenarios-companies-bridge.md) — FIBO to Dublin Core translation
- [Superforecasting: Layered Model](superforecasting-layers.md) — three-layer architecture (skill, math, servers)