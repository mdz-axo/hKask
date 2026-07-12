---
title: "Companies MCP Server Reference"
audience: [developers, operators, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "hkask-mcp-companies"
mds_categories: [domain, composition, trust, lifecycle]
---

# Companies MCP Server Reference

`hkask-mcp-companies` exposes **41 MCP tools** for company data, financial analysis, valuation, research retrieval, durable forecast feedback, and a local portfolio ledger. The server requires Financial Modeling Prep (FMP) and EOD Historical Data (EODHD) credentials at startup.

## Tool groups

| Router group | Tools |
|---|---:|
| Financial data | 8 |
| Analysis and research | 5 |
| Portfolio analytics and DCF | 5 |
| Valuation and forecasting | 8 |
| Economic-profit valuation | 1 |
| Expectations analysis | 1 |
| Portfolio ledger, notes, and files | 13 |

The crate [README](../../../mcp-servers/hkask-mcp-companies/README.md) is the compact complete tool-name catalog. The MCP schemas generated from `src/types.rs` are authoritative for request fields.

## Provider routing

Eligible symbol-based financial-data requests route by symbol shape: exchange-qualified symbols prefer EODHD; plain symbols prefer FMP. The selected provider may be overridden by the in-memory learning state when one provider is classified as flaky or stale. A failed primary request falls back to the other provider. EODHD financial results are normalized where the downstream calculation expects FMP-shaped data.

`company_screener` is an FMP-specific endpoint, not a dual-provider query. `research_search` queries optional Exa, Tavily, and Brave providers independently of FMP/EODHD.

See [Companies provider routing](../../diagrams/sequence-companies-provider-routing.md).

## Valuation semantics

The server’s main DCF tools use a history-calibrated two-stage financial model and a Gordon-growth terminal-value calculation. The model includes revenue, COGS, gross profit, D&A, EBIT, tax, NOPAT, capex, net-working-capital change, and free cash flow. It bridges enterprise value to equity value by subtracting net debt.

The active model does **not** implement a separate SG&A line, an exit-multiple terminal calculation, or an other-non-operating-assets adjustment. The MCP request schema exposes only inputs the active projection model consumes.

`scenario_analysis` runs a fixed revenue-growth × gross-margin matrix. The emitted `axes` object is the executed scenario definition.

## Forecast feedback lifecycle

`dcf_valuation` and `calibrate_forecast` persist an owner-scoped structured JSON snapshot with a `forecast_id`. `forecast_get` retrieves a record and recorded outcomes; `forecast_list` returns the owner's records for one symbol. Passing `revision_of` creates a same-symbol child forecast after validating that the parent belongs to the current owner. `forecast_record` appends its outcome to the forecast and reloads the snapshot for decomposition, so restart does not discard calibration history.

This is a durable owner-scoped feedback loop. It is not a general-purpose document store. See [Companies forecast feedback](../../diagrams/sequence-companies-forecast-feedback.md).

## Portfolio storage and safety boundary

Portfolio records, notes, and attachment metadata use an owner-scoped SQLite database at `~/.config/hkask/portfolios/<sanitized-webid>/master.db`; attached file bytes live under that owner-scoped directory. The authenticated MCP `WebID` determines the namespace, so caller-supplied portfolio names and child-resource IDs cannot cross owner databases.

Ledger import accepts at most 5 MiB and 10,000 transactions. Attachments accept at most 10 MiB of base64 input and 6 MiB after decoding. Persistence work runs on Tokio's blocking worker pool rather than the async request worker.

Legacy data in the former shared `portfolios/master.db` location is intentionally not auto-migrated: that database has no trustworthy owner identity. Export it from a trusted single-principal deployment and import it into the appropriate owner-scoped server.

## Ontology annotations

Some derived outputs contain a `fibo` object whose values are FIBO compact identifiers from `src/fibo.rs`. Raw provider data is returned as provider JSON and is not field-mapped. The response does not currently provide a JSON-LD context or prefix-expansion mapping, and it does not emit the claimed Dublin Core/PKO bridge.

## Request constraints

The valuation boundary validates finite values, documented rate and horizon ranges, bounded sensitivity/Monte Carlo ranges, checked projection horizons, and `discount_rate > terminal_growth`. Invalid request values are rejected before projection.

## Operational configuration

| Variable | Required | Purpose |
|---|---|---|
| `HKASK_FMP_API_KEY` | Yes | FMP financial-data provider |
| `HKASK_EODHD_API_KEY` | Yes | EODHD financial-data provider |
| `HKASK_EXA_API_KEY` | No | Exa research retrieval |
| `HKASK_TAVILY_API_KEY` | No | Tavily research retrieval |
| `HKASK_BRAVE_API_KEY` | No | Brave research retrieval |
| `HKASK_FERMI_DEFAULTS` | No | Fermi `growth`/`margin` question defaults |

## Related documentation

- [MCP server index](README.md)
- [Companies server README](../../../mcp-servers/hkask-mcp-companies/README.md)
- [Companies provider-routing diagram](../../diagrams/sequence-companies-provider-routing.md)
- [Documentation standards](../../specifications/DOCUMENTATION_STANDARDS.md)
