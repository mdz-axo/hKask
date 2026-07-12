---
title: "Companies MCP Forecast Feedback — Sequence Diagram"
audience: [developers, operators, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "hkask-mcp-companies"
mds_categories: [domain, composition, trust, lifecycle]
diataxis: reference
---

# Companies MCP Forecast Feedback

This reference diagram shows the durable forecast loop. A DCF or calibrated forecast writes an owner-scoped structured JSON snapshot; an optional same-symbol revision references its parent. `forecast_record` reloads the snapshot, retrieves current financial data for decomposition, appends the outcome, and independently sends an experience to the daemon when configured.

See [Companies MCP Server Reference](../reference/mcp-servers/hkask-mcp-companies.md) for request fields and ownership boundaries.

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant Forecast as Forecast Tool
    participant Store as Owner Forecast Store
    participant Provider as Financial Provider
    participant Daemon as Optional Daemon

    Client->>+Forecast: dcf_valuation or calibrate_forecast
    opt Revision requested
        Forecast->>+Store: get parent forecast
        Store-->>-Forecast: parent or not found
        Forecast->>Forecast: verify same symbol
    end
    Forecast->>Forecast: build structured snapshot
    Forecast->>+Store: save forecast snapshot
    Store-->>-Forecast: forecast_id
    Forecast-->>-Client: forecast_id and valuation

    Client->>+Forecast: forecast_record(forecast_id, outcome)
    Forecast->>+Store: get forecast snapshot
    Store-->>-Forecast: structured snapshot
    Forecast->>+Provider: fetch current financial data
    Provider-->>-Forecast: actual data or error
    opt Actual data available
        Forecast->>Forecast: decompose return gap
    end
    Forecast->>+Store: append outcome and decomposition
    Store-->>-Forecast: persisted
    opt Daemon configured
        Forecast->>+Daemon: store outcome experience
        Daemon-->>-Forecast: accepted or logged failure
    end
    Forecast-->>-Client: recorded outcome
```
<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-011
verified_date: 2026-07-10
verified_against: mcp-servers/hkask-mcp-companies/src/tools/analytics.rs:438-457; mcp-servers/hkask-mcp-companies/src/tools/valuation.rs:634-659,774-915; mcp-servers/hkask-mcp-companies/src/portfolio.rs:303-400
status: VERIFIED
-->
