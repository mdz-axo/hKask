# hkask-mcp-companies

Company financial data MCP server — FMP + EODHD dual-provider integration for global fundamental analysis.

## Tools (31)

| Tool | Description |
|------|-------------|
| `company_profile` | Company profile lookup |
| `stock_quote` | Real-time stock quote |
| `income_statement` | Income statement data |
| `balance_sheet` | Balance sheet data |
| `cash_flow_statement` | Cash flow statement |
| `key_metrics` | Key financial metrics |
| `historical_price` | Historical price data |
| `symbol_search` | Symbol search |
| `fmp_search_get` | FMP provider search |
| `eodhd_search_get` | EODHD provider search |
| `expectations_gap` | Expectations gap analysis |
| `management_scorecard` | Management quality scorecard |
| `moat_check` | Economic moat analysis |
| `working_capital_cycle` | Working capital cycle analysis |
| `companies_get` | Get company data |
| `portfolio_list` | List portfolios |
| `portfolio_delete` | Delete portfolio |
| `portfolio_returns` | Portfolio returns |
| `portfolio_attribution` | Portfolio attribution |
| `portfolio_characteristics` | Portfolio characteristics |
| `portfolio_comparison` | Portfolio comparison |
| `note_add` | Add note |
| `note_delete` | Delete note |
| `note_list` | List notes |
| `transaction_note_append` | Append transaction note |
| `ledger_export` | Export ledger |
| `ledger_import` | Import ledger |
| `file_attach` | Attach file |
| `file_delete` | Delete file |
| `file_list` | List files |
| `run` | Main run loop |

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
