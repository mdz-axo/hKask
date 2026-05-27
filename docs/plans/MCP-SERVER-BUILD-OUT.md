# hKask MCP Specification & Continuation Plan

**Version:** 0.21.0  
**Date:** 2026-05-27  
**Scope:** MCP server specifications/standards/protocols for hKask, and build-out plan for web, scholar, and condenser servers

---

## Part 1: hKask MCP Server Specification

### 1.1 Protocol & Transport

All hKask MCP servers use **rmcp** (Rust MCP) implementing JSON-RPC 2.0:

| Transport | Type | Use Case |
|-----------|------|----------|
| `rmcp::transport::stdio()` | Stdio | Default for all server binaries |
| `InProcessMcpTransport` | In-process | Co-located servers within `kask` process |
| `HttpMcpTransport` | HTTPS | Remote servers with OCAP token auth |

### 1.2 Server Binary Architecture

Every hKask MCP server binary follows this canonical structure:

```
mcp-servers/hkask-mcp-{name}/
├── Cargo.toml
└── src/
    └── main.rs     # Single-file server (uses shared scaffolding)
```

**`main.rs` structure:**
1. Imports from `hkask_mcp::server` (shared scaffolding)
2. Constants (`SERVER_VERSION`, API base URLs)
3. Request types (`#[derive(Deserialize, JsonSchema)]`)
4. Domain types (internal, no I/O)
5. Outbound port traits (if hexagonal — `#[async_trait]`)
6. Driven adapter implementations
7. Server struct with `#[tool_router(server_handler)]` impl block
8. `#[tokio::main] async fn main()` using `run_stdio_server()`

### 1.3 Shared Scaffolding API (`hkask_mcp::server`)

| Export | Purpose |
|--------|---------|
| `McpToolError` | Structured errors with `McpErrorKind` classification |
| `McpToolOutput` | Structured output with optional metadata |
| `CredentialRequirement` | Declarative credential needs (required/optional) |
| `classify_http_error(service, status, body)` | HTTP status → McpToolError mapping |
| `api_get(client, service, url)` | Authenticated GET with auto error classification |
| `api_post(client, service, url, payload)` | Authenticated POST with auto error classification |
| `resolve_credential(env_var)` | Keystore-first credential resolution (keychain → env var) |
| `emit_tool_span(tool, outcome, duration_ms, error_kind)` | CNS `cns.tool` tracing span emission |
| `validate_identifier(name, value, max_len)` | Input sanitization for identifiers |
| `validate_tool_url(url)` | SSRF-protected URL validation |
| `run_stdio_server(name, version, factory, credentials)` | Common bootstrap (tracing, cred check, rmcp serve) |

### 1.4 Tool Method Convention

```rust
#[tool(description = "Human-readable description")]
async fn tool_name(
    &self,
    Parameters(RequestType { field1, field2 }): Parameters<RequestType>,
) -> String {
    let start = Instant::now();
    // 1. Input validation
    if let Err(e) = validate_identifier("field1", &field1, 64) {
        return e.to_json_string();
    }
    // 2. API call or business logic
    match some_operation().await {
        Ok(v) => {
            emit_tool_span("tool_name", "ok", start.elapsed().as_millis() as u64, None);
            McpToolOutput::with_timing(v, start).to_json_string()
        }
        Err(e) => {
            emit_tool_span("tool_name", "error", start.elapsed().as_millis() as u64, Some(&e.kind));
            e.to_json_string()
        }
    }
}
```

### 1.5 Credential Resolution Protocol

Resolution order for every credential:
1. **OS keychain** via `hkask_keystore::Keychain::retrieve_by_key(env_var_name)`
2. **Environment variable** via `std::env::var(env_var_name)`

All API key env vars follow `HKASK_{SERVICE}_API_KEY` or `HKASK_{SERVICE}_TOKEN` naming.

### 1.6 Error Classification Protocol

HTTP status codes map to `McpErrorKind` uniformly:

| Status | McpErrorKind | Retryable |
|--------|-------------|-----------|
| 401/403 | `PermissionDenied` | No |
| 404 | `NotFound` | No |
| 422 | `InvalidArgument` | No |
| 429 | `RateLimited` | Yes |
| 502/503 | `Unavailable` | Yes |
| Other 5xx | `Unavailable` | Yes |
| Other | `Internal` | No |

### 1.7 CNS Observability Protocol

Every tool emits `tracing::info!(target: "cns.tool", ...)` spans:
- **Mutation tools** (create, delete, send, subscribe): emit on both success and error
- **Read tools** (get, list, search): emit on error only (success timing captured by `McpToolOutput::with_timing`)
- Span fields: `tool`, `outcome` ("ok"/"error"), `duration_ms`, `error_kind`

### 1.8 Security Gateway Integration

All tools are gated through `SecurityGateway` (`hkask-mcp/src/security.rs`):
- OCAP `CapabilityToken` verification before dispatch
- Rate limiting per `WebID`
- Input size validation (1MB default)
- Tool allow/deny list enforcement
- SSRF URL validation on all URL parameters

### 1.9 Tool Tier Classification

| Tier | Description | Example |
|------|-------------|---------|
| **L2 Upstream** | Thin wrapper over external API | `github_get_repo`, `scholar_search`, `web_search` |
| **L2 Store** | Local persistence (no network) | `scholar_store_get_paper`, `rss_get_entries` |
| **L3 Composite** | Multi-step orchestration across L2 tools | `scholar_graph_segment`, `web_research`, `condenser_compress` |

---

## Part 2: Web Server Specification (`hkask-mcp-web`)

### 2.1 Identity
- **Crate:** `hkask-mcp-web`
- **Description:** Unified web search, content extraction, and interactive browsing via multi-provider routing
- **Tool tier:** L2 (upstream) + L3 (composite: `web_research`)

### 2.2 Tool Surface (5 tools)

| Tool | Tier | Description | Required Params | Optional Params |
|------|------|-------------|-----------------|-----------------|
| `web_ping` | utility | Liveness + provider health check | — | — |
| `web_search` | L2 | Keyword/semantic web search | `query` | `num_results`, `include_domains`, `exclude_domains`, `freshness`, `search_type`, `strategy` |
| `web_extract` | L2 | Extract content from URL → markdown/JSON | `url` | `format` (markdown/json), `json_prompt`, `json_schema`, `main_content_only`, `wait_for_ms` |
| `web_browse` | L2 | Interactive JS-heavy page navigation | `url` | `instruction`, `timeout_secs` |
| `web_research` | L3 | Deep multi-step research cascade | `query` | `max_pages`, `include_domains`, `exclude_domains`, `freshness` |

### 2.3 Provider Architecture (Hexagonal)

**Outbound port traits:**

```rust
#[async_trait]
trait WebSearchProvider: Send + Sync {
    fn kind(&self) -> &str;
    fn capabilities(&self) -> Vec<SearchCapability>;
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
trait WebExtractProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn extract(&self, url: &str, opts: ExtractOptions) -> Result<ExtractedContent, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
trait WebBrowseProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn browse(&self, url: &str, instruction: &str, timeout: Duration) -> Result<BrowseResult, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}
```

**Provider implementations:**

| Provider | Traits | API Base | Auth | Capabilities |
|----------|--------|----------|------|-------------|
| **BraveProvider** | Search | `https://api.search.brave.com/res/v1` | `X-Subscription-Token: {key}` | keyword, news, freshness |
| **FirecrawlProvider** | Search, Extract, Browse | `https://api.firecrawl.dev/v1` | `Bearer {key}` | search, extract (md/json), browse |
| **RawFetchProvider** | Extract only | (direct HTTP) | (none) | basic HTML→text |

**Initial providers:** Brave (search) + Firecrawl (search/extract/browse) + RawFetch (extract fallback).  
**Deferred providers:** Exa (semantic search), Browserbase (headless browse) — can be added later without architectural changes.

### 2.4 Strategy Engine

| Strategy | Flow | Primary | Fallback |
|----------|------|---------|----------|
| `quick` | Single search | Brave | Firecrawl |
| `semantic` | Neural search | Firecrawl | Brave |
| `extract` | Search + extract top N | Brave search → Firecrawl extract | — |
| `deep` | Multi-query → dedup → extract → cross-ref | All providers concurrent | — |
| `fetch` | URL → content | Firecrawl | RawFetch |

### 2.5 Cache

- **Backend:** In-memory `RwLock<HashMap<CacheKey, CacheEntry>>`
- **TTL:** Default 300s, max 7200s (`HKASK_WEB_CACHE_TTL_SECS`)
- **Max entries:** Default 50, max 200 (`HKASK_WEB_CACHE_MAX_ENTRIES`)
- **Key:** blake3 hash of (strategy + query/url + serialized params)

### 2.6 Credentials

| Env Var | Provider | Required |
|---------|----------|----------|
| `HKASK_BRAVE_API_KEY` | BraveProvider | Yes |
| `HKASK_FIRECRAWL_API_KEY` | FirecrawlProvider | No (degraded: no extract/browse) |

### 2.7 Error Domain (`WebError`)

| Variant | McpErrorKind |
|---------|-------------|
| `BadArgs` | `InvalidArgument` |
| `ProviderUnavailable` | `Unavailable` |
| `ProviderError` | `Internal` |
| `RateLimited` | `RateLimited` |
| `NoProvider` | `Unavailable` |
| `CascadeFailed` | `Internal` |

### 2.8 hKask Adaptations from Arsenal Reference

1. **Replace `stack-mcp-protocol`** with `hkask_mcp::server` shared scaffolding
2. **Replace `arsenal-config-secrets`** with `resolve_credential()` from `hkask_mcp::server`
3. **Replace `arsenal-mcp-middleware`** with `emit_tool_span()` from `hkask_mcp::server`
4. **Replace `McpToolServer` trait** with `rmcp` `#[tool_router(server_handler)]` macros
5. **Replace `stack-domain-types::ErrorKind`** with `hkask_types::McpErrorKind`
6. **Use `validate_tool_url()`** on all URL parameters
7. **Use `classify_http_error()`** for all API error classification
8. **Use `run_stdio_server()`** with factory pattern for main()
9. **CNS spans** target `cns.tool.web_search`, `cns.tool.web_extract`, `cns.tool.web_browse`, `cns.tool.web_research`
10. **No redb/bitemporal store** — web server is stateless (cache only)

---

## Part 3: Scholar Server Specification (`hkask-mcp-scholar`)

### 3.1 Identity
- **Crate:** `hkask-mcp-scholar`
- **Description:** Semantic Scholar Graph API wrapper + local bitemporal persistence + citation-graph traversal
- **Tool tier:** L2 (upstream + store) + L3 (composite: `scholar_graph_segment`)

### 3.2 Tool Surface (12 tools)

| Tool | Tier | Description | Required Params |
|------|------|-------------|-----------------|
| `scholar_ping` | utility | Liveness + API health | — |
| `scholar_search` | L2 upstream | Relevance search over S2 corpus | `query` |
| `scholar_paper_details` | L2 upstream | Full paper metadata | `paper_id` |
| `scholar_paper_batch` | L2 upstream | Up to 500 papers | `paper_ids` |
| `scholar_citations` | L2 upstream | Papers citing a paper | `paper_id` |
| `scholar_references` | L2 upstream | Papers cited by a paper | `paper_id` |
| `scholar_author` | L2 upstream | Author profile + papers | `author_id` |
| `scholar_recommendations` | L2 upstream | Recommended papers from seeds | `positive_paper_ids` |
| `scholar_graph_segment` | L3 composite | Bounded citation-graph traversal | `seeds` OR `query` |
| `scholar_store_get_paper` | L2 store | Read paper from local store | `paper_id` |
| `scholar_store_graph` | L2 store | Traverse locally-persisted citation graph | `seeds` |
| `scholar_store_stats` | L2 store | Summary counts | — |

### 3.3 Outbound Port Trait

```rust
#[async_trait]
trait ScholarApi: Send + Sync {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError>;
    async fn get_papers_batch(&self, ids: &[&str], fields: Option<&str>) -> Result<Vec<Option<Paper>>, ScholarError>;
    async fn search_papers(&self, query: &str, limit: Option<u32>, offset: Option<u32>, fields: Option<&str>) -> Result<SearchResults, ScholarError>;
    async fn list_citations(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<CitationPage, ScholarError>;
    async fn list_references(&self, paper_id: &str, offset: Option<u32>, limit: Option<u32>, fields: Option<&str>) -> Result<ReferencePage, ScholarError>;
    async fn get_author(&self, author_id: &str, fields: Option<&str>) -> Result<Author, ScholarError>;
    async fn recommend(&self, positive_ids: &[&str], negative_ids: &[&str]) -> Result<Vec<Paper>, ScholarError>;
}
```

### 3.4 Driven Adapters

| Adapter | Purpose |
|---------|---------|
| `HttpScholarApi` | Production HTTP adapter calling `https://api.semanticscholar.org/graph/v1` |
| `PersistingScholarApi` | Decorator: write-through to local SQLite store after every API call |

### 3.5 API Endpoints (Semantic Scholar Graph API v1)

| S2 Endpoint | Tool |
|-------------|------|
| `GET /paper/{paper_id}?fields={fields}` | `scholar_paper_details` |
| `POST /paper/batch?fields={fields}` | `scholar_paper_batch` |
| `GET /paper/search?query={q}&limit={n}&offset={o}&fields={f}` | `scholar_search` |
| `GET /paper/{id}/citations?fields={f}&offset={o}&limit={n}` | `scholar_citations` |
| `GET /paper/{id}/references?fields={f}&offset={o}&limit={n}` | `scholar_references` |
| `GET /author/{author_id}?fields={f}` | `scholar_author` |
| `GET /recommendations/v1/papers/forpaper/{id}` | `scholar_recommendations` (single) |
| `POST /recommendations/v1/papers/` | `scholar_recommendations` (multi) |

**Auth:** `x-api-key: {HKASK_SEMANTIC_SCHOLAR_API_KEY}`

### 3.6 Local Persistence (ScholarStore)

- **Backend:** Plain `rusqlite` (same pattern as RSS reader, NOT encrypted `hkask-storage::Database`)
- **Tables:** `papers`, `authors`, `citations`, `paper_authors`
- **Write-through:** `PersistingScholarApi` wraps `HttpScholarApi` — every API call writes to local store
- **Store path:** `HKASK_SCHOLAR_DB` env var, default `hkask-scholar.db`
- **Store tools work offline** — no API key required for `scholar_store_*`

### 3.7 Graph Segment (L3 Composite)

- **BFS traversal** of citation graph with configurable depth
- **Two modes:** `Full` (fetch paper metadata for every node) and `Skeleton` (edges-only topology)
- **Hard caps:** depth ≤ 3, fan-out ≤ 100 per node per direction, upstream call budget default 200
- **Batch optimization:** Level-by-level BFS with `get_papers_batch` for metadata (3× efficiency)
- **Returns:** `{seeds, depth, nodes, edges, truncated, upstream_calls}`

### 3.8 Credentials

| Env Var | Required |
|---------|----------|
| `HKASK_SEMANTIC_SCHOLAR_API_KEY` | Yes (for upstream tools; store tools work without it) |

### 3.9 hKask Adaptations from Arsenal Reference

1. **Replace `stack-bitemporal-store`** with plain `rusqlite` — simpler, no redb dependency, consistent with RSS reader pattern
2. **Replace `arsenal-config-secrets`** with `resolve_credential()`
3. **Replace `McpToolServer`** with rmcp `#[tool_router(server_handler)]`
4. **Replace `ScholarApiError` custom kind strings** with `McpErrorKind` taxonomy
5. **Use `validate_identifier()`** on paper_id/author_id params
6. **Use `classify_http_error("Scholar", ...)` for S2 API errors
7. **Use `emit_tool_span()` for CNS observability
8. **Use `run_stdio_server()` with factory pattern
9. **CNS spans:** `cns.tool.scholar_search`, `cns.tool.scholar_graph_segment`, etc.

---

## Part 4: Condenser Server Specification (`hkask-mcp-condenser`)

### 4.1 Identity
- **Crate:** `hkask-mcp-condenser`
- **Description:** Context condensation — compress tool outputs, manage profiles, classify categories
- **Tool tier:** L2 (local engine, no network) + L3 (composite: cascade pipelines)
- **Dependency:** `hkask-templates` (for `InferencePort` LLM access in advanced algorithms)

### 4.2 Tool Surface (5 tools)

| Tool | Tier | Description | Required Params | Optional Params |
|------|------|-------------|-----------------|-----------------|
| `condenser_ping` | utility | Liveness + profile info | — | — |
| `condenser_compress` | L2 | Compress tool output | `tool_name`, `output` | `category` |
| `condenser_set_profile` | L2 | Set compression aggressiveness | `profile` | — |
| `condenser_stats` | L2 | Cumulative compression statistics | — | — |
| `condenser_classify` | L2 | Classify tool name → context category | `tool_name` | — |

### 4.3 Compression Profiles

| Profile | Retained | Max Lines | Use Case |
|---------|----------|-----------|----------|
| `heavy` | 10% | 30 | Emergency context recovery |
| `normal` | 20% | 80 | Default — balanced |
| `soft` | 60% | 200 | Preserve most detail |
| `light` | 95% | unlimited | Minimal compression |

### 4.4 Context Categories

| Category | Description | Typical Tools |
|----------|-------------|---------------|
| `shell_command` | CLI output | git, cargo, docker |
| `test_output` | Test runner output | cargo test, pytest |
| `build_output` | Compiler/build output | cargo build, npm |
| `file_contents` | Source file content | cat, read |
| `conversation_history` | Agent chat history | chat messages |
| `structured_data` | JSON/table data | API responses |
| `log_output` | Log/error streams | journalctl, kubectl logs |
| `unknown` | Fallback | everything else |

### 4.5 Algorithm Portfolio

**Phase 1 (local, no LLM):**

| Algorithm | Strategy | Default For |
|-----------|----------|-------------|
| `rtk_style` | Command-specific rules (filter/group/truncate/dedup) for git, cargo, docker, test, lint | shell_command, test_output, build_output |
| `saliency_rank` | TF-IDF + entropy scoring + structural bonus | conversation_history, log_output, unknown |
| `flashrank` | Greedy marginal-utility selection (α·relevance + β·novelty − γ·brevity) under token budget | file_contents, structured_data |

**Phase 2 (LLM-assisted, via `hkask-templates::InferencePort`):**

| Algorithm | Strategy | Default For |
|-----------|----------|-------------|
| `openhands_style` | Rolling window + LLM summarization of middle | conversation_history |
| `opencode_style` | Prune old tool outputs + LLM compact | structured_data |
| `reranker` | Cross-encoder via Okapi/Ollama `/api/rerank` | file_contents, log_output |

**Phase 3 (Cascade composition):**

| Recipe | Pipeline | Use Case |
|--------|----------|----------|
| `tool-basic` | rtk_style → flashrank | Dev CLI commands |
| `tool-deep` | saliency_rank → flashrank → reranker | Long sessions |
| `log-filter` | flashrank → opencode_style | Log/error output |
| `conversation` | openhands_style → flashrank → saliency_rank | Agent history |
| `lightweight` | saliency_rank → flashrank | Low-resource |

### 4.6 Architecture

```rust
trait CondenserAlgorithm: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_for(&self) -> &[ContextCategory];
    fn compress(&self, input: &str, profile: Profile, category: ContextCategory) -> CompressedOutput;
    fn handles(&self, category: ContextCategory) -> bool;
}

struct CondenserEngine {
    algorithms: Vec<Box<dyn CondenserAlgorithm>>,
    registry: AlgorithmRegistry,
    profile: Profile,
    stats: CondenserStats,
}
```

**Server wraps `CondenserEngine` in `Mutex`** (CPU-bound, synchronous compression).

### 4.7 Credentials

None (local engine). Phase 2+ LLM algorithms use `hkask-templates::InferencePort` which uses `HKASK_OKAPI_API_KEY`.

### 4.8 hKask Adaptations from Stack Reference

1. **Replace `stack-condenser` library** with direct implementation in the MCP server crate (simpler, no separate lib crate needed for Phase 1)
2. **Replace `stack-mcp::McpToolServer`** with rmcp `#[tool_router(server_handler)]`
3. **Use `McpToolError`/`McpToolOutput`** for structured results
4. **Use `emit_tool_span()`** for CNS observability (`cns.tool.condenser_compress`, etc.)
5. **Use `run_stdio_server()`** with factory pattern
6. **Phase 1 only initially** — rtk_style, saliency_rank, flashrank algorithms (no LLM dependency)
7. **Phase 2 later** — add `hkask-templates` dependency for LLM-assisted algorithms
8. **Learning loop** — `CondenserStats` tracks per-algorithm quality scores for future auto-selection

---

## Part 5: Build-Out Work Plan

### Phase 1: Web Server (Priority: HIGH)

**Estimated effort:** ~600 LOC

| Step | Task | Dependencies |
|------|------|-------------|
| 1.1 | Update `hkask-mcp-web/Cargo.toml` with `hkask-mcp`, `schemars`, `anyhow`, `chrono`, `futures` | — |
| 1.2 | Define request types: `SearchRequest`, `ExtractRequest`, `BrowseRequest`, `ResearchRequest` | — |
| 1.3 | Define domain types: `SearchResult`, `ExtractedContent`, `BrowseResult`, `SearchQuery`, `WebError` | — |
| 1.4 | Define outbound port traits: `WebSearchProvider`, `WebExtractProvider`, `WebBrowseProvider` | — |
| 1.5 | Implement `BraveProvider` (search) | `HKASK_BRAVE_API_KEY` |
| 1.6 | Implement `FirecrawlProvider` (search + extract + browse) | `HKASK_FIRECRAWL_API_KEY` |
| 1.7 | Implement `RawFetchProvider` (extract fallback, no API key) | — |
| 1.8 | Implement `ProviderPool` with fallback chains | — |
| 1.9 | Implement strategy engine (quick, semantic, extract, deep, fetch) | — |
| 1.10 | Implement `ResponseCache` (TTL + LRU) | — |
| 1.11 | Implement `WebServer` struct with 5 tools | — |
| 1.12 | Wire CNS spans, input validation, keystore credentials | — |
| 1.13 | Update `.env.example` with `HKASK_BRAVE_API_KEY`, `HKASK_FIRECRAWL_API_KEY` | — |
| 1.14 | `cargo clippy -- -D warnings` + `cargo test` | — |

### Phase 2: Scholar Server (Priority: HIGH)

**Estimated effort:** ~800 LOC

| Step | Task | Dependencies |
|------|------|-------------|
| 2.1 | Update `hkask-mcp-scholar/Cargo.toml` with `hkask-mcp`, `schemars`, `anyhow`, `chrono`, `async-trait` | — |
| 2.2 | Define request types for all 12 tools | — |
| 2.3 | Define domain types: `Paper`, `Author`, `Citation`, `Reference`, `GraphSegment`, `GraphNode`, `GraphEdge` | — |
| 2.4 | Define `ScholarApi` outbound port trait | — |
| 2.5 | Implement `HttpScholarApi` driven adapter (S2 Graph API v1) | `HKASK_SEMANTIC_SCHOLAR_API_KEY` |
| 2.6 | Implement SQLite `ScholarStore` (papers, authors, citations tables) | — |
| 2.7 | Implement `PersistingScholarApi` decorator (write-through) | — |
| 2.8 | Implement `scholar_graph_segment` L3 composite (BFS traversal) | — |
| 2.9 | Implement `ScholarServer` struct with 12 tools | — |
| 2.10 | Wire CNS spans, input validation, keystore credentials | — |
| 2.11 | Update `.env.example` with `HKASK_SEMANTIC_SCHOLAR_API_KEY` | — |
| 2.12 | `cargo clippy -- -D warnings` + `cargo test` | — |

### Phase 3: Condenser Server (Priority: MEDIUM)

**Estimated effort:** ~500 LOC (Phase 1 algorithms only)

| Step | Task | Dependencies |
|------|------|-------------|
| 3.1 | Update `hkask-mcp-condenser/Cargo.toml` with `hkask-mcp`, `schemars`, `anyhow` | — |
| 3.2 | Define request types for 5 tools | — |
| 3.3 | Define domain types: `Profile`, `ContextCategory`, `CompressedOutput`, `CondenserStats` | — |
| 3.4 | Define `CondenserAlgorithm` trait | — |
| 3.5 | Implement `RtkStyleAlgorithm` (command-specific rules) | — |
| 3.6 | Implement `SaliencyRankAlgorithm` (TF-IDF + entropy) | — |
| 3.7 | Implement `FlashrankAlgorithm` (marginal-utility selection) | — |
| 3.8 | Implement `AlgorithmRegistry` with auto-selection | — |
| 3.9 | Implement `CondenserEngine` with profile management | — |
| 3.10 | Implement `CondenserServer` struct with 5 tools | — |
| 3.11 | Wire CNS spans, emit_tool_span() | — |
| 3.12 | `cargo clippy -- -D warnings` + `cargo test` | — |

### Phase 4: Advanced Condenser (Deferred)

| Step | Task | Dependencies |
|------|------|-------------|
| 4.1 | Add `hkask-templates` dependency to condenser | hkask-templates InferencePort |
| 4.2 | Implement `OpenHandsStyleAlgorithm` (LLM summarization) | Okapi API |
| 4.3 | Implement `OpenCodeStyleAlgorithm` (LLM compact) | Okapi API |
| 4.4 | Implement `RerankerAlgorithm` (cross-encoder) | Okapi/Ollama |
| 4.5 | Implement cascade recipes (tool-basic, tool-deep, etc.) | — |
| 4.6 | Implement learning loop (promote/demote algorithms) | — |

### Phase 5: Architecture Doc Updates (After All Servers Built)

| Step | Task |
|------|------|
| 5.1 | Update `domain-and-capability.md` §6.1 — change web/scholar/condenser from "⚠️ Stub" to "✅ Complete" with updated LOC counts |
| 5.2 | Update tool surface tables in architecture docs |
| 5.3 | Add hexagonal port inventory entries for WebSearchProvider, ScholarApi, CondenserAlgorithm |
| 5.4 | Add port/adapter ERD entries to `subsystem-erds.md` |
| 5.5 | Update `.env.example` with all new credential templates |

---

## Part 6: Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Plain `rusqlite` for scholar (not `hkask-storage::Database`) | Scholar store doesn't need encryption — research papers are public. Consistent with RSS reader pattern. |
| `rusqlite` for scholar (not `stack-bitemporal-store`/redb) | Redb is not a workspace dependency; rusqlite is already used by RSS reader, hkask-storage, hkask-memory. Reduces dependency surface. |
| Phase 1 condenser: local algorithms only (no LLM) | Avoids `hkask-templates` dependency cycle. Phase 1 algorithms (rtk_style, saliency_rank, flashrank) are pure CPU — no inference needed. |
| Web: Brave + Firecrawl + RawFetch initially (not Exa/Browserbase) | Brave and Firecrawl cover search + extract + browse. Exa and Browserbase are optional enhancements that plug into the same port traits later. |
| Web: in-memory cache (not SQLite) | Web server is stateless by nature. Cache is for deduplication within a session, not durable storage. |
| Scholar: `PersistingScholarApi` decorator pattern | Decouples persistence from API logic. Write failures logged but never surface to caller — the upstream result still returns. |
| All servers: `Mutex<Engine>` for condenser, `Arc<Mutex<Connection>>` for SQLite | Both are CPU-bound synchronous operations. `spawn_blocking` for SQLite, `Mutex` for condenser engine. |
| Credential naming: `HKASK_{SERVICE}_API_KEY` | Consistent with existing pattern (HKASK_GITHUB_TOKEN, HKASK_FMP_API_KEY, etc.) |
