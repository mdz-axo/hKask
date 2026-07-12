---
title: "MCP Server Reference"
audience: [developers, operators]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition]
last-verified-against: "3d1a876f"
---

# MCP Server Reference

All 15 MCP servers registered in `hkask-mcp::BUILTIN_SERVERS`. Each server follows the standard bootstrap pattern: `bootstrap_mcp_server` → P4 startup gates → `run_stdio_server`. Tools are dispatched through the OCAP membrane (`GovernedTool`).

## Server Listing

| Server | Purpose | Tools | Capability Tier |
|--------|---------|-------|----------------|
| `hkask-mcp-memory` | Unified episodic + semantic memory with cloud backup | 16 | Standard |
| `hkask-mcp-condenser` | Context condensation (thin wrapper over `hkask-condenser`) | 7 | Standard |
| `hkask-mcp-research` | Web search, extraction, and feed-based research | 17 | Standard |
| [`hkask-mcp-companies`](hkask-mcp-companies.md) | Company financial data, valuation, research, and portfolio tools | 41 | Standard |
| `hkask-mcp-communication` | Thin MCP wrapper over core communication crate | 9 | Communication |
| `hkask-mcp-curator` | System observability, escalation management, regulatory memory | 16 | Curator |
| `hkask-mcp-filesystem` | Filesystem and shell access (OCAP-governed, path allowlisting) | 12 | Restricted |
| `hkask-mcp-media` | Media generation (image, video, audio, 3D, workflows via fal.ai) | 37 | Standard |
| `hkask-mcp-docproc` | Unified document processing (format conversion, OCR, chunking, QA) | 9 | Standard |
| `hkask-mcp-training` | Model training (QA pairs, fine-tuning pipeline ingestion) | 8 | Standard |
| `hkask-mcp-replica` | Authorial style and corpus-pipeline operations | 14 | Standard |
| `hkask-mcp-scenarios` | Scenario planning and calibrated event-tree forecasting | 18 | Standard |
| `hkask-mcp-kata-kanban` | Kata-Kanban workflow coordination | 8 | Standard |
| `hkask-mcp-skill` | Skill invocation (exposes registered skills as callable tools) | 15 | Standard |
| `hkask-mcp-codegraph` | Code understanding (query, traverse, impact, context, embed) | 11 | Standard |

## Bootstrap Pattern

Every MCP server follows this pattern. `bootstrap_mcp_server` requires a non-empty host identity (`HKASK_MCP_HOST`, or `HKASK_CURATOR_REPLICANT` for Curator) and fails before daemon verification when it is absent.

```
bootstrap_mcp_server(name, target, host_env_var)
  → Require host identity
  → Register tools via mcp_server! macro
  → Build ToolContext with impl_tool_context!
  → verify_startup_gates() — P4 startup checks
  → run_stdio_server()
```

## Tool Dispatch Flow

1. Tool invocation arrives at server
2. `GovernedTool` membrane checks OCAP capability
3. Energy reserved from gas budget
4. CNS ν-event emitted (`cns.tool.reserved`)
5. Tool implementation executed
6. Energy settled (`cns.tool.completed`)
7. If OCAP denied → `cns.tool.denied` ν-event, error returned

## Capability Tiers

| Tier | Description | Example Servers |
|------|-------------|-----------------|
| `Standard` | General-purpose tools, available to all pods | Most servers |
| `Restricted` | Requires explicit OCAP delegation | `hkask-mcp-filesystem` |
| `Communication` | Matrix/chat integration | `hkask-mcp-communication` |
| `Curator` | System administration and escalation | `hkask-mcp-curator` |

## Registration

All servers are registered in `crates/hkask-mcp/src/lib.rs` in the `BUILTIN_SERVERS` constant (15 entries). See `docs/how-to/bootstrap-mcp-server.md` for adding new servers.

---

## Companies MCP Server (Merged from hkask-mcp-companies.md)


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
