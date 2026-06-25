# hkask-mcp-companies — Continuation: Wire Financial Model + Rebuild Decomposition

**Date:** 2026-06-24 | **Tests:** 99 passed, 0 failed, 0 warnings | **Tools:** 34 (27 original + 7 new)

## 1. Session Context

This session substantially upgraded the hkask-mcp-companies MCP server. We fixed 6 bugs, added 7 new tools (DCF valuation, reverse DCF, scenario analysis, calibrated superforecast, forecast record, result feedback), built a kanban-style learning loop for provider routing, anchored all financial concepts to FIBO ontology, removed 7 bureaucrat selector templates + 4 redundant FlowDef manifests, and constructed an 11-line-item financial statement projection engine (`financial_model.rs`). The final task — wiring `financial_model.rs` into the DCF tools and rebuilding `forecast_record` decomposition on it — was not completed. This handoff covers exactly those two remaining items.

## 2. What Was Done

### Bugs fixed (6)
- NaN propagation in `expectations_gap` output JSON (now uses `Option<f64>` → `null`)
- Year misalignment in `management_scorecard` (ROIC and invested capital now aligned by calendar year)
- IRR Newton's method without convergence detection (now returns `irr_converged: bool`)
- TOCTOU race in `ledger_import` portfolio auto-create (`INSERT OR IGNORE`)
- Missing CNS experience recording for 9 data-fetch/analysis tools (now 15/27 tools record narrative detail)
- CSV export corrupting notes containing semicolons (now uses proper CSV quoting with double-quote escaping)

### New tools built (7)
- `dcf_valuation` — Two-stage Gordon Growth DCF: stage 1 (1-3yr explicit), stage 2 (2-7yr convergence), terminal perpetuity or exit multiple, annual/quarterly. Uses the OLD simplified model in `dcf.rs` (revenue × margin only).
- `reverse_dcf` — Mauboussin expectations investing: binary search (-50% to +100%) solves for implied growth rate.
- `scenario_analysis` — Schwartz 2×2 matrix: growth × profitability → Bull/Land Grab/Cash Cow/Bear. Runs DCF per scenario.
- `calibrate_forecast` — Tetlock GJP pipeline: Fermi decomposition → outside/inside view calibration → probability-weighted scenario distribution. Configurable via `HKASK_FERMI_DEFAULTS` env var, per-call estimates, or Fermi sub-question overrides.
- `forecast_record` — Records actual outcome (multiple, price change) at a horizon (3mo/6mo/1yr/2yr/3yr). Computes Brier scores. Stores in daemon as `forecast_outcome:{symbol}` triples. CURRENTLY uses simplified gap narrative — needs 11-line-item decomposition.
- `result_feedback` — 1–5 score + optional comments. Feeds `LearningState` for kanban-style provider routing.
- 11-line-item financial model (`financial_model.rs`) — Built and tested (9 tests), but NOT yet wired into the DCF tools.

### Infrastructure built
- `LearningState` — in-memory feedback tracker: `is_flaky()`, `preferred_provider()`, running average. Wired into `providers::companies_get()` via `CompaniesServer::fetch()`.
- `FIBO ontology mapping` (`fibo.rs`) — 25 FIBO concept URIs, `fmp_field_to_fibo()`, `fibo_weighted_average()`. Wired into `portfolio_characteristics` output.
- Bureaucrat templates removed — 7 selector `.j2` files + 4 FlowDef manifests + 4 empty selector directories deleted.
- Pre-existing `hkask-mcp` crate breakage fixed (`emit_tool_span_with_caller` missing ontology param, `execute_tool_semantic` removal).

### Tests
- 99 total: analysis.rs (28), providers.rs (9), portfolio.rs (15), dcf.rs (9), financial_model.rs (9), superforecast.rs (13), fibo.rs (5), lib.rs (11)
- 26 fuzz targets in `fuzz/fuzz_targets/companies_fuzz.rs`
- Zero compiler warnings

## 3. What Remains

### HIGH: Wire `financial_model.rs` into `dcf_valuation`

**Current state:** `dcf_valuation` in `lib.rs` uses the old `dcf::run_dcf()` which takes a simplified `dcf::CompanyFundamentals` (just `ttm_revenue`, `fcf_margin`, `hist_revenue_growth`, `shares_outstanding`). It projects FCF as `revenue * fcf_margin` — no income statement, no balance sheet, no net debt adjustment, no NWC tracking.

**Target state:** `dcf_valuation` should:
1. Pull historical data from API responses (income_statement, balance_sheet, cash_flow_statement, key_metrics, profile) into a `financial_model::HistoricalSnapshot`
2. Build `financial_model::ProjectionAssumptions` from history (calibrating ratios: gross_margin, da_to_revenue, capex_to_revenue, nwc_to_revenue, tax_rate, revenue_cagr)
3. Allow user overrides for any assumption (growth, margin, discount rate, terminal growth, stage years)
4. Run `financial_model::project_model()` 
5. Return the full `ProjectedModel` output: every period with all 13 line items (revenue, cogs, gross_profit, da, ebit, tax, nopat, capex, change_in_nwc, fcf, discount_factor, present_value) + terminal value + enterprise → equity bridge + per-share intrinsic

**Files to modify:**
- `mcp-servers/hkask-mcp-companies/src/lib.rs` — `dcf_valuation` tool method (~lines 1885-2050)
- `mcp-servers/hkask-mcp-companies/src/types.rs` — `DcfValuationRequest` may need additional fields for assumption overrides
- `mcp-servers/hkask-mcp-companies/src/dcf.rs` — may remain as-is (legacy) or be deprecated

**Data extraction from API:**
The `HistoricalSnapshot` needs these fields extracted from FMP/EODHD JSON:
- `revenue` ← `income_statement[].revenue` (multiple years, sorted ascending)
- `cogs` ← `income_statement[].costOfRevenue`
- `da` ← `income_statement[].depreciationAndAmortization`
- `capex` ← `cash_flow_statement[].capitalExpenditure` (negative in FMP)
- `total_assets` ← `balance_sheet[].totalAssets`
- `current_assets` ← `balance_sheet[].totalCurrentAssets`
- `current_liabilities` ← `balance_sheet[].totalCurrentLiabilities`
- `cash` ← `balance_sheet[].cashAndCashEquivalents` or `balance_sheet[].cashAndShortTermInvestments`
- `long_term_debt` ← `balance_sheet[].longTermDebt`
- `equity` ← `balance_sheet[].totalStockholdersEquity`
- `shares_outstanding` ← `key_metrics[].weightedAverageShsOut` or `profile[].sharesOutstanding`
- `tax_rate` ← compute from `income_statement[].incomeTaxExpense / incomeBeforeTax`

**Key: sort all arrays by calendarYear ascending** before constructing the HistoricalSnapshot. The FMP API returns newest-first; EODHD normalization also sorts descending. The HistoricalSnapshot expects ascending (oldest first) for CAGR computation.

### HIGH: Rebuild `forecast_record` decomposition on 11-line-item model

**Current state:** `forecast_record` uses a binary gap narrative: "multiple_and_return_diverged" / "multiple_drove_gap" / "return_drove_gap" / "forecast_accurate". This is a placeholder — it doesn't attribute the gap to specific line items.

**Target state:** After wiring `financial_model.rs` into `dcf_valuation`, `forecast_record` should:
1. Accept a `forecast_id` (UUID generated by `calibrate_forecast` or `dcf_valuation` at forecast time) so the record can look up the original projections
2. Pull the original projected model from daemon storage (stored at forecast time as `forecast_model:{forecast_id}`)
3. Compare each projected line item to the actual outcome
4. Decompose the total return gap into:
   - Revenue growth gap (actual vs projected)
   - Gross margin gap
   - D&A / Capex efficiency gap
   - NWC intensity gap
   - Multiple expansion/contribution gap
   - Net debt / capital structure gap
5. Store the full decomposition alongside the Brier scores in the daemon

**This requires storing the projected model at forecast time.** When `dcf_valuation` or `calibrate_forecast` runs, the full `ProjectedModel` (or at minimum the `ProjectionAssumptions` + `HistoricalSnapshot` sufficient to reconstruct it) must be stored as a daemon triple with a unique `forecast_id`. `forecast_record` references that ID.

**Files to modify:**
- `mcp-servers/hkask-mcp-companies/src/lib.rs` — `dcf_valuation` and `calibrate_forecast` to emit forecast_id and store model
- `mcp-servers/hkask-mcp-companies/src/lib.rs` — `forecast_record` to accept forecast_id and run decomposition
- `mcp-servers/hkask-mcp-companies/src/types.rs` — `ForecastRecordRequest` to add `forecast_id` field

### MEDIUM: Unify or deprecate old `dcf.rs`

After wiring `financial_model.rs`, the old `dcf.rs` (simplified revenue×margin model) is redundant. Options:
- Delete it and have `reverse_dcf` and `scenario_analysis` use `financial_model` instead
- Or keep it as a "quick" path and add a `model` parameter (`"simple"` vs `"full"`)

The old model is used by: `dcf_valuation`, `reverse_dcf`, `scenario_analysis`. All three should migrate.

### Design questions to resolve (not block execution)

1. **Per-symbol Fermi defaults**: Currently server-level via `HKASK_FERMI_DEFAULTS`. Should users be able to set per-symbol overrides stored in the daemon or portfolio notes?

2. **Continuous Brier scoring**: Currently binary (above/below baseline, within 20% tolerance). A continuous probability distribution over a range of multiples would be more informative but requires the forecaster to specify a distribution, not a point estimate.

3. **Forecast model storage**: Storing the full `ProjectedModel` (10 periods × 13 fields = 130 values) in the daemon as JSON is straightforward. But should it be stored at forecast time or reconstructed from stored assumptions at record time? Storing at forecast time is immutable and verifiable.

## 4. Recommended Skills and Commands

Activate these skills before starting:
- `coding-guidelines` — guardrail, mandatory before code changes
- `rust-expertise` — type-driven design, ownership patterns
- `tdd` — red-green-refactor for the financial model wiring

Commands:
```bash
cargo check -p hkask-mcp-companies          # fast feedback
cargo test -p hkask-mcp-companies           # 99 tests, must stay green
cargo clippy -p hkask-mcp-companies         # zero warnings standard
```

## 5. Key Decisions to Preserve

1. **FIBO anchoring is non-negotiable.** All financial concepts reference FIBO URIs. New fields added to output must include `fibo` annotations via `fmp_field_to_fibo()`.

2. **Kanban-style learning loop, not separate consumer.** Feedback updates `LearningState` in-process. Provider routing reads it synchronously. No async consumer, no polling, no separate process. This was a deliberate simplification.

3. **Gordon Growth terminal growth is capped at 10%.** The perpetuity formula requires `r > g`. A 0.5% safety margin is applied in the terminal value computation (`terminal_g.min(discount_rate - 0.005)`).

4. **DCF growth tapering is linear, not exponential.** Stage 1 tapers from historical growth toward the midpoint of historical and terminal. Stage 2 converges linearly from stage 1 endpoint to terminal. This is simpler than S-curve models but has been verified correct through tests.

5. **Revenue CAGR uses geometric mean (product-of-growths), not arithmetic mean.** The `revenue_cagr()` method in `HistoricalSnapshot` and the standalone `cagr_from_series()` both use `Π(1+g)^(1/n) - 1`.

6. **NWC = current_assets - current_liabilities - cash.** This is net working capital *net of cash*, consistent with standard DCF practice where cash is modeled separately.

7. **Equity value = enterprise value - net debt, where net debt = long_term_debt - cash.** No adjustments for minority interests, pensions, or operating leases in the simplified model.

8. **CSV quoting uses double-quote wrapping with internal quote doubling**, not semicolon substitution. This was the fix for bug #6.

9. **`forecast_record` Brier scoring is binary (above/below, within tolerance), not continuous distribution.** This was a deliberate collapse to avoid the complexity of defining a full probability distribution over a continuous outcome.

10. **Portfolio tools do NOT have narrative daemon recording.** Their state changes are auditable via `portfolio_list`/`ledger_export`. This was an intentional deferral, not an omission.
