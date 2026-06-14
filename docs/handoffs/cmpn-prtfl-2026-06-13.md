# Handoff — hKask Companies MCP Server: Rename, EODHD Integration, Portfolio Tracking

**Date:** 2026-06-13
**Session:** Companies crate restructure + dual-provider + portfolio tracking implementation
**Status:** Portfolio tracking Phase 1-2 complete + analysis tools added. 19 tools, 37 tests, all passing.

---

## 1. Session Context

Renamed `hkask-mcp-fmp` to `hkask-mcp-companies`, added EODHD as a second data provider with automatic routing and response normalization, then built portfolio tracking from the spec — transaction ledger, data linkage, and analysis tools (attribution, characteristics, expectations gap). Stripped multiple tools that were infrastructure/internal rather than user-facing. The crate now has 19 user-facing tools across financial data (12) and portfolio management (7), all compiling clean with clippy `-D warnings`.

---

## 2. What Was Done

### Crate Rename & EODHD Integration

- **New crate:** `mcp-servers/hkask-mcp-companies/` (old `hkask-mcp-fmp` deleted)
- **`src/providers.rs`** — Dual-provider abstraction:
  - `Provider` enum (Fmp, Eodhd)
  - `companies_get()` — routes by symbol exchange suffix (`.L`, `.DE`, `.T` → EODHD; plain → FMP), auto-fallback
  - EODHD normalizers reshape nested `/fundamentals/{symbol}` JSON to FMP flat-array format
  - `compute_year_metrics()` derives `grossProfitMargin`, `roic`, `daysOfPayablesOutstanding`, `daysOfSalesOutstanding` from EODHD financial statements
  - `fmp_search_get()` / `eodhd_search_get()` for query-based search
- **All external references updated:** workspace `Cargo.toml`, CLI (`serve.rs`, `builtin_servers.rs`, `bootstrap.rs`), CNS (`table_energy_estimator.rs`), `hkask-types/src/r7.rs`, and 6 doc files

### Portfolio Tracking — Phase 1 (Core Ledger)

- **`src/portfolio.rs`** — `PortfolioManager` with SQLite master DB at `~/.config/hkask/portfolios/master.db`
  - Single DB with `portfolios`, `transactions`, `price_cache`, `security_links` tables
  - `portfolio_name` column on all tables with CASCADE deletes
  - CRUD, transaction add/retrieve/filter, append note, CSV/JSON import/export, validation (positions = buys − sells, cash consistency)
  - Injectable base directory via `with_dir()` for testing
- **8 tests** in portfolio.rs covering CRUD, transactions, filtering, import, export, validation

### Portfolio Tracking — Phase 2 (Data Linkage)

- Price cache table, security links table
- `get_symbols()`, `resolve_symbol()`, `link_security()`, `get_date_range()`, `get_missing_price_dates()`, `store_price()`, `get_prices()`
- Tools: `portfolio_link_security` (later removed as user-facing), `portfolio_refresh_prices` (removed), `portfolio_refresh_fundamentals` (removed)
- These methods kept internally (`#[allow(dead_code)]`) for future use

### Tool Cleanup & Analysis Tools

**Removed (infrastructure/bullshit — not user-facing):**
- `ping` — API health check
- `analyst_estimates` — other people's guesses
- `dcf_analysis` — someone else's model
- `transaction_add` — nobody types transactions one at a time
- `ledger_validate` — made automatic on import
- `portfolio_link_security` — should be automatic
- `portfolio_refresh_prices` — reports pull data on demand
- `portfolio_refresh_fundamentals` — reports pull data on demand

**Added:**
- `portfolio_attribution` — What moved the portfolio. Computes positions at start/end dates, fetches prices, ranks each position by contribution (weight × return)
- `portfolio_characteristics` — Weighted-average fundamentals. Fetches quotes, profiles, key metrics, balance sheets for all holdings, computes weighted averages across valuation, profitability, leverage, growth, composition
- `portfolio_comparison` — Side-by-side: summary stats, shared positions, symbols unique to each

**Rewritten:**
- `expectations_gap` — Gordon Growth Model on 3 matching sets (NetMargin+SalesGrowth+P/S, ROE+BVGrowth+P/B, ROA+AssetGrowth+P/Assets). Default `r=15%`. Uses profitability-growth correlation heuristic (`P/V = profitability / (r - 2g)`, so `g = (r - profitability/valuation_ratio) / 2`). Compares implied growth to trailing 5-year CAGR. Computes gap per set.

**Auto-create:** `ledger_import` now auto-creates the portfolio if it doesn't exist — user just imports a CSV.

### Documentation

- `docs/specifications/portfolio-tracking.md` — Full spec: ledger schema, position math, TWR/IRR, characteristics, attribution, notes/files, storage schema, implementation phases, mathematical reference
- `docs/status/mcp-tools-inventory.md` — Updated companies section with architecture detail
- `AGENTS.md` — Added crate to map, portfolio spec to key docs
- `providers.env.example` — Added `HKASK_FMP_API_KEY` and `HKASK_EODHD_API_KEY`
- `sample_ledger.csv` — 15-row template with US/UK/Japan/Germany examples

### Verification

- `cargo check -p hkask-mcp-companies` ✅
- `cargo clippy -p hkask-mcp-companies -- -D warnings` ✅
- `cargo test -p hkask-mcp-companies` — **37 tests pass** (20 analysis + 9 providers + 8 portfolio)
- Workspace `cargo check` passes (unrelated pre-existing error in `hkask-mcp-media`)

---

## 3. What Remains

### HIGH — Phase 3: Notes & Files (not started)
Per `docs/specifications/portfolio-tracking.md` §4 and §6.3:
- **5 tools:** `note_add`, `note_list`, `note_delete`, `file_attach`, `file_list`, `file_delete`
- **Storage:** `notes` and `files` tables need to be added to the master DB schema
- **Location:** `src/portfolio.rs` for storage, `src/main.rs` for tools
- **Strategy:** Follow same pattern as existing portfolio tools — PortfolioManager methods + tool_router entries

### MEDIUM — TWR/IRR Return Calculation (not started)
Per spec §5.1-5.2:
- Time-weighted return (break at cash flow dates, geometric linking)
- Money-weighted return (IRR)
- **Location:** New analysis module or `src/portfolio.rs`; tool in `main.rs`
- **Dependency:** Needs price data (already have `get_prices()` on PortfolioManager, and `companies_get("historical_price", ...)` in the server)

### MEDIUM — Workspace `cargo check` has pre-existing error in `hkask-mcp-media`
- `CreateCollageRequest` struct has fields not destructured in pattern match
- Unrelated to companies crate — was present before this session

### LOW — `portfolio_create` still exists as a standalone tool
- User said it should auto-happen on import, which it now does
- The tool is still available for explicit creation but is redundant
- Could be removed, or kept as-is (harmless)

---

## 4. Recommended Skills and Tools

```bash
# Activate before working
skill coding-guidelines   # Enforce surgical changes, simplicity, goal-driven execution

# Verify state
cargo check -p hkask-mcp-companies
cargo clippy -p hkask-mcp-companies -- -D warnings
cargo test -p hkask-mcp-companies

# Key files
mcp-servers/hkask-mcp-companies/src/main.rs       # Tools (financial + portfolio)
mcp-servers/hkask-mcp-companies/src/portfolio.rs  # PortfolioManager + tests
mcp-servers/hkask-mcp-companies/src/providers.rs  # FMP/EODHD abstraction
mcp-servers/hkask-mcp-companies/src/analysis.rs   # MAIA framework
docs/specifications/portfolio-tracking.md          # Full spec
```

---

## 5. Key Decisions to Preserve

1. **Single master DB** — All portfolios in one `master.db` at `~/.config/hkask/portfolios/`. Not one `.db` per portfolio. Enables cross-portfolio queries and comparison. All tables have `portfolio_name` with CASCADE deletes.

2. **Provider routing by exchange suffix** — Symbols with `.` (VOD.L, BMW.DE, 7203.T) route to EODHD primary. Plain symbols (AAPL) route to FMP primary. Fallback tries the other provider. This is heuristic but correct for the two providers' coverage areas.

3. **EODHD normalization to FMP format** — All EODHD responses are reshaped to match FMP's flat array structure so `analysis.rs` functions work unchanged. Derived metrics (grossProfitMargin, roic, DPO, DSO) computed from EODHD financial statements when native key-metrics unavailable.

4. **No analyst estimates, no DCF** — These are other people's guesses/models, not data. Removed from tools and endpoint mappings. The expectations gap was rewritten to use only historical data + Gordon Growth Model.

5. **Gordon Growth with profitability-growth correlation** — `P/V = profitability / (r - 2g)`. The `2g` reflects the heuristic that growth and profitability improvement are proportional (a company expected to grow 10% is also expected to improve margins ~10%). Default `r = 15%`. This produces more conservative gap estimates than pure Gordon. The methodology is consistent across all companies, so rank ordering is reliable even if precise quantification is approximate.

6. **Tools are user-facing, not infrastructure** — Stripped `ping`, `ledger_validate`, `portfolio_link_security`, `portfolio_refresh_prices`, `portfolio_refresh_fundamentals`. Validation runs automatically on import. Data fetching happens inline in reports. Symbol linking is internal.

7. **`ledger_import` auto-creates portfolios** — User just imports a CSV. No separate `portfolio_create` step needed. The tool is kept for explicit creation but is redundant.

8. **Portfolio is a ledger, holdings are residuals** — Position = SUM(buys) - SUM(sells) at a point in time. No separate "holdings" concept. All analysis flows from the ledger. No Sharpe ratios, no Brinson attribution, no risk models.
