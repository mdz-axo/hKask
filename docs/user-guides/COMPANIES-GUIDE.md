---
title: "Companies MCP Server — User Guide"
audience: [users, agents, operators]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "hkask-mcp-companies"
mds_categories: [domain, composition, trust, lifecycle]
---

# Companies MCP Server User Guide

Use `hkask-mcp-companies` to retrieve company data, calculate valuations, research claims, maintain durable forecast feedback, and manage a local investment ledger. The server has **41 tools**. For tool names, request fields, and implemented constraints, consult the [Companies MCP Server Reference](../reference/mcp-servers/hkask-mcp-companies.md).

## Configure providers

The server requires both market-data credentials at startup. Research keys are optional.

```bash
kask keystore set HKASK_FMP_API_KEY "your-fmp-key"
kask keystore set HKASK_EODHD_API_KEY "your-eodhd-key"
```

Optional research providers:

```bash
kask keystore set HKASK_EXA_API_KEY "your-exa-key"
kask keystore set HKASK_TAVILY_API_KEY "your-tavily-key"
kask keystore set HKASK_BRAVE_API_KEY "your-brave-key"
```

Set `HKASK_FERMI_DEFAULTS` only when you need custom Fermi starting points. It is a JSON object with `growth` and `margin` arrays; each entry may provide `question`, `estimate`, and `confidence`.

```bash
export HKASK_FERMI_DEFAULTS='{
  "growth": [{"question": "market growth", "estimate": 0.08, "confidence": 0.6}],
  "margin": [{"question": "steady-state margin", "estimate": 0.25, "confidence": 0.6}]
}'
```

## Retrieve company data

Use an exchange-qualified symbol when the listing is not a plain US-style ticker.

```text
company_profile AAPL
stock_quote MSFT
income_statement VOD.L
symbol_search "Tesla"
```

Eligible financial-data calls prefer FMP for plain symbols and EODHD for exchange-qualified symbols, then fall back after a provider failure. `company_screener` is FMP-only. `research_search` uses optional Exa, Tavily, and Brave providers instead of the financial-data route.

See the [provider-routing sequence](../diagrams/sequence-companies-provider-routing.md) for the exact path.

## Run a valuation

A DCF valuation builds a history-calibrated, two-stage projection and persists an owner-scoped `forecast_id`.

```text
dcf_valuation AAPL
reverse_dcf AAPL
sensitivity_analysis AAPL
monte_carlo_dcf AAPL
```

The active DCF model uses a Gordon-growth terminal value and subtracts net debt from enterprise value. It does not use exit multiples, a separate SG&A line, or other non-operating assets. The MCP boundary rejects non-finite values, out-of-range assumptions, invalid horizons, and terminal growth at or above the discount rate.

`scenario_analysis` executes a fixed revenue-growth × gross-margin matrix.

## Record a forecast outcome

`forecast_record` requires forecast and outcome values, not a free-text actuals summary. Supply all required fields:

```json
{
  "symbol": "AAPL",
  "forecast_date": "2025-01-01",
  "horizon": "1yr",
  "forecast_multiple": 30.0,
  "forecast_price_change": 0.10,
  "outcome_date": "2026-01-01",
  "actual_multiple": 28.0,
  "actual_price_change": 0.03,
  "forecast_id": "the-id-returned-by-dcf-valuation"
}
```

The identifier persists across restarts. Use `forecast_get` to retrieve one record and outcomes, or `forecast_list AAPL` to review the owner's history. Pass `revision_of` to `dcf_valuation` or `calibrate_forecast` to create a same-symbol revision linked to its predecessor. Decomposition still requires current actual financial data.

## Manage a portfolio ledger

Import transactions as CSV or JSON, then query portfolio analysis or add research records.

```text
ledger_import my_portfolio csv "type,date,symbol,quantity,price,commission,amount\nbuy,2024-01-15,AAPL,10,150,1,"
portfolio_returns my_portfolio 2024-01-01 2024-12-31
portfolio_attribution my_portfolio 2024-01-01 2024-12-31
note_add my_portfolio AAPL 2024-06-15 "Earnings review" "Raised guidance" ["earnings"]
```

Portfolio state is stored locally under `~/.config/hkask/portfolios/<sanitized-webid>/`, so the authenticated MCP `WebID` determines each caller's database and attachment namespace. Imports are limited to 5 MiB and 10,000 transactions; attachments are limited to 10 MiB encoded and 6 MiB decoded.

The former shared `portfolios/master.db` is not auto-migrated because it has no reliable owner identity. Export legacy data from a trusted single-principal deployment and import it into the correct owner-scoped server.

## Ontology annotations

Derived outputs may provide a `fibo` object with compact FIBO identifiers. Raw provider payloads are not field-mapped, and the server does not emit a JSON-LD context or Dublin Core/PKO mapping. Treat these identifiers as application metadata, not self-resolving semantic-web URIs.

## Related documentation

- [Companies MCP Server Reference](../reference/mcp-servers/hkask-mcp-companies.md)
- [Companies server README](../../mcp-servers/hkask-mcp-companies/README.md)
- [MCP Server Reference](../reference/mcp-servers/README.md)
- [Documentation Standards](../specifications/DOCUMENTATION_STANDARDS.md)
