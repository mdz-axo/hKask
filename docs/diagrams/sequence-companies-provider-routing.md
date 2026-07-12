---
title: "Companies MCP Provider Routing — Sequence Diagram"
audience: [developers, operators, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "hkask-mcp-companies"
mds_categories: [domain, composition, trust, lifecycle]
diataxis: reference
---

# Companies MCP Provider Routing

This reference diagram shows the routing used by eligible financial-data tools. An exchange-qualified symbol prefers EODHD; a plain symbol prefers FMP. The in-memory learning state can select the alternate provider when the default is classified as flaky or stale. A failed primary request is retried through the alternate provider. `company_screener` and `research_search` are outside this path.

See [Companies MCP Server Reference](../reference/mcp-servers/hkask-mcp-companies.md) for the tool-boundary details.

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant Tool as Financial Data Tool
    participant Learn as Learning State
    participant Route as Provider Router
    participant FMP as FMP
    participant EODHD as EODHD

    Client->>+Tool: symbol request
    Tool->>+Learn: clone routing state
    Learn-->>-Tool: provider observations
    Tool->>+Route: companies_get(symbol, state)
    Route->>Route: choose default by symbol shape

    opt Default provider is flaky or stale
        Route->>Route: choose alternate provider
    end

    alt EODHD selected first
        Route->>+EODHD: fetch financial data
        EODHD-->>-Route: response or failure
        opt Successful EODHD financial response
            Route->>Route: normalize to FMP-shaped JSON
        end
        opt EODHD failure
            Route->>+FMP: retry request
            FMP-->>-Route: response or failure
        end
    else FMP selected first
        Route->>+FMP: fetch financial data
        FMP-->>-Route: response or failure
        opt FMP failure
            Route->>+EODHD: retry request
            EODHD-->>-Route: response or failure
            opt Successful EODHD financial response
                Route->>Route: normalize to FMP-shaped JSON
            end
        end
    end

    Route-->>-Tool: JSON result or typed error
    Tool-->>-Client: MCP result
```
<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-010
verified_date: 2026-07-10
verified_against: mcp-servers/hkask-mcp-companies/src/providers.rs:84-247; mcp-servers/hkask-mcp-companies/src/lib.rs:340-361
status: VERIFIED
-->
