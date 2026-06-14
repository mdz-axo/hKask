---
title: "hKask Companies MCP Server — User Guide"
audience: [users, agents, operators]
last_updated: 2026-06-14
version: "0.27.0"
status: "Active"
domain: "hkask-mcp-companies"
mds_categories: [domain, composition]
---

# hKask Companies MCP Server

**Crate:** `hkask-mcp-companies` · **Tools:** 21 · **Tests:** 41
**Required credentials:** `HKASK_FMP_API_KEY`, `HKASK_EODHD_API_KEY`

The companies server provides dual-provider company financial data and portfolio tracking. It wraps Financial Modeling Prep (FMP) and EOD Historical Data (EODHD) behind a unified interface, with MAIA-framework fundamental analysis and a full transaction-ledger portfolio management system.

---

## Quick Start

### Setup

```bash
# Set API keys in OS keychain (preferred)
kask keystore set HKASK_FMP_API_KEY "your-fmp-key"
kask keystore set HKASK_EODHD_API_KEY "your-eodhd-key"

# Or load from file (then securely delete plaintext)
cp providers.env.example providers.env   # edit with your keys
kask keystore load --path providers.env --shred
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

### Fundamental Data (8 tools)

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

**Symbol routing rules:** Exchange-qualified symbols (`.L`, `.DE`, `.T`, etc.) route to EODHD. Plain symbols route to FMP. Each provider falls back to the other on failure.

### MAIA Framework Analysis (4 tools)

Deep fundamental analysis using historical financials — no analyst estimates or DCF models:

| Tool | What it does |
|------|-------------|
| **`moat_check`** | 10-year gross margin stability × working capital market power signal → wide/narrow/none classification |
| **`management_scorecard`** | ROIC trend vs invested capital trend → CEO capital allocation score (poor/good/excellent) |
| **`working_capital_cycle`** | 5-year DPO, DSO, DIO, CCC trends → market power and operational efficiency signal |
| **`expectations_gap`** | Gordon Growth Model with profitability-growth correlation → market-implied vs historical growth gap across 3 valuation sets (P/S, P/B, P/Assets) |

**Philosophy:** The market price is a bet on future fundamentals. MAIA tools use only historical data to assess competitive position (moat), management quality (capital allocation), operational efficiency (working capital), and valuation reasonableness (expectations gap). No subjective forecasts — just the numbers.

---

## Portfolio Management Capabilities

A portfolio is a **ledger** — an ordered list of transactions. Positions, returns, and characteristics are all arithmetic on the ledger at a point in time. There is no separate "holdings" concept.

### Ledger Management (5 tools)

| Tool | Description |
|------|-------------|
| `ledger_import` | Import transactions from CSV or JSON. **Auto-creates the portfolio if it doesn't exist.** |
| `ledger_export` | Export full ledger to CSV or JSON for backup or spreadsheet analysis |
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
| **`portfolio_characteristics`** | What the portfolio owns. Weighted-average fundamentals (valuation, profitability, leverage, growth, composition) across all holdings. Computed from price data + key metrics. |
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
├── main.rs          Server entry point, 21 tools
├── providers.rs     Dual-provider abstraction (FMP + EODHD)
├── analysis.rs      MAIA framework (moat, management, WC, expectations)
└── portfolio.rs     Ledger, notes, files, returns
```

---

## Sample Workflow

```bash
# 1. Import your brokerage history
kask chat -f sample_ledger.csv -m qwen3:8b
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

# 6. Add research notes as you go
> note_add my_portfolio AAPL 2024-06-15 "Earnings review" "Beat estimates by 5%, raised guidance" ["earnings","bullish"]

# 7. Attach earnings reports
> file_attach my_portfolio AAPL 2024-06-15 "q2-report.pdf" "application/pdf" "<base64>"

# 8. Compute your returns accounting for deposits and withdrawals
> portfolio_returns my_portfolio 2024-01-01 2024-12-31
```

---

## Configuration

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `HKASK_FMP_API_KEY` | Yes | — | Financial Modeling Prep API key |
| `HKASK_EODHD_API_KEY` | Yes | — | EOD Historical Data API key |

Keys are resolved from the OS keychain (`kask keystore`) first, then environment variables. Use `kask keystore load --path providers.env --shred` for bulk setup with secure deletion.

---

## Related Documents

| Document | Relevance |
|----------|-----------|
| [`docs/specifications/portfolio-tracking.md`](../specifications/portfolio-tracking.md) | Full specification with mathematical reference |
| [`docs/status/mcp-tools-inventory.md`](../status/mcp-tools-inventory.md) | Tool catalog across all 12 MCP servers |
| [`docs/plans/mcp-server-roadmap.md`](../plans/mcp-server-roadmap.md) | Server consolidation and value-add roadmap |
| [`docs/architecture/PRINCIPLES.md`](../architecture/PRINCIPLES.md) | Architecture principles (P8, C4, C8) |
