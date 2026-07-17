# hkask-services-research

Web search, extraction, browsing, and RSS feed management service.

## What this crate provides

- **Provider pool** with RRF fusion across multiple search providers (Brave, Firecrawl, Tavily, SerpAPI, Exa, arXiv, SemanticScholar)
- **Content extraction** via Firecrawl and raw HTTP fetch
- **Headless browsing** via Firecrawl and Browserbase
- **RSS feed management** with SQLite storage (Google Reader stream model)
- **Response caching** with TTL + LRU eviction and provider-fingerprint invalidation
- **Rate limiting** with per-tool token-bucket windows

## Architecture

This is the service layer for the `hkask-mcp-research` MCP server. The MCP server
is a thin tool surface that delegates to this crate.

```
hkask-mcp-research (MCP server — 880 lines)
  └── hkask-services-research (this crate — business logic)
        ├── providers/     Provider pool, RRF fusion, fallback
        ├── types/          Request/response types, ranking, rate limiting, validation
        ├── cache.rs        TTL + LRU response cache
        ├── db.rs           RSS SQLite schema and operations
        ├── feed.rs         Feed fetching and autodiscovery
        ├── rss_types.rs    RSS request types
        └── strip_html.rs   HTML to plain-text conversion
```

## Key types

- `WebSearchPort` — hexagonal port trait for web search operations
- `ProviderPool` — adapter implementing `WebSearchPort`
- `WebError` — domain error type (converts to `McpToolError`)
- `build_provider_pool()` — factory that constructs a pool from credential map

## Dependencies

- `hkask-types` — foundation types (`McpErrorKind`)
- `hkask-memory` — ranking utilities (`rrf_score`, `parse_age_to_days`)
- `hkask-storage` — database abstraction for RSS SQLite
- `hkask-mcp` — URL validation (`validate_tool_url`)