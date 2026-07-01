# hkask-mcp-companies

Company financial data MCP server — FMP + EODHD dual-provider integration for global fundamental analysis, DCF valuation, scenario planning, and superforecasting.

## Tools (38)

### Financial Data (8)
| Tool | Description |
|------|-------------|
| `company_profile` | Get company profile |
| `stock_quote` | Get stock quote |
| `income_statement` | Get income statement |
| `key_metrics` | Get key metrics |
| `historical_price` | Get historical price data |
| `balance_sheet` | Get balance sheet |
| `cash_flow_statement` | Get cash flow statement |
| `symbol_search` | Search for symbols |

### MAIA Fundamental Analysis (4)
| Tool | Description |
|------|-------------|
| `moat_check` | Competitive moat: gross margin stability + working capital market power signal |
| `management_scorecard` | CEO capital allocation: ROIC vs invested capital over time |
| `working_capital_cycle` | CFO scorecard: DPO, DSO, cash conversion cycle trends |
| `expectations_gap` | Gordon Growth Model: market-implied growth vs historical performance |

### Analysis (2)
| Tool | Description |
|------|-------------|
| `company_screener` | Company screener. Parses natural language prompts into FMP stock screener API parameters. Supports filtering by market cap, price, volume, P/E ratio, dividend yield, beta, sector, industry, country, exchange, ROE, ROIC, and more. Use criteria_overrides to adjust parsed criteria. Reply with a modified prompt to refine results. |
| `research_search` | Multi-provider fundamental research search. Searches across Exa, Tavily, and Brave for company-specific information and returns raw claims with source URLs. Use with thesis_test, scenario_weight, or guidance_check skills for structured financial analysis. |

### Valuation (6)
| Tool | Description |
|------|-------------|
| `dcf_valuation` | Two-stage DCF: stage 1 (1-3yr) + stage 2 (2-7yr), terminal perpetuity or multiple, annual/quarterly. Returns intrinsic per share, margin of safety, full projection table. Default: 10yr, 3yr S1, 7yr S2, 10% discount, 2.5% terminal growth. |
| `reverse_dcf` | Mauboussin expectations investing: solves for the growth rate implied by current price using binary search (-50% to +100%). Returns implied growth + interpretation (low/moderate/high expectations). |
| `scenario_analysis` | Schwartz 2×2 matrix: revenue growth × profit margin → four scenarios (Bull, Land Grab, Cash Cow, Bear). Runs DCF under each. Returns intrinsic value range and dispersion. |
| `comparable_analysis` | Comparable company analysis. Gathers valuation multiples (P/E, P/B, P/S, EV/EBITDA) from peer companies in the same industry, alongside a DCF intrinsic value overlay for the target. Multiples provide market-relative context; DCF provides fundamentals-anchored valuation. Accepts optional comma-separated peer list. |
| `sensitivity_analysis` | Tornado chart sensitivity analysis. Varies each DCF driver (revenue growth, gross margin, D&A, capex, NWC, discount rate) by +/- range_pct (default 10%) while holding others constant. Returns drivers ranked by impact on intrinsic value per share. Identifies which assumptions most affect the valuation. |
| `monte_carlo_dcf` | Monte Carlo DCF simulation. Runs N simulations (default 1000, clamped 100-10000) with each DCF assumption randomized uniformly within its +/- configured range. Returns intrinsic value distribution (percentiles p10/p25/median/p75/p90, histogram), probability of undervaluation, and base case comparison. Quantifies valuation uncertainty from assumption ranges. |

### Superforecasting (2)
| Tool | Description |
|------|-------------|
| `calibrate_forecast` | Tetlock GJP pipeline: Fermi decomposition → outside/inside view calibration → probability-weighted scenario distribution. Produces expected intrinsic value vs market price. Accepts direct estimates, Fermi sub-question overrides, and configurable reference class. |
| `forecast_record` | Close the feedback loop: record what actually happened (multiple, price change) at a forecast horizon (3mo/6mo/1yr/2yr/3yr). Computes Brier scores on multiple direction and return accuracy. Decomposes the gap. Stores outcome in daemon for cumulative calibration tracking. |

### Learning & Feedback (1)
| Tool | Description |
|------|-------------|
| `result_feedback` | Rate any tool result 1–5 with optional comments. Feeds the kanban-style learning loop: feedback → LearningState → provider routing override when a provider is flaky for a given symbol. |

### Portfolio Management (15)
| Tool | Description |
|------|-------------|
| `ledger_import` | Import transactions from CSV/JSON (auto-creates portfolio) |
| `ledger_export` | Export portfolio ledger to CSV/JSON |
| `portfolio_list` | List all portfolios |
| `portfolio_delete` | Delete a portfolio and all its data |
| `portfolio_comparison` | Compare two portfolios side by side |
| `portfolio_returns` | Time-weighted + modified Dietz + IRR via Newton's method |
| `portfolio_attribution` | Position-level return decomposition, ranked by contribution |
| `portfolio_characteristics` | FIBO-annotated weighted-average fundamentals |
| `transaction_note_append` | Append a note to a transaction |
| `note_add` | Add a research note to a security |
| `note_list` | List notes, optionally filtered by date range or tags |
| `note_delete` | Delete a note by ID |
| `file_attach` | Attach a file (base64) to a security |
| `file_list` | List attached files for a symbol |
| `file_delete` | Delete an attached file — DB record and disk |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_FMP_API_KEY` | Financial Modeling Prep API key |
| `HKASK_EODHD_API_KEY` | EOD Historical Data API key |
| `HKASK_FERMI_DEFAULTS` | (Optional) JSON override for Fermi sub-question seed estimates |

## Architecture

### Modules
```
src/
├── lib.rs              Server struct, tool methods, LearningState
├── types.rs            Request/response types (34 tool inputs)
├── providers.rs        Dual-provider routing (FMP ↔ EODHD) with learning override
├── analysis.rs         MAIA framework: moat, management, working capital
├── dcf.rs              Two-stage DCF + reverse DCF (binary search)
├── financial_model.rs  11-line-item financial statement projection engine
├── scenarios.rs        Schwartz 2×2 scenario matrix
├── superforecast.rs    Fermi decomposition, outside/inside view, Brier scoring
├── fibo.rs             FIBO ontology mapping (OMG standard financial concepts)
└── portfolio.rs        SQLite-backed portfolio ledger + notes + files
```

### Key design decisions
- **FIBO-anchored**: All financial concepts map to FIBO (Financial Industry Business Ontology) URIs
- **Kanban-style learning loop**: User feedback → `LearningState` → provider routing override. No separate consumer process.
- **Dual-provider with learning**: FMP primary for plain symbols, EODHD for exchange-qualified. Learning state can override when a provider is consistently flaky.
- **Narrative memory**: All data-fetch and analysis tools record full JSON detail to the daemon (not just binary success/error). 15/27 tools have narrative recording.
- **Removed bureaucrat templates**: Deleted 7 selector templates + 4 redundant FlowDef manifests that routed mechanical decisions through LLM personas.

### Tests
- **99 tests** across all modules
- **26 fuzz targets**: dispatch + deserialization for all request types
- **Tracer-bullet contracts**: moat pipeline, CEO alignment, portfolio returns (total return + Modified Dietz), Gordon Growth formula, CAGR, CFO rating boundaries, attribution weights, learning loop integration, Brier scoring

## What Remains

### Not yet built
| Item | Priority |
|------|----------|
| Wire `financial_model.rs` into DCF tools (currently `dcf.rs` uses the old simplified model) | High |
| Decomposition in `forecast_record` uses the 11-line-item model to attribute gap drivers | High |
| Portfolio tools narrative recording (15 tools) | Low — auditable via list/export |
| Real-API integration test fixtures | Deferred — requires mock HTTP layer |
| Full learning loop consumer (daemon → semantic query → pattern → directive) | Deferred — sensor is instrumented |

### Design questions to resolve
- Should `dcf_valuation` and `reverse_dcf` be rebuilt on `financial_model.rs` instead of the old `dcf.rs`?
- Should `forecast_record` store the full projected model alongside the outcome for later decomposition?
- The Fermi defaults are server-level (`HKASK_FERMI_DEFAULTS`). Should they be per-symbol overridable?
- `forecast_record` binary Brier scoring (above/below baseline) vs. continuous Brier score on a distribution

## Quick Start

```bash
export HKASK_FMP_API_KEY="your-fmp-key"
export HKASK_EODHD_API_KEY="your-eodhd-key"

# Optional: custom Fermi seed estimates
export HKASK_FERMI_DEFAULTS='{"growth":[{"estimate":0.70,"confidence":0.8},...],"margin":[...]}'

kask chat
# Or standalone:
hkask-mcp-companies
```
