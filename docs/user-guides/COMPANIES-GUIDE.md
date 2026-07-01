---
title: "hKask Companies MCP Server — User Guide"
audience: [users, agents, operators]
last_updated: 2026-06-24
version: "0.31.0"
status: "Active"
domain: "hkask-mcp-companies"
mds_categories: [domain, composition]
---

# hKask Companies MCP Server

**Crate:** `hkask-mcp-companies` · **Tools:** 38 · **Tests:** 107
**Required credentials:** `HKASK_FMP_API_KEY`, `HKASK_EODHD_API_KEY`

The companies server provides dual-provider company financial data and portfolio tracking. It wraps Financial Modeling Prep (FMP) and EOD Historical Data (EODHD) behind a unified interface, with MAIA-framework fundamental analysis, a full DCF valuation engine, superforecasting pipeline, and a complete transaction-ledger portfolio management system.

---

## Quick Start

### Setup

```bash
# Set API keys in OS keychain (preferred)
kask keystore set HKASK_FMP_API_KEY "your-fmp-key"
kask keystore set HKASK_EODHD_API_KEY "your-eodhd-key"

# Or load from file (then securely delete plaintext)
cp .env.example .env   # edit with your keys
kask keystore load --path .env --shred
```

### Test Your Setup

From an agent session, try:

```
> company_profile AAPL          # Fetch Apple profile
> stock_quote MSFT              # Real-time Microsoft quote
> symbol_search "Tesla"         # Find Tesla symbols
```

---

## Company Research Capabilities

### Fundamental Data (9 tools)

Pull raw financial data from either provider with automatic routing:

| Tool | What it returns | Provider routing |
|------|----------------|------------------|
| `company_profile` | Name, sector, industry, market cap, description, CEO | AAPL → FMP; VOD.L → EODHD |
| `stock_quote` | Price, change, volume, day range | Same auto-routing |
| `income_statement` | Revenue, operating income, net income (default 5 years) | Pass `limit` for more years |
| `balance_sheet` | Assets, liabilities, equity (default 5 years) | Pass `limit` for more years |
| `cash_flow_statement` | Operating/investing/financing cash flows | Pass `limit` for more years |
| `key_metrics` | P/E, ROE, ROIC, margins, debt ratios | Derived from EODHD financials when native unavailable |
| `historical_price` | Daily OHLCV data for a date range | `from` and `to` required |
| `symbol_search` | Find symbols by company name or ticker | FMP primary, EODHD fallback |
| `company_screener` | Parse natural language screening prompts into FMP stock screener parameters | FMP screener API — supports market cap, price, volume, P/E, dividend, beta, sector, industry, country, exchange, ROE, ROIC, debt/equity, price/book |

**Symbol routing rules:** Exchange-qualified symbols (`.L`, `.DE`, `.T`, etc.) route to EODHD. Plain symbols route to FMP. Each provider falls back to the other on failure.

---

### Valuation Framework (13 tools)

Deep fundamental analysis and valuation — from competitive position assessment through full two-stage DCF modeling, expectations investing, scenario planning, and calibrated probability forecasting.

#### MAIA Competitive Assessment

| Tool | What it does |
|------|-------------|
| **`moat_check`** | 10-year gross margin stability × working capital market power signal → wide/narrow/none classification |
| **`management_scorecard`** | ROIC trend vs invested capital trend → CEO capital allocation score (poor/good/excellent) |
| **`working_capital_cycle`** | 5-year DPO, DSO, DIO, CCC trends → market power and operational efficiency signal |
| **`expectations_gap`** | Gordon Growth Model with profitability-growth correlation → market-implied vs historical growth gap across 3 valuation sets (P/S, P/B, P/Assets) |

**Philosophy:** The market price is a bet on future fundamentals. MAIA tools use only historical data to assess competitive position (moat), management quality (capital allocation), operational efficiency (working capital), and valuation reasonableness (expectations gap). No subjective forecasts — just the numbers.

#### DCF Valuation & Scenario Analysis

| Tool | What it does |
|------|-------------|
| **`dcf_valuation`** | 11-line-item two-stage DCF with history-calibrated projections. Returns `forecast_id` for later decomposition |
| **`reverse_dcf`** | Mauboussin expectations investing — solve for implied growth from current price. Binary search over revenue growth |
| **`scenario_analysis`** | Schwartz 2×2 matrix: Bull / Land Grab / Cash Cow / Bear scenarios, each run through the full financial model |
| `sensitivity_analysis` | Tornado chart — varies each DCF driver ±range_pct, ranks by impact on intrinsic value |
| `monte_carlo_dcf` | N simulations with randomized assumptions → intrinsic value distribution, probability of undervaluation |
| `comparable_analysis` | Peer multiples (P/E, P/B, P/S, EV/EBITDA) alongside DCF intrinsic overlay |

#### Superforecasting Pipeline

Per **Tetlock's Good Judgment Project methodology** (Tetlock, 2005; Tetlock & Gardner, 2015): calibrated probability forecasting with Fermi decomposition, outside/inside view, Bayesian evidence updating, and Brier score tracking.[^tetlock-2005] [^tetlock-2015]

| Tool | What it does |
|------|-------------|
| **`calibrate_forecast`** | Tetlock GJP pipeline — Fermi decomposition + outside/inside view → probability-weighted intrinsic value |
| **`forecast_record`** | Close the loop — record actual outcome, Brier scores, 11-line-item gap decomposition |
| **`result_feedback`** | 1–5 score + comments feeding a kanban-style learning loop for provider routing quality |

---

#### Financial Model Architecture

The DCF engine models 11 line items from revenue through free cash flow:

```
Revenue → COGS → Gross Profit → SG&A → EBIT → Tax → NOPAT
→ (+) D&A add-back
→ (−) Capex
→ (−) NWC change
= Free Cash Flow
```

**Historical calibration.** Each projection is seeded by a `HistoricalSnapshot::from_api_json` — pulled from 5+ years of income statements, balance sheets, and cash flow statements. The model computes historical averages for margins, capex/revenue ratios, working capital turnover, and tax rates, then projects forward using a two-stage structure (explicit forecast period → terminal value via perpetuity growth or exit multiple).

**Enterprise → equity bridge.** The model produces enterprise value (PV of future FCFs), then bridges to equity value per share:

```
Enterprise Value
  − Net Debt (total debt − cash)
  + Other non-operating assets
= Equity Value
  ÷ Diluted Shares Outstanding
= Intrinsic Value Per Share
```

The `sensitivity_analysis` tool produces a tornado chart — varying each of 6 key drivers (revenue growth, gross margin, D&A/revenue, capex/revenue, NWC/revenue, discount rate) independently by a configurable percentage (default ±10%) while holding others constant. Drivers are ranked by their impact on intrinsic value per share. This answers "which assumption matters most?" — critical for focusing research effort on the highest-impact drivers.

The `monte_carlo_dcf` tool replaces single-point assumptions with uniform distributions. Each assumption is randomized within its configured ±range across N simulations (default 1,000, clamped 100–10,000). The result is a full intrinsic value distribution: percentiles (p10/p25/median/p75/p90), mean, standard deviation, probability of undervaluation vs current price, and a 10-bucket histogram. This quantifies valuation uncertainty — not just "what is it worth?" but "how sure are we?"

#### Superforecasting Loop

The superforecasting pipeline closes the loop between prediction and outcome:

```
dcf_valuation / calibrate_forecast  →  forecast_id
        |
        v
   (time passes, actual results arrive)
        |
        v
forecast_record  →  Brier scores  →  7-component gap decomposition
```

- **`forecast_id`** links `dcf_valuation` and `calibrate_forecast` outputs to a `forecast_record` call. Every projection is traceable.
- **Brier scores** measure probability calibration — how close were your confidence-weighted predictions to outcomes?
- **7-component gap decomposition** breaks the difference between forecast and actual into: revenue miss, gross margin delta, SG&A leverage, tax rate change, capex intensity, working capital surprise, and terminal value error. Each component is attributable to a specific assumption.

#### Learning Loop

`result_feedback` tracks per-symbol provider quality. Flaky providers accumulate low scores and are de-prioritized in routing decisions. The learning loop is **in-process** — no async consumer, no separate database. Scores live in the server's runtime state and feed routing decisions on the next symbol lookup.

#### FIBO Ontology Anchoring

All outputs carry FIBO (Financial Industry Business Ontology) concept URIs. Every response that includes financial data — portfolio characteristics, DCF valuations, scenario analyses, superforecast projections, and fundamental research — is annotated with a `fibo` block mapping output fields to canonical FIBO URIs. This enables semantic interoperability: an agent can reason that `fibo:FBC:EquityValuation:NetDebt` in a DCF output is the same concept as `fibo:FBC:DebtAndEquity:NetFinancialDebt` in a balance sheet. Over 25 FIBO concepts are mapped in `fibo.rs`, including the three newest additions:

- `sensitivity_analysis`: `fibo-fbc-fct-ra:SensitivityAnalysis`
- `monte_carlo_dcf`: `fibo-fbc-fct-ra:MonteCarloDcf`
- `comparable_company_analysis`: `fibo-fbc-fct-ra:ComparableCompanyAnalysis`

Fundamental research tools carry fibo annotations for assumption impacts (`fibo-fbc-fct-ra:DcfAssumptionAdjustment`) and scenario probabilities (`fibo-fbc-fct-ra:ScenarioProbability`).

---

### Fundamental Research (1 tool + 3 skills)

Multi-provider research engine (Exa, Tavily, Brave) that searches for company-specific information. Structured financial analysis (thesis testing, scenario weighting, guidance calibration) is performed by LLM-driven skills using templates in the registry.

| Tool | What it does |
|------|-------------|
| **`research_search`** | Multi-provider search across Exa, Tavily, and Brave. Returns raw claims with source URLs for downstream analysis |

#### Research Skills (registry templates)

These compose `research_search` with company data and financial models via FlowDef manifests:

| Skill | Manifest | KnowAct Template | Pipeline |
|-------|----------|------------------|----------|
| `thesis_test` | `registry/templates/thesis-test/manifest.yaml` | `thesis-test.j2` | profile → research_search → key_metrics → LLM extraction → conditional dcf_valuation |
| `scenario_weight` | `registry/templates/scenario-weight/manifest.yaml` | `scenario-weight.j2` | profile → research_search → scenario_analysis → LLM probability shifts |
| `guidance_check` | `registry/templates/guidance-check/manifest.yaml` | `guidance-check.j2` | profile → research_search → key_metrics → LLM calibration |

Run them via: `kask run thesis_test --symbol AAPL --thesis "services revenue will accelerate"`

#### Provider Configuration

| Variable | Required | Purpose |
|----------|----------|---------|
| `HKASK_EXA_API_KEY` | No | Exa AI search API (semantic search) |
| `HKASK_TAVILY_API_KEY` | No | Tavily research API (deep web search) |
| `HKASK_BRAVE_API_KEY` | No | Brave Search API (web + news) |

Any combination works — the engine uses whichever keys are available. If none are configured, `research_search` returns empty results gracefully.

---

## Portfolio Management Capabilities

A portfolio is a **ledger** — an ordered list of transactions. Positions, returns, and characteristics are all arithmetic on the ledger at a point in time. There is no separate "holdings" concept.

### Ledger Management (5 tools)

| Tool | Description |
|------|-------------|
| `ledger_import` | Import transactions from CSV or JSON. **Auto-creates the portfolio if it doesn't exist.** |
| `ledger_export` | Export full ledger to CSV or JSON for backup or spreadsheet analysis. Includes FIBO-annotated metadata blocks. |
| `portfolio_list` | List all portfolios in `~/.config/hkask/portfolios/master.db` |
| `portfolio_delete` | Delete a portfolio and all associated data (ledger, notes, files, prices) |
| `transaction_note_append` | Annotate a transaction with rationale (why did you buy/sell?) |

**CSV format:**

```csv
type,date,symbol,quantity,price,commission,amount
buy,2024-01-15,AAPL,10,150.0,1.0,
sell,2024-02-20,AAPL,3,160.0,1.0,
dividend,2024-03-01,AAPL,,,,0.5
deposit,2024-01-01,,,,,10000.0
withdrawal,2024-06-01,,,,,5000.0
```

Transaction types: `buy`, `sell`, `dividend`, `deposit`, `withdrawal`. For buy/sell: symbol + quantity + price. For dividend/deposit/withdrawal: amount only. Commission is optional.

**Validation:** Import automatically validates positions (buys = sells + residuals) and cash consistency. Issues are reported in the response. No separate validate step needed.

### Portfolio Analysis (4 tools)

| Tool | Description |
|------|-------------|
| **`portfolio_attribution`** | What moved the portfolio. For a date range, ranks each position by contribution (weight × return). Shows weight change, security return, contribution in basis points, dollar gain/loss. |
| **`portfolio_characteristics`** | What the portfolio owns. Weighted-average fundamentals (valuation, profitability, leverage, growth, composition) across all holdings. Computed from price data + key metrics. Output includes FIBO concept URIs for each characteristic. |
| **`portfolio_comparison`** | Side-by-side comparison of two portfolios — shared positions, unique symbols, transaction counts. |
| **`portfolio_returns`** | TWR (Modified Dietz approximation) and IRR (Newton's method) for any date range. Accounts for deposits and withdrawals at day-level precision. |

**How attribution works:**

```
contribution_i = weight_i_at_start × return_i

weight_i_at_start = (shares × price_start_i) / total_value_at_start
return_i = (price_end_i - price_start_i + dividends_i) / price_start_i
```

Positions ranked by absolute contribution — biggest movers first. No decomposition, no benchmarks, no cash drag. A portfolio of 20+ stocks is complicated enough — this just shows what moved and by how much.

### Research Notes & Files (6 tools)

Track your research alongside your positions:

| Tool | Description |
|------|-------------|
| `note_add` | Add a note to a security (symbol, date, title, body, tags). Returns UUID. |
| `note_list` | List notes with optional date range and tag filtering |
| `note_delete` | Delete a note by ID |
| `file_attach` | Attach a file (base64-encoded PDF, image, spreadsheet) to a security. Stored at `~/.config/hkask/portfolios/{name}/files/`. |
| `file_list` | List attached files for a symbol |
| `file_delete` | Delete file record + physical file from disk |

---

## Architecture

### Storage

All data lives in a single SQLite database at `~/.config/hkask/portfolios/master.db`. Every table carries `portfolio_name` with CASCADE deletes — delete a portfolio and everything goes.

```sql
-- Tables (all CREATE IF NOT EXISTS on first use)
portfolios      — name, created_at
transactions    — id, portfolio_name, date, type, symbol, quantity, price, ...
price_cache     — portfolio_name, symbol, date, close, source
security_links  — portfolio_name, ledger_symbol, data_symbol
notes           — id, portfolio_name, symbol, date, title, body, tags
files           — id, portfolio_name, symbol, date, filename, mime_type, size, path, notes
```

Files are stored on the local filesystem at `~/.config/hkask/portfolios/{portfolio_name}/files/{uuid}_{filename}`.

### Provider Architecture

```
User query: "BMW.DE"
  → providers.rs detects exchange suffix ".DE"
  → Routes to EODHD primary
  → Fetches /fundamentals/BMW.DE (nested JSON)
  → Normalizes to FMP flat-array format
  → analysis.rs functions work transparently

User query: "AAPL"
  → Plain symbol → FMP primary
  → FMP returns flat array natively
  → Falls back to EODHD with ".US" suffix if FMP fails
```

EODHD's nested response format is normalized to match FMP's structure, so all analysis functions (moat, management, working capital, expectations gap, characteristics) work identically regardless of which provider served the data.

### Module Structure

```
mcp-servers/hkask-mcp-companies/src/
├── main.rs            Server entry point, 38 tools
├── providers.rs       Dual-provider abstraction (FMP + EODHD)
├── analysis.rs        MAIA framework (moat, management, WC, expectations)
├── research.rs        Multi-provider fundamental research engine (Exa, Tavily, Brave)
├── portfolio.rs       Ledger, notes, files, returns
├── financial_model.rs 11-line-item projection engine + gap decomposition
├── scenarios.rs       Schwartz 2×2 scenario planning
├── screener.rs        Natural language stock screener prompt parser
├── superforecast.rs   Fermi decomposition, Bayesian calibration, Brier scores
├── fibo.rs            FIBO ontology mapping (25+ concepts)
└── types.rs           Request/response types
```

---

## Sample Workflow

```bash
# 1. Import your brokerage history
/chat -f sample_ledger.csv -m qwen3:8b
> ledger_import my_portfolio csv "<paste CSV content>"

# 2. Check your positions
> portfolio_list
# → ["my_portfolio"]

# 3. See what you own and what it's worth
> portfolio_characteristics my_portfolio 2025-01-01

# 4. See what moved your portfolio year-to-date
> portfolio_attribution my_portfolio 2024-01-01 2024-12-31

# 5. Compare with a friend's portfolio
> portfolio_comparison my_portfolio their_portfolio

# 6. Search for research across multiple providers
> research_search AAPL "services revenue growth outlook 2025"

# 7. Test an investment thesis (skill)
> kask run thesis_test --symbol AAPL --thesis "services will accelerate"

# 8. Weight scenarios by current signals (skill)
> kask run scenario_weight --symbol AAPL

# 9. Calibrate against management guidance (skill)
> kask run guidance_check --symbol AAPL

# 10. Run a DCF valuation on a holding
> dcf_valuation my_portfolio AAPL
# → returns forecast_id for later decomposition

# 11. Check market-implied growth expectations
> reverse_dcf AAPL

# 12. See which assumptions most affect your valuation
> sensitivity_analysis AAPL

# 13. Quantify valuation uncertainty
> monte_carlo_dcf AAPL

# 14. Compare against industry peers
> comparable_analysis AAPL

# 15. Add research notes as you go
> note_add my_portfolio AAPL 2024-06-15 "Earnings review" "Beat estimates by 5%, raised guidance" ["earnings","bullish"]

# 16. Attach earnings reports
> file_attach my_portfolio AAPL 2024-06-15 "q2-report.pdf" "application/pdf" "<base64>"

# 17. Compute your returns accounting for deposits and withdrawals
> portfolio_returns my_portfolio 2024-01-01 2024-12-31

# 18. Close the superforecasting loop when actuals arrive
> forecast_record dcf_abc123 "2024 Q4 actuals: rev=119.6B, eps=2.40"
```

---

## Limitations & Roadmap

**Current limitations:**
- **Public securities only.** Only FMP/EODHD-listed securities are supported. Private investments (startups, real estate, collectibles, private equity) cannot be tracked.
- **USD only.** Multi-currency support is deferred to Phase 5.
- **Manual corporate actions.** Stock splits and mergers require manual adjustment via offsetting transactions.
- **In-memory forecast store.** DCF projections are point-in-time snapshots stored in-memory. Server restart clears the forecast store. Forecast-to-record linking requires the same server session.

**Planned (see spec §10 for details):**
- **Custom/private securities** (Phase 6) — user-defined symbols with manual pricing or proxy links to public securities
- **Multi-currency support** (Phase 5) — exchange rate data and currency-aware returns
- **Corporate actions** — automated split/spin-off/merger processing
- **Tax lot accounting** — FIFO/LIFO/specific identification
- **Persistent forecast store** — disk-backed forecast storage for cross-session linking

---

## Configuration

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `HKASK_FMP_API_KEY` | Yes | — | Financial Modeling Prep API key |
| `HKASK_EODHD_API_KEY` | Yes | — | EOD Historical Data API key |
| `HKASK_FERMI_DEFAULTS` | No | Server defaults | JSON env var for Fermi sub-question seed estimates |
| `HKASK_EXA_API_KEY` | No | — | Exa AI search API (semantic search) |
| `HKASK_TAVILY_API_KEY` | No | — | Tavily research API (deep web search) |
| `HKASK_BRAVE_API_KEY` | No | — | Brave Search API (web + news) |

Keys are resolved from the OS keychain (`kask keystore`) first, then environment variables. Use `kask keystore load --path .env --shred` for bulk setup with secure deletion.

`HKASK_FERMI_DEFAULTS` accepts a JSON object of `{ "symbol": { "sub_question_key": value } }` pairs that seed the Fermi decomposition engine in `calibrate_forecast`. These are starting-point estimates, not overrides — the Bayesian update pipeline will adjust them as evidence arrives.

---

## Related Documents

| Document | Relevance |
|----------|-----------|
| [`docs/status/PROJECT_STATUS.md`](../status/PROJECT_STATUS.md) | Tool catalog across all 12 MCP servers |
| [`docs/architecture/core/PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) | Architecture principles (P8) |

[^tetlock-2005]: Tetlock, P. E. (2005). *Expert Political Judgment: How Good Is It? How Can We Know?* Princeton University Press.
[^tetlock-2015]: Tetlock, P. E. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*. Crown.
