# hkask-mcp-companies

Company financial data MCP server — FMP + EODHD dual-provider integration for global fundamental analysis.

## Tools (27)

| Tool | Description |
|------|-------------|
| `company_profile` | Get company profile |
| `stock_quote` | Get stock quote |
| `income_statement` | Get income statement |
| `balance_sheet` | Get balance sheet |
| `cash_flow_statement` | Get cash flow statement |
| `key_metrics` | Get key metrics |
| `historical_price` | Get historical price data |
| `symbol_search` | Search for symbols |
| `moat_check` | Analyze competitive moat using MAIA framework: gross margin stability and working capital market power signal |
| `management_scorecard` | CEO capital allocation scorecard (MAIA framework): rates how well management allocates capital by comparing returns on capital vs invested capital over time |
| `working_capital_cycle` | Working capital cycle analysis (MAIA CFO scorecard): tracks days payable, days sales outstanding, and cash conversion cycle over time |
| `expectations_gap` | Expectations gap: compare trailing 5-year actual performance to the future performance implied by the current price. Uses Gordon Growth Model to compute implied growth from valuation multiples vs historical profitability and growth |
| `portfolio_delete` | Delete a portfolio and all its data |
| `portfolio_list` | List all portfolios |
| `ledger_import` | Import transactions from CSV or JSON into a portfolio ledger |
| `ledger_export` | Export portfolio ledger to CSV or JSON |
| `transaction_note_append` | Append a note to an existing transaction |
| `portfolio_comparison` | Compare two portfolios side by side — positions, overlap, unique symbols |
| `portfolio_returns` | Time-weighted and money-weighted returns for a date range |
| `portfolio_attribution` | What moved the portfolio — each position's weight, return, and contribution, ranked by impact |
| `portfolio_characteristics` | Weighted-average fundamentals of what the portfolio owns — valuation, profitability, leverage, growth, composition |
| `note_add` | Add a note to a company/security as of a date |
| `note_list` | List notes for a symbol, optionally filtered by date range or tags |
| `note_delete` | Delete a note by ID |
| `file_attach` | Attach a file (base64-encoded) to a company/security |
| `file_list` | List attached files for a symbol in a portfolio |
| `file_delete` | Delete an attached file by ID — removes record and file from disk |

## Configuration

| Variable | Description |
|----------|-------------|
| `FMP_API_KEY` | Financial Modeling Prep API key |
| `EODHD_API_KEY` | EOD Historical Data API key |

## Quick Start

```bash
# Set API keys
export FMP_API_KEY="your-fmp-key"
export EODHD_API_KEY="your-eodhd-key"

# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-companies
```

## Usage

```
# In kask chat, invoke tools via the MCP router:
"Look up AAPL company profile"   → company_profile
"What's TSLA's P/E ratio?"        → key_metrics
"Show me MSFT income statement"   → income_statement
"Compare my tech portfolio"       → portfolio_comparison
```
