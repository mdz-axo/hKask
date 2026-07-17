---
title: "Companies MCP Semantic Graph Audit"
audience: [developers, architects]
last_updated: 2026-07-17
version: "0.31.0"
status: "Active"
domain: "Companies"
mds_categories: [domain, composition, lifecycle, curation]
last-verified-against: "fae4d94"
---

# Companies MCP Semantic Graph Audit

A code-review snapshot of the internal module dependency graph of `mcp-servers/hkask-mcp-companies`. The authoritative interfaces are the server's tool definitions in `src/tools/`; this inventory classifies the binding strength of each module edge, evaluates graph health through four lenses (pragmatic-cybernetics, essentialist, grill-me, pragmatic-semantics), detects structural pathologies, and reports a normalized graph-health verdict.

This audit follows the [semantic-graph-audit](../../.agents/skills/semantic-graph-audit/SKILL.md) skill: classify → analyze → detect → report.

## 1. Graph under audit

| Node | Kind | Source |
|------|------|--------|
| `lib` | Server composition, forecast store, learning state, `combined_router` | `src/lib.rs` |
| `tools/financial_data` | 8 financial-data tool handlers | `src/tools/financial_data.rs` |
| `tools/analysis` | 5 fundamental-analysis and research handlers (moat, scorecard, working capital, screener, research) | `src/tools/analysis.rs` |
| `tools/valuation` | 8 valuation and forecasting handlers (comps, sensitivity, Monte Carlo, calibrate, forecast get/list/record, result feedback) | `src/tools/valuation.rs` |
| `tools/portfolio` | 13 ledger, notes, and files handlers | `src/tools/portfolio.rs` |
| `tools/analytics` | 5 portfolio analytics and DCF handlers (attribution, characteristics, DCF, reverse DCF, scenario) | `src/tools/analytics.rs` |
| `tools/economic_profit` | 1 economic-profit handler | `src/tools/economic_profit.rs` |
| `tools/expectations` | 1 expectations-gap handler | `src/tools/expectations.rs` |
| `providers` | FMP/EODHD routing and normalization | `src/providers.rs` |
| `financial_model` | Two-stage projection model | `src/financial_model.rs` |
| `economic_profit` | Economic-profit / residual-income model | `src/economic_profit.rs` |
| `scenarios` | Fixed growth × margin scenario matrix | `src/scenarios.rs` |
| `superforecast` | Fermi calibration and Brier scoring | `src/superforecast.rs` |
| `analysis` | Moat, scorecard, working-capital math | `src/analysis.rs` |
| `research` | Exa, Tavily, Brave retrieval and claim classification | `src/research.rs` |
| `screener` | FMP natural-language screener | `src/screener.rs` |
| `portfolio` | SQLite ledger, notes, files, durable forecasts | `src/portfolio.rs` |
| `data_quality` | Temporal snapshots, staleness, signal quality | `src/data_quality.rs` |
| `fibo` | FIBO concept identifiers (leaf) | `src/fibo.rs` |
| `types` | MCP request schemas (leaf) | `src/types.rs` |

Edges are directed "depends on / uses". An edge `A → B` means A's compilation or behavior requires B.

## 2. Edge classification (pragmatic-semantics force)

Constraint force scale: Prohibition > Guardrail > Guideline > Evidence > Hypothesis. Provenance: spec, implementation, observation, inference, unknown.

Edges below are verified by grep of module-qualified paths (`module::`) in each source file. The `use crate::*` glob in every tool file hides additional re-export consumption; the cycle finding in §3.1 is partly a consequence of this opacity. Edges marked *inference* are not directly path-qualified but are implied by the tool's behavior and the `use crate::*` surface.

| Edge | Force | Provenance | Rationale |
|------|-------|------------|-----------|
| `lib → financial_model` | Prohibition | implementation | `StoredForecast` embeds `ProjectedModel` and `ProjectionAssumptions`; compiler-enforced struct composition |
| `lib → providers` | Prohibition | implementation | `fetch` calls `providers::companies_get`; `run` re-exports `Provider` |
| `lib → portfolio` | Prohibition | implementation | `lib` re-exports `PersistedForecast`, `PortfolioError`, `PortfolioManager`, `TxType` and wraps forecast persistence in `save_forecast` |
| `lib → data_quality` | Guardrail | implementation | `LearningState` stores `TemporalSnapshot`; coupling is to one struct, not the full module |
| `lib → superforecast` | Guardrail | implementation | `run` constructs `FermiDefaults::from_env`; narrow seam |
| `lib → analysis` | Guardrail | implementation | `lib.rs` path-qualifies `analysis::`; shared fundamental calculations |
| `lib → types` | Prohibition | implementation | `use types::*` glob; request schemas flow through every tool |
| `lib → tools` | Guardrail | implementation | `pub mod tools`; the dispatch surface, loosable only by a full restructure |
| `tools/* → lib` | Evidence | implementation | `use crate::*` in every tool file; factual re-export coupling |
| `tools/financial_data → providers` | Evidence | implementation | `symbol_search` path-qualifies `providers::`; other tools reach providers via `self.fetch` |
| `tools/analysis → analysis` | Prohibition | implementation | Moat, scorecard, working-capital handlers delegate to `analysis::` |
| `tools/analysis → research` | Guardrail | implementation | `research_search` delegates to `research::` |
| `tools/analysis → screener` | Guardrail | implementation | `company_screener` delegates to `screener::` (FMP-specific) |
| `tools/analysis → fibo` | Evidence | implementation | FIBO anchors on analysis outputs |
| `tools/valuation → financial_model` | Prohibition | implementation | Projection model is the valuation engine |
| `tools/valuation → scenarios` | Guardrail | implementation | `monte_carlo_dcf` / sensitivity delegate to `scenarios::` |
| `tools/valuation → superforecast` | Guardrail | implementation | `calibrate_forecast` uses Fermi/Bayesian calibration |
| `tools/valuation → fibo` | Evidence | implementation | FIBO anchors on valuation outputs |
| `tools/valuation → portfolio` | Guideline | inference | `forecast_*` persist via `self.save_forecast` (lib method) → `PortfolioManager`; mediated by `lib` |
| `tools/analytics → financial_model` | Guardrail | implementation | `dcf_valuation` / `reverse_dcf` / `scenario_analysis` path-qualify `financial_model::` |
| `tools/analytics → scenarios` | Guardrail | implementation | `scenario_analysis` path-qualifies `scenarios::` |
| `tools/analytics → data_quality` | Guardrail | implementation | Signal-quality spans on analytics outputs |
| `tools/analytics → fibo` | Evidence | implementation | FIBO anchors on analytics outputs |
| `tools/analytics → portfolio` | Guideline | inference | Reads ledger via `run_portfolio` (lib helper) and `self.portfolio`; mediated by `lib` |
| `tools/economic_profit → economic_profit` | Prohibition | implementation | `ep_valuation` delegates to the EP model |
| `tools/economic_profit → fibo` | Evidence | implementation | FIBO anchors on EP outputs |
| `tools/economic_profit → financial_model` | Guideline | inference | Shares projection primitives via `use crate::*` |
| `tools/expectations → financial_model` | Guardrail | implementation | Market-implied growth solver path-qualifies `financial_model::` |
| `tools/expectations → research` | Guardrail | implementation | `expectations_gap` path-qualifies `research::` for guidance data |
| `tools/expectations → fibo` | Evidence | implementation | FIBO anchors on expectations outputs |
| `tools/portfolio → portfolio` | Prohibition | implementation | Every ledger/notes/files tool path-qualifies `portfolio::` |
| `tools/portfolio → fibo` | Evidence | implementation | FIBO `PORTFOLIO` anchor on list output |
| `providers → data_quality` | Guardrail | implementation | `emit_provider_cns` and temporal snapshots |
| `financial_model → data_quality` | Guardrail | implementation | Signal-quality on projected line items |
| `financial_model → types` | Evidence | implementation | Schema types |
| `scenarios → financial_model` | Guardrail | implementation | Scenario matrix drives the projection |
| `superforecast → scenarios` | Guardrail | implementation | Fermi calibration reuses the scenario matrix |
| `portfolio → types` | Evidence | implementation | Transaction and forecast types |
| `types → economic_profit` | Evidence | implementation | `types.rs` path-qualifies `economic_profit::` (shared EP types) |
| `fibo → (none)` | — | — | Leaf module |
| `data_quality → (none)` | — | — | Leaf module |
| `analysis → (none)` | — | — | Leaf module (reached via `tools/analysis`) |
| `research → (none)` | — | — | Leaf module (reached via `tools/analysis`, `tools/expectations`) |
| `screener → (none)` | — | — | Leaf module (reached via `tools/analysis`) |
| `economic_profit → (none)` | — | — | Leaf module (reached via `tools/economic_profit`, `types`) |

**Over-constraint flag:** none. No Prohibition edges are used where Guideline would suffice. The Prohibition edges are genuine compiler-enforced struct composition or re-export couplings.

## 3. Four-lens analysis

### 3.1 Pragmatic-cybernetics

- **Cycle detected:** `lib ↔ tools/*` mediated by `pub mod tools` (lib → tools) and `use crate::*` (tools → lib). Rust permits module-level mutual reference, so this is not a build error. Polarity: negative-feedback (tools depend on lib's re-exports; lib owns the tool router). Delay: zero (compile-time). Gain: high (every tool transitively pulls lib's entire re-export surface). Closure: closed at the crate boundary. Fidelity: low — the cycle obscures which re-exports each tool actually consumes.
- **Ashby requisite variety:** the `LearningState` regulator (Beta posterior + temporal staleness) models provider reliability across two providers. The environment variety (provider outage modes, stale data, normalization gaps) is larger than the two-state model captures; the flaky override is a coarse control. Requisite variety is partially satisfied.
- **Good Regulator:** `LearningState` does model the system it regulates (provider success/failure). The regulation is of the right kind, but the model is a first-order Beta posterior without regime-change detection.

### 3.2 Essentialist

- **Exist (deletion test):** every domain module (`analysis`, `financial_model`, `economic_profit`, `scenarios`, `superforecast`, `research`, `screener`, `portfolio`, `data_quality`, `fibo`) survives deletion — removing any one re-introduces its complexity into the tool handlers that call it. `fibo` is the weakest survivor (a constant map); its deletion would scatter FIBO strings across modules. `data_quality` is borderline; it centralizes staleness and signal-quality logic that would otherwise duplicate.
- **Surface (fan-out):** three tool modules sit at fan-out 6 (4 domain modules + `lib` + `types`): `tools/analysis` (`analysis`, `research`, `screener`, `fibo`), `tools/valuation` (`financial_model`, `scenarios`, `superforecast`, `fibo`), and `tools/analytics` (`financial_model`, `scenarios`, `data_quality`, `fibo`). This is at the essentialist surface threshold (read as ≤7 dependencies per module). Each is the natural integration point for its domain — acceptable, but it marks the three modules that would benefit most from narrowing the `use crate::*` glob.
- **Contract (pass-through):** `lib` re-exports `PersistedForecast`, `PortfolioError`, `PortfolioManager`, `TxType` from `portfolio`. This is a pass-through abstraction: `tools/valuation` and `tools/portfolio` could depend on `portfolio` directly, bypassing `lib`. The re-export is a convenience surface, not a deep module. Flagged as a mild pass-through.

### 3.3 Grill-me (5-level probe)

| Knowledge area | Recall | Mechanism | Rationale | Edge cases | Synthesis | Gap |
|----------------|--------|-----------|----------|------------|-----------|-----|
| Provider routing | ✓ | ✓ | ✓ | partial (no regime-change) | ✓ | Staleness threshold (90d) is a constant, not configurable |
| Forecast persistence | ✓ | ✓ | ✓ | ✓ | ✓ | `revision_of` chain has no depth limit documented |
| Learning loop | ✓ | ✓ | ✓ | ✓ (both-flaky tested) | ✓ | Beta prior has no cold-start bias documented |
| FIBO anchoring | ✓ | partial | partial | gap | partial | Compact-string vs JSON-LD boundary is stated but not enforced by a type |
| Portfolio owner isolation | ✓ | ✓ | ✓ | ✓ | ✓ | — |

The strongest gaps are in FIBO anchoring (no type-level enforcement of the compact-vs-context boundary) and in configurability of the staleness threshold.

### 3.4 Pragmatic-semantics

- **Force coherence:** Prohibition edges are structural (struct composition, re-exports). Guardrail edges are behavioral seams (engine delegation, persistence). Guideline edges are inferential (signal-quality spans, FIBO anchoring). No force contradictions detected.
- **OT conflict ranking:** no two edges of different force bind the same pair in conflicting directions.
- **Unanchored Hypothesis edges:** none. All edges trace to implementation source.

## 4. Structural pathology detection

| Pathology | Location | Severity | Notes |
|-----------|----------|----------|-------|
| Cycle | `lib ↔ tools/*` (`pub mod tools` + `use crate::*`) | medium | Permitted by Rust; obscures re-export consumption. Not a Prohibition cycle (would be critical). |
| Fan-out anomaly | `tools/analysis`, `tools/valuation`, `tools/analytics` (fan-out 6 each) | low | At the essentialist threshold; the three integration-point modules |
| Pass-through | `lib` re-exports `portfolio` symbols to `tools/valuation` and `tools/portfolio` | low | Convenience surface; tools could depend on `portfolio` directly |
| Broad coupling | `use crate::*` glob in all 7 tool files | low | Increases the `lib` fan-in surface; a targeted `use crate::{fetch, validate_symbol, ...}` would narrow it |
| Gap | `LearningState` staleness threshold is a non-configurable constant (90 days) | low | A cybernetic variety deficit, not a graph defect |
| Gap | FIBO compact-string boundary is stated in docs but not enforced by a type | low | Documentation-vs-type gap, not a graph defect |

No redundancies, orphans, or fan-in anomalies detected. `fibo` and `types` are healthy leaves. Every module has at least one consumer.

**Most critical issue:** the `lib ↔ tools/*` cycle (medium). It is not a build blocker, but it indicates `lib.rs` is simultaneously the dispatch owner, the forecast store, and the re-export hub — a deep-module concern (one module, three responsibilities).

## 5. Graph-health convergence metric

Starting at 0.0:

| Penalty | Weight | Contribution |
|---------|--------|--------------|
| Medium cycle (lib ↔ tools) | 0.12 | 0.12 |
| Low fan-out anomaly (valuation) | 0.03 | 0.03 |
| Low pass-through (lib re-exports) | 0.03 | 0.03 |
| Low broad coupling (`use crate::*`) | 0.03 | 0.03 |
| Variety deficit (staleness constant) | 0.02 | 0.02 |
| Documentation-vs-type gap (FIBO) | 0.02 | 0.02 |
| Good Regulator partial (no regime-change) | 0.02 | 0.02 |
| Prohibition cycles | 0.00 | 0.00 |
| Unanchored Hypothesis edges | 0.00 | 0.00 |
| Surviving deletion candidates | 0.00 | 0.00 |

**Convergence metric: 0.27**

Verdict bands: ≤0.15 healthy, 0.16–0.25 viable_with_issues, 0.26–0.50 degraded, >0.50 unsound.

## 6. Verdict

**Graph health: degraded (0.27).** The graph is sound — no orphans, no Prohibition cycles, every module earns its place, and the over-constraint check is clean. The single degraded signal is the `lib ↔ tools/*` cycle driven by `use crate::*` glob imports, compounded by `lib.rs` holding three responsibilities (dispatch, forecast store, re-export hub). The cybernetic and FIBO gaps are minor and do not threaten viability.

### Top issues, ranked by severity

1. **`lib ↔ tools/*` cycle (medium).** `pub mod tools` in `lib.rs` and `use crate::*` in every tool file create a mutual dependency that obscures each tool's actual re-export consumption.
2. **`lib.rs` triple responsibility (medium).** Dispatch ownership, forecast store, and re-export hub co-locate in one module — a deep-module violation (Ousterhout's interface-depth criterion).
3. **`use crate::*` broad coupling (low).** Seven tool files pull lib's entire re-export surface; targeted imports would narrow the seam and make the cycle's actual edges visible.

### Most material lens finding

The essentialist lens is the most material: the `lib ↔ tools` cycle and the `lib` re-export pass-through are both deep-module concerns, not correctness defects. The graph is structurally healthy; the degradation is a design-depth signal.

### Recommended actions, ordered by constraint force

| Priority | Action | Force |
|----------|--------|-------|
| 1 | Split `lib.rs` into a `dispatch` module (router + `execute_tool` wrappers) and a `forecast_store` module; keep `lib.rs` as the thin crate root | Guardrail |
| 2 | Replace `use crate::*` in tool files with targeted `use crate::{fetch, validate_symbol, record_fetch_outcome, ...}` lists | Guideline |
| 3 | Have `tools/valuation` and `tools/portfolio` depend on `portfolio` directly instead of through `lib` re-exports | Guideline |
| 4 | Make the staleness threshold (`CHRONIC_STALENESS_DAYS`) configurable via env or a `LearningState` constructor parameter | Guideline |
| 5 | Add a type-level boundary for FIBO compact strings vs JSON-LD contexts (e.g. a `FiboId` newtype) | Guideline |
| 6 | Document the `revision_of` chain depth expectation and the Beta prior cold-start bias | Evidence |

### Blockers to a healthy verdict

- Resolve the `lib ↔ tools/*` cycle (split `lib.rs` and narrow tool imports). This alone moves the metric from 0.27 to ~0.12 (healthy).
- No other blockers. The remaining findings are low-severity improvements.

## Cross-links

- [Companies MCP Server Reference](../reference/mcp-servers/companies.md) — full tool catalog and architecture
- [Companies User Guide](../how-to/companies-mcp.md) — task-oriented procedures
- [Tool Routing and Dispatch Flow](../diagrams/flowchart-companies-tool-routing.md) — DIAG-RF-004 dispatch diagram
- [Companies MCP Code Review](companies-mcp-code-review-2026-07-15.md) — adversarial code review of the same server
- [Scenarios Semantic Graph Audit](scenarios-semantic-graph-audit.md) — companion audit of the scenarios server