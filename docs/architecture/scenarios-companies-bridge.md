---
title: "Scenarios–Companies Bridge"
audience: [developers, architects]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting"
mds_categories: [domain, composition, lifecycle]
---

# Scenarios–Companies Bridge [^mcp]

## Architecture

The `hkask-mcp-companies` server has domain-specific scenario logic in:
- `scenarios.rs` — Schwartz 2×2 matrix with financial model projections
- `superforecast.rs` — Fermi decomposition + Bayesian update for financial drivers

The `hkask-mcp-scenarios` server now has the **canonical** implementations of:
- Fermi decomposition (`scenario_calibrate`)
- Bayesian updating (`scenario_update`)
- Brier scoring (`scenario_score`)
- Event tree quantification (`scenario_quantify`)
- Dragonfly-eye synthesis (`scenario_synthesize`)
- Calibration tracking (`scenario_calibration`)
- Triage (`scenario_triage`)

## Deduplication Plan [^rust]

### What stays in companies
- `scenarios.rs` → `ScenarioMatrix::growth_x_margin()` — financial-model-specific 2×2
- `scenarios.rs` → `run_scenario_analysis()` — plugs into 11-line-item DCF
- Financial-model-specific Fermi sub-questions (growth rate + margin decomposition)

### What moves to scenarios (canonical)
- `superforecast.rs` → `FermiDefaults`, `calibrate_from_fermi`, `outside_view_adjustment`,
  `bayesian_update`, `brier_score*`, `distribute_scenario_probabilities`

## Bridge Path [^mcp]

### Path 1: companies → scenarios (calibration delegation)

```
CompaniesServer.calibrate_forecast
  → build financial-model-specific Fermi sub-questions
  → serialize to ScenarioEvent format
  → call ScenariosServer.scenario_calibrate via MCP (future: MCP client)
  → receive calibrated probability
  → run financial model projection with calibrated growth/margin
```

### Path 2: scenarios → companies (outcome recording)

```
ScenariosServer.scenario_score (for a company event)
  → compute Brier score
  → record outcome in forecast_store
  → future: notify CompaniesServer of resolved forecast
  → CompaniesServer updates financial model backtest
```

### Path 3: Portfolio aggregation (future)

```
CompaniesServer.portfolio_characteristics
  → for each holding, fetch event tree from ScenariosServer
  → cross-reference event dependencies across holdings
  → compute portfolio concentration on shared assumptions
```

## Shared Engine [^rust]

Both servers now delegate to `hkask-forecast` (crates/hkask-forecast) for canonical implementations of:
- `calibrate_from_fermi` — Fermi decomposition
- `outside_view_adjustment` — Base rate calibration
- `bayesian_update` — Bayesian evidence revision
- `brier_score` / `brier_score_multi` — Brier scoring
- `brier_interpretation` — Human-readable score interpretation

Each server wraps these with its own error types and SubQuestion conversions.

## Implementation Status [^mcp]

**Working today (both directions via shared engine):**
- Companies' `calibrate_from_fermi` → `hkask_forecast::calibrate_from_fermi` (same computation as scenarios)
- Companies' `brier_score` → `hkask_forecast::brier_score` (same scoring)
- `scenario_from_companies` converts companies output into ScenarioEvents for full pipeline

**Working today (companies → scenarios via MCP bridge):**
- Companies output → `scenario_from_companies` → quantify → calibrate → score

**Not yet wired (live MCP call from companies to scenarios):**
- Companies server cannot call `scenario_calibrate` as an MCP tool at runtime
- Requires MCP client protocol in companies server (rmcp client support exists but isn't plumbed)

## Implementation Notes [^mcp]

[^mcp]: Model Context Protocol. (2025). *Specification*. https://modelcontextprotocol.io/specification/2025-06-18
[^rust]: Rust Project. (2026). *The Rust Programming Language*. https://doc.rust-lang.org/book/

The bridge requires MCP client support in the companies server. Currently,
MCP servers in hKask are stdio-based and don't have client protocols to
call other servers. The MCP client protocol exists in `rmcp` but hasn't
been wired into the companies server's tool methods.

When MCP client support is added:
1. `calibrate_forecast` tool delegates Fermi calibration to `scenario_calibrate`
2. `forecast_record` tool records outcomes in `scenario_score`
3. Companies server drops its `superforecast.rs` module
