# Portfolio Tracking & Analysis — Specification

**Status:** Specification (not yet implemented)
**Crate:** `hkask-mcp-companies` (extension)
**Last updated:** 2026-06-13

---

## 1. Overview

The companies MCP server (`hkask-mcp-companies`) provides company financial data via FMP + EODHD dual-provider integration. This specification extends it with **portfolio tracking** — a transaction ledger, security-company linkage, notes and files, and simple transparent reporting on what the portfolio is and what moved it.

**Core idea:** A portfolio is a ledger. Everything else — positions, weights, returns, characteristics — is just arithmetic on the ledger at a point in time. There are no separate "holdings" — a position is always the residual of buys minus sells. The goal is to show what's going on clearly, without the made-up stats that distract from understanding.

---

## 2. Transaction Ledger

### 2.1 Ledger Schema

The ledger is the single source of truth for portfolio positions. Every transaction is immutable once recorded. Positions are always the net of buys and sells — there is no separate "holdings" table.

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique transaction identifier |
| `date` | Date (ISO 8601) | Trade date (not settlement date) |
| `type` | Enum | `buy`, `sell`, `dividend`, `deposit`, `withdrawal` |
| `symbol` | String | Security identifier (nullable for cash transactions) |
| `quantity` | f64 | Number of shares/units (positive for buys, zero for cash transactions) |
| `price` | f64 | Price per share (nullable for cash transactions) |
| `commission` | f64 | Transaction cost (nullable, always positive) |
| `amount` | f64 | Cash amount for dividends/deposits/withdrawals (nullable for trades) |
| `currency` | String | ISO 4217 currency code (default: `USD`) |
| `notes` | String | Free-text rationale or explanation |

### 2.2 Position Calculation

Positions are derived, never stored:

```
position(symbol) = SUM(buys.quantity) - SUM(sells.quantity)
```

Cash balance:

```
cash = SUM(deposits.amount)
     + SUM(withdrawals.amount)  // withdrawals are negative
     + SUM(dividends.amount)
     - SUM(buys.quantity * buys.price + buys.commission)
     + SUM(sells.quantity * sells.price - sells.commission)
```

### 2.3 Ledger Ingestion

**Supported formats:**
- CSV (column-mapped, header row required)
- JSON (array of transaction objects)
- Manual entry (one transaction at a time via tool call)

**Ingestion rules:**
- Duplicate detection by date + symbol + type + quantity + price tuple
- Validation: quantity > 0 for buys, price > 0 for trades, dates parseable
- Errors collected and reported; no partial ingestion

### 2.4 Ledger Maintenance

- Append-only: transactions can be added but not modified
- Correction: add an offsetting transaction with notes explaining the correction
- Deletion: not supported (append offsetting transaction instead)
- Export: CSV or JSON dump of full ledger

---

## 3. Security-Company Linkage

### 3.1 Symbol Resolution

Securities in the ledger are identified by symbol. The companies MCP server resolves symbols to company data via FMP (US) or EODHD (global, exchange-qualified).

**Symbol formats:**
- Plain: `AAPL`, `MSFT` → FMP primary, EODHD fallback with `.US` suffix
- Exchange-qualified: `VOD.L`, `BMW.DE`, `7203.T` → EODHD primary

### 3.2 Data Linkage

Each security in the portfolio links to:

| Data | Source Tool | Frequency |
|------|------------|-----------|
| Company profile | `company_profile` | On demand / weekly refresh |
| Real-time quote | `stock_quote` | On demand |
| Historical prices | `historical_price` | Daily (for return calculation) |
| Key metrics | `key_metrics` | Quarterly (after earnings) |
| Financial statements | `income_statement`, `balance_sheet`, `cash_flow_statement` | Quarterly |
| MAIA analysis | `moat_check`, `management_scorecard`, `working_capital_cycle`, `expectations_gap` | On demand |

### 3.3 Price Master

For portfolio valuation and return calculation, a **price master** is maintained:

- Daily closing prices for each security in the portfolio
- Sourced from `historical_price` (FMP or EODHD)
- Stored locally for offline access and audit
- Updated via a `refresh_prices` tool that fetches missing dates

---

## 4. Notes & Files

### 4.1 Company Notes

Notes associated with a company/security as of a specific date:

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique note identifier |
| `symbol` | String | Security symbol |
| `date` | Date | As-of date for the note |
| `title` | String | Note title |
| `body` | String | Note content (Markdown) |
| `tags` | [String] | Optional tags for categorization |
| `created_at` | DateTime | When the note was created |

### 4.2 Transaction Notes

Notes attached to specific ledger transactions (rationale for buy/sell, explanation of deposit/withdrawal):

- Stored as the `notes` field on the transaction itself (see §2.1)
- Can be added at transaction creation time or appended later
- Append operation: `append_transaction_note(tx_id, note_text)` — concatenates to existing notes with timestamp prefix

### 4.3 File Attachments

Files associated with companies/securities:

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique file identifier |
| `symbol` | String | Security symbol |
| `date` | Date | As-of date |
| `filename` | String | Original filename |
| `mime_type` | String | MIME type |
| `size` | u64 | File size in bytes |
| `path` | String | Storage path (local filesystem) |
| `notes` | String | Optional description |

**Supported formats:** PDF, PNG, JPEG, CSV, XLSX, plain text.
**Storage:** Local filesystem under `~/.config/hkask/portfolios/{portfolio_name}/files/`.

---

## 5. Portfolio Analysis & Reporting

A portfolio is a ledger. Everything else is just arithmetic on the ledger at a point in time.

### 5.1 What the Portfolio Is (At Any Date)

For any date `T`, compute from the ledger:

```
position(symbol, T) = SUM(buys of symbol before T) - SUM(sells of symbol before T)
```

This is a residual — the net shares from all transactions up to that date. If you bought more than you sold, it's positive. If you sold everything, it's zero. There is no separate "holdings" concept.

```
market_value(symbol, T) = position(symbol, T) * price(symbol, T)
cash(T) = net of all deposits, withdrawals, dividends, and trade settlements up to T
total_value(T) = SUM(market_value) + cash(T)
```

Weight of each position: `weight_i = market_value_i / total_value`.

### 5.2 How the Portfolio Performed

Between two dates `T1` and `T2`, the portfolio value changed. Some of that change is from money you added or removed; the rest is investment return.

**Time-weighted return (TWR):** Removes the effect of deposits and withdrawals by breaking the period at each cash flow date. Between cash flows, return is just the change in value divided by starting value:

```
r_i = (MV_end - MV_start) / MV_start
TWR = ∏(1 + r_i) - 1
```

**Money-weighted return (IRR):** The discount rate that makes the net present value of all cash flows zero — what the investor actually experienced given when they added or removed money.

### 5.3 What the Portfolio Owns (Characteristics)

Weighted-average fundamentals of the residuals. These describe the aggregate business you own a piece of — not forecasts, not risk models.

**Valuation:**

| Characteristic | Source |
|---------------|--------|
| Weighted P/E | `company_profile` / `key_metrics` |
| Weighted P/B | `company_profile` |
| Weighted P/S | `company_profile` (price / revenuePerShare) |
| Weighted EV/EBITDA | `key_metrics` |

**Profitability:**

| Characteristic | Source |
|---------------|--------|
| Weighted ROIC | `key_metrics` |
| Weighted ROE | `key_metrics` |
| Weighted gross margin | `key_metrics` |
| Weighted operating margin | `key_metrics` |
| Weighted net margin | `key_metrics` |

**Balance sheet:**

| Characteristic | Source |
|---------------|--------|
| Weighted financial leverage (assets/equity) | `balance_sheet` |
| Weighted debt/equity | `balance_sheet` |
| Weighted current ratio | `balance_sheet` |

**Growth:**

| Characteristic | Source |
|---------------|--------|
| Weighted revenue growth (1yr) | `income_statement` |
| Weighted EPS growth (1yr) | `income_statement` |
| Weighted dividend yield | `company_profile` |

**Composition:**

| Characteristic | Source |
|---------------|--------|
| Sector weights | `company_profile` |
| Industry weights | `company_profile` |
| Market cap distribution | `company_profile` |
| Geographic exposure | `company_profile` |

Each characteristic is `SUM(weight_i * metric_i)` — the portfolio-weight-weighted average of what each residual represents. Computable as-of any date by using positions and prices at that date with the most recent fundamental data.

### 5.4 What Moved the Portfolio

For any period, each residual's contribution to the total move:

```
contribution_i = weight_i_at_start * return_i
```

Where `weight_i_at_start` is the position's percentage of total portfolio value at period start, and `return_i` is the security's price change plus dividends over the period.

**Output:** A table ranked by absolute contribution — biggest movers first:
- Symbol, sector
- Weight at start (%), weight at end (%)
- Security return (%)
- Contribution to portfolio return (basis points)
- Dollar gain/loss

That's it. No decomposition, no benchmarks, no cash drag. A portfolio of 20+ stocks is complicated enough — this just shows what moved and by how much.

### 5.5 How the Residuals Changed Over Time

The ledger accumulates. Residuals appear, grow, shrink, disappear. Over any period:

| Question | How to answer it |
|----------|-----------------|
| What entered? | Symbols with zero position at start, non-zero at end |
| What exited? | Symbols with non-zero position at start, zero at end |
| What grew? | Symbols where position increased (more buys than sells) |
| What shrank? | Symbols where position decreased (more sells than buys) |
| How many positions? | Count of non-zero residuals |
| How concentrated? | Weight of the largest residuals |

All of these are just queries on the ledger between two dates. No separate analytics — just looking at how the residuals evolved.

### 5.6 Reports

| Tool | Description |
|------|-------------|
| `portfolio_summary` | Current residuals, market values, cash, total value |
| `portfolio_returns` | TWR and IRR for any date range |
| `portfolio_characteristics` | Weighted-average fundamentals of what the residuals represent |
| `portfolio_attribution` | What moved the portfolio — each residual's contribution ranked by impact |
| `portfolio_holdings` | Each residual with cost basis, gain/loss, weight |
| `portfolio_income` | Dividend history |
| `portfolio_transactions` | Filtered ledger view |

---

## 6. Tool Summary

### 6.1 Ledger Tools

| Tool | Description |
|------|-------------|
| `portfolio_create` | Create a new named portfolio |
| `portfolio_delete` | Delete a portfolio and all associated data |
| `portfolio_list` | List all portfolios |
| `transaction_add` | Add a transaction to the ledger (buy/sell/dividend/deposit/withdrawal) |
| `transaction_note_append` | Append a note to an existing transaction |
| `ledger_import` | Import transactions from CSV or JSON |
| `ledger_export` | Export full ledger to CSV or JSON |
| `ledger_validate` | Validate ledger consistency (positions, cash, dates) |

### 6.2 Data Tools

| Tool | Description |
|------|-------------|
| `portfolio_refresh_prices` | Fetch missing daily prices for all portfolio securities |
| `portfolio_refresh_fundamentals` | Refresh company profiles and key metrics for all holdings |
| `portfolio_link_security` | Explicitly link a ledger symbol to a company data symbol |

### 6.3 Notes & Files Tools

| Tool | Description |
|------|-------------|
| `note_add` | Add a note to a company/security as of a date |
| `note_list` | List notes for a symbol, optionally filtered by date range or tags |
| `note_delete` | Delete a note |
| `file_attach` | Attach a file to a company/security |
| `file_list` | List attached files for a symbol |
| `file_delete` | Delete an attached file |

### 6.4 Analysis & Reporting Tools

| Tool | Description |
|------|-------------|
| `portfolio_summary` | Current residuals, market values, cash, total value |
| `portfolio_returns` | TWR and IRR for any date range |
| `portfolio_characteristics` | Weighted-average fundamentals of what the residuals represent |
| `portfolio_attribution` | What moved the portfolio — each residual's contribution ranked by impact |
| `portfolio_holdings` | Each residual with cost basis, gain/loss, weight |
| `portfolio_income` | Dividend history |
| `portfolio_transactions` | Filtered ledger view |

**Total new tools:** 21 (8 ledger + 3 data + 5 notes/files + 7 analysis)

---

## 7. Storage

### 7.1 Ledger Storage

SQLite database at `~/.config/hkask/portfolios/{name}.db`:

```sql
CREATE TABLE transactions (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('buy','sell','dividend','deposit','withdrawal')),
    symbol TEXT,
    quantity REAL,
    price REAL,
    commission REAL,
    amount REAL,
    currency TEXT DEFAULT 'USD',
    notes TEXT DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX idx_transactions_date ON transactions(date);
CREATE INDEX idx_transactions_symbol ON transactions(symbol);
```

### 7.2 Notes & Files Storage

```sql
CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    tags TEXT DEFAULT '[]',  -- JSON array
    created_at TEXT NOT NULL
);

CREATE TABLE files (
    id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size INTEGER NOT NULL,
    path TEXT NOT NULL,
    notes TEXT DEFAULT '',
    created_at TEXT NOT NULL
);
```

### 7.3 Price Cache

```sql
CREATE TABLE price_cache (
    symbol TEXT NOT NULL,
    date TEXT NOT NULL,
    close REAL NOT NULL,
    source TEXT NOT NULL,  -- 'FMP' or 'EODHD'
    fetched_at TEXT NOT NULL,
    PRIMARY KEY (symbol, date)
);
```

---

## 8. Implementation Phases

### Phase 1: Core Ledger (tools 1–8)
- Portfolio CRUD (create, delete, list)
- Transaction add with validation
- Position and cash computation
- CSV/JSON import and export
- Ledger validation

### Phase 2: Data Linkage (tools 9–11)
- Symbol-to-company resolution via existing companies MCP tools
- Price master with daily refresh
- Fundamental data refresh

### Phase 3: Notes & Files (tools 12–16)
- Company notes CRUD
- Transaction note append
- File attachment and listing

### Phase 4: Analysis & Reporting (tools 17–23)
- TWR and IRR calculation
- Portfolio characteristics (valuation, profitability, leverage, growth, composition)
- What moved the portfolio (security-level contribution ranking)
- Holdings analysis
- Report generation tools

---

## 9. Mathematical Reference

### 9.1 Position (The Residual)

At any date `T`, for each symbol:

```
position = SUM(buys before T) - SUM(sells before T)
```

This is the only definition of a "holding." It is always a residual of the ledger.

### 9.2 Time-Weighted Return

Given cash flow dates `t_0, t_1, ..., t_n` and end date `T`:

1. Divide the period into sub-periods at each cash flow date
2. For each sub-period `[t_i, t_{i+1})`:
   - `MV_start` = market value at `t_i` after the cash flow at `t_i`
   - `MV_end` = market value at `t_{i+1}` before the cash flow at `t_{i+1}`
   - `r_i = (MV_end - MV_start) / MV_start`
3. `TWR = ∏(1 + r_i) - 1`

Market values are computed from residuals × closing prices on each date. Cash is included in total value.

When daily valuations aren't available, the Modified Dietz method approximates:

```
R ≈ (MV_end - MV_start - CF) / (MV_start + SUM(CF_i * w_i))
```

Where `w_i = (T - t_i) / T` weights each cash flow by time remaining.

### 9.3 What Moved the Portfolio

For a period from `t_start` to `t_end`:

```
contribution_i = weight_i_start * return_i

where:
  weight_i_start = (residual_i * price_start_i) / total_value_at_start
  return_i = (price_end_i - price_start_i + dividends_i) / price_start_i
```

Sum of contributions equals the return from invested residuals. The difference between that and total portfolio return is the effect of cash — visible directly in the numbers.

---

## 10. Open Questions

1. **Multi-currency support:** Initial implementation is USD-only. Multi-currency requires exchange rate data and currency-aware return calculation. Deferred to Phase 5.

2. **Corporate actions:** Stock splits, spin-offs, and mergers affect position quantities and cost basis. Initial implementation requires manual adjustment via offsetting transactions. Automated corporate action processing deferred.

3. **Tax lot accounting:** FIFO, LIFO, or specific identification for cost basis and realized gain/loss. Initial implementation uses average cost basis. Tax lot methods deferred.

4. **Multi-portfolio:** Initial implementation is single-portfolio. Multi-portfolio aggregation (household, strategy sleeves) deferred.

5. **Real-time pricing:** Initial implementation uses daily closing prices from historical data. Real-time intraday pricing via `stock_quote` deferred.

6. **Custom/private securities:** Currently only supports publicly traded securities with data from FMP or EODHD. Private investments — startups, real estate, collectibles, private equity, unregistered securities — cannot be tracked alongside public holdings.

   A custom security would be a user-defined journal entry with:
   - Symbol (user-chosen identifier)
   - Company name, description, URL to data resources
   - User-managed pricing (manual entry or periodic upload)
   - Optional links to external data sources for auto-pricing

   This enables:
   - Tracking private investments in the same ledger as public holdings
   - Pure private portfolios for alternative asset tracking
   - Hybrid portfolios mixing public + private for true total-wealth accounting

   **Storage:** New `custom_securities` table in master DB:
   ```sql
   CREATE TABLE custom_securities (
       id TEXT PRIMARY KEY,
       portfolio_name TEXT NOT NULL,
       symbol TEXT NOT NULL UNIQUE,
       name TEXT,
       description TEXT,
       url TEXT,
       pricing_mode TEXT DEFAULT 'manual',  -- 'manual' or 'linked'
       linked_source TEXT,                   -- e.g., 'fmp:AAPL' to track private holding at AAPL's price
       valuation_currency TEXT DEFAULT 'USD',
       created_at TEXT NOT NULL,
       FOREIGN KEY (portfolio_name) REFERENCES portfolios(name) ON DELETE CASCADE
   );
   ```

   **Flow:**
   1. User creates custom security with `security_create_custom` (portfolio, symbol, name, url, pricing_mode)
   2. User adds buy/sell transactions with that symbol (same as public securities)
   3. User periodically updates price via `security_update_price` (symbol, date, price) or uploads CSV
   4. All portfolio analysis tools work unchanged — positions, returns, characteristics, attribution

   **Tool surface (6 new tools):** `security_create_custom`, `security_list_custom`, `security_delete_custom`, `security_update_price`, `security_import_prices`, `security_link_public` (track private holding at public security's price for proxy valuation)

   **Status:** Deferred to Phase 6. Depends on Phase 5 multi-currency for non-USD private holdings.
