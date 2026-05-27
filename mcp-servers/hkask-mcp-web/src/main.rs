mod cache;
pub mod providers;
pub mod strip_html;
pub mod types;

use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, emit_tool_span, resolve_credential,
    run_stdio_server, validate_tool_url,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cache::{ResponseCache, cache_key};
use providers::{
    BraveProvider, BrowserbaseProvider, ExaProvider, FirecrawlProvider, ProviderPool,
    RawFetchProvider, SerapiProvider, TavilyProvider, WebSearchPort,
};
use types::*;

pub struct WebServer {
    pool: Arc<dyn WebSearchPort>,
    cache: Arc<ResponseCache>,
    rate_limiter: RateLimiter,
}

impl WebServer {
    fn new(
        brave_api_key: Option<String>,
        firecrawl_api_key: Option<String>,
        tavily_api_key: Option<String>,
        serpapi_api_key: Option<String>,
        exa_api_key: Option<String>,
        browserbase_api_key: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let cache_ttl = std::env::var("HKASK_WEB_CACHE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(|s| s.min(MAX_CACHE_TTL_SECS))
            .unwrap_or(DEFAULT_CACHE_TTL_SECS);
        let cache_max = std::env::var("HKASK_WEB_CACHE_MAX_ENTRIES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .map(|s| s.min(MAX_CACHE_MAX_ENTRIES))
            .unwrap_or(DEFAULT_CACHE_MAX_ENTRIES);

        let mut search_providers: Vec<Box<dyn providers::WebSearchProvider>> = Vec::new();
        let mut extract_providers: Vec<Box<dyn providers::WebExtractProvider>> = Vec::new();
        let mut browse_providers: Vec<Box<dyn providers::WebBrowseProvider>> = Vec::new();

        let exa_provider = exa_api_key
            .as_ref()
            .map(|key| ExaProvider::new(key.clone()));

        if let Some(ref key) = brave_api_key {
            search_providers.push(Box::new(BraveProvider::new(key.clone())));
        }
        if let Some(ref key) = firecrawl_api_key {
            search_providers.push(Box::new(FirecrawlProvider::new(Some(key.clone()))));
            extract_providers.push(Box::new(FirecrawlProvider::new(Some(key.clone()))));
            browse_providers.push(Box::new(FirecrawlProvider::new(Some(key.clone()))));
        }
        if let Some(ref key) = tavily_api_key {
            search_providers.push(Box::new(TavilyProvider::new(key.clone())));
        }
        if let Some(ref key) = serpapi_api_key {
            search_providers.push(Box::new(SerapiProvider::new(key.clone())));
        }
        if let Some(ref key) = exa_api_key {
            search_providers.push(Box::new(ExaProvider::new(key.clone())));
        }
        if let Some(ref key) = browserbase_api_key {
            browse_providers.push(Box::new(BrowserbaseProvider::new(key.clone())));
        }

        extract_providers.push(Box::new(RawFetchProvider::new()));

        if search_providers.is_empty() {
            anyhow::bail!(
                "No search providers configured. Set at least one of: \
                 HKASK_BRAVE_API_KEY, HKASK_FIRECRAWL_API_KEY, HKASK_TAVILY_API_KEY, \
                 HKASK_SERPAPI_API_KEY, HKASK_EXA_API_KEY"
            );
        }

        Ok(Self {
            pool: Arc::new(ProviderPool::new(
                search_providers,
                extract_providers,
                browse_providers,
                exa_provider,
                Arc::new(EnvCredentialResolver),
            )),
            cache: Arc::new(ResponseCache::new(
                cache_max,
                Duration::from_secs(cache_ttl),
            )),
            rate_limiter: RateLimiter::new(RATE_LIMIT_MAX_REQUESTS, RATE_LIMIT_WINDOW_SECS),
        })
    }
}

#[tool_router(server_handler)]
impl WebServer {
    #[tool(description = "Liveness and provider health check")]
    async fn web_ping(&self) -> String {
        // Task 9: Rate limit web_ping to prevent DoS amplification
        if let Err(e) = self.rate_limiter.check("web_ping") {
            // Task 11: Return PingOutput with rate_limited status instead of McpToolError
            let output = PingOutput {
                status: "rate_limited".to_string(),
                version: SERVER_VERSION.to_string(),
                providers: vec![],
            };
            tracing::warn!(
                target: "cns.web",
                message = %e.message,
                "web_ping rate limited"
            );
            return McpToolOutput::new(serde_json::to_value(&output).unwrap_or_default())
                .to_json_string();
        }

        let providers = self.pool.health_check().await;
        let output = PingOutput {
            status: "ok".to_string(),
            version: SERVER_VERSION.to_string(),
            providers,
        };
        McpToolOutput::new(serde_json::to_value(&output).unwrap_or_default()).to_json_string()
    }

    #[tool(
        description = "Search the web with RRF fusion across providers. Strategy selects providers: quick (single keyword), web (all), news (news-capable), deep (all + rerank)"
    )]
    async fn web_search(&self, Parameters(req): Parameters<SearchRequest>) -> String {
        let start = Instant::now();

        if let Err(e) = self.rate_limiter.check("web_search") {
            return e.to_json_string();
        }

        // Task 5: Input validation bounds
        if req.query.is_empty() {
            return McpToolError::invalid_argument("query must not be empty").to_json_string();
        }
        if req.query.len() > MAX_QUERY_LENGTH {
            return McpToolError::invalid_argument(format!(
                "query exceeds maximum length of {} characters",
                MAX_QUERY_LENGTH
            ))
            .to_json_string();
        }

        let strat = req
            .strategy
            .as_deref()
            .and_then(|s| s.parse::<SearchStrategy>().ok())
            .unwrap_or(SearchStrategy::Quick);

        let num_results = req.num_results.unwrap_or(10).min(50);

        // Task 9: Normalize freshness from free-form string to canonical enum
        let freshness = req
            .freshness
            .as_deref()
            .and_then(|f| f.parse::<Freshness>().ok());

        let fingerprint = self.pool.provider_fingerprint();
        let ckey = cache_key(
            &strat.to_string(),
            &req.query,
            &serde_json::json!({ "num_results": num_results, "freshness": freshness }),
            &fingerprint,
        );

        if let Some(cached) = self.cache.get(&ckey).await {
            return McpToolOutput::with_timing(cached, start).to_json_string();
        }

        let search_query = SearchQuery {
            query: req.query.clone(),
            num_results,
            include_domains: req.include_domains.unwrap_or_default(),
            exclude_domains: req.exclude_domains.unwrap_or_default(),
            freshness, // Task 3: Type-safe Freshness enum flows to providers
            depth: SearchDepth::Basic, // Will be overridden by ProviderPool based on strategy
        };

        // Task 13: CapabilityContext passed as None for now
        let mut compound = match self.pool.search(&search_query, strat, None).await {
            Ok(c) => c,
            Err(e) => {
                emit_tool_span(
                    "web_search",
                    "error",
                    start.elapsed().as_millis() as u64,
                    Some(&e.kind()),
                );
                return McpToolOutput::with_timing(
                    serde_json::json!({
                        "error": e.to_string(),
                        "strategy": strat.to_string(),
                    }),
                    start,
                )
                .to_json_string();
            }
        };

        compound.results.truncate(num_results as usize);

        // Build DTOs: SearchOutput for MCP client, SearchMetadata for CNS
        let search_output = SearchOutput {
            query: compound.query.clone(),
            strategy: compound.strategy.clone(),
            results: compound
                .results
                .iter()
                .map(SearchResultOutput::from)
                .collect(),
            answer_box: compound.answer_box.clone(),
            related_questions: compound.related_questions.clone(),
            count: compound.results.len(),
        };

        let metadata = SearchMetadata::from(&compound);
        emit_tool_span("web_search", "ok", start.elapsed().as_millis() as u64, None);
        // Emit metadata to CNS (not to MCP client)
        tracing::info!(
            target: "cns.web",
            strategy = %metadata.strategy,
            providers_queried = ?metadata.providers_queried,
            providers_succeeded = ?metadata.providers_succeeded,
            providers_failed = ?metadata.providers_failed,
            total_before_dedup = metadata.total_before_dedup,
            duplicates_removed = metadata.duplicates_removed,
            top_rrf_scores = ?metadata.top_rrf_scores,
            "CNS web_search metadata"
        );

        let output = serde_json::to_value(&search_output)
            .unwrap_or_else(|_| serde_json::json!({ "error": "serialization failed" }));

        self.cache.insert(ckey, output.clone()).await;
        McpToolOutput::with_timing(output, start).to_json_string()
    }

    #[tool(description = "Find pages similar to a given URL using Exa findSimilar")]
    async fn web_find_similar(
        &self,
        Parameters(FindSimilarRequest { url, num_results }): Parameters<FindSimilarRequest>,
    ) -> String {
        let start = Instant::now();

        if let Err(e) = self.rate_limiter.check("web_find_similar") {
            return e.to_json_string();
        }

        // URL validation — prevent SSRF-adjacent probing of internal URLs
        if let Err(e) = validate_tool_url(&url) {
            return e.to_json_string();
        }

        let num = num_results.unwrap_or(5).min(20);

        // Task 13: CapabilityContext passed as None for now
        match self.pool.find_similar(&url, num, None).await {
            Ok(output) => {
                let results: Vec<FindSimilarResultOutput> = output
                    .results
                    .into_iter()
                    .map(|r| {
                        let key = r.url.to_lowercase();
                        FindSimilarResultOutput {
                            title: r.title,
                            url: r.url,
                            description: r.description,
                            source: r.source,
                            published: r.published,
                            semantic_score: output.semantic_scores.get(&key).copied(),
                            content_preview: output.content_previews.get(&key).cloned(),
                        }
                    })
                    .collect();

                let fs_output = FindSimilarOutput {
                    source_url: url,
                    count: results.len(),
                    results,
                };

                emit_tool_span(
                    "web_find_similar",
                    "ok",
                    start.elapsed().as_millis() as u64,
                    None,
                );

                McpToolOutput::with_timing(
                    serde_json::to_value(&fs_output)
                        .unwrap_or_else(|_| serde_json::json!({ "error": "serialization failed" })),
                    start,
                )
                .to_json_string()
            }
            Err(e) => {
                emit_tool_span(
                    "web_find_similar",
                    "error",
                    start.elapsed().as_millis() as u64,
                    Some(&e.kind()),
                );
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Extract content from a URL into markdown or structured JSON")]
    async fn web_extract(
        &self,
        Parameters(ExtractRequest {
            url,
            format,
            json_prompt,
            json_schema,
            main_content_only,
            wait_for_ms,
        }): Parameters<ExtractRequest>,
    ) -> String {
        let start = Instant::now();

        if let Err(e) = self.rate_limiter.check("web_extract") {
            return e.to_json_string();
        }

        // Task 5: Input validation bounds
        if url.len() > MAX_URL_LENGTH {
            return McpToolError::invalid_argument(format!(
                "url exceeds maximum length of {} characters",
                MAX_URL_LENGTH
            ))
            .to_json_string();
        }
        if let Some(ref prompt) = json_prompt
            && prompt.len() > MAX_JSON_PROMPT_LENGTH
        {
            return McpToolError::invalid_argument(format!(
                "json_prompt exceeds maximum length of {} characters",
                MAX_JSON_PROMPT_LENGTH
            ))
            .to_json_string();
        }
        if let Some(ref schema) = json_schema
            && let Ok(bytes) = serde_json::to_string(schema)
            && bytes.len() > MAX_JSON_SCHEMA_BYTES
        {
            return McpToolError::invalid_argument(format!(
                "json_schema exceeds maximum size of {} bytes",
                MAX_JSON_SCHEMA_BYTES
            ))
            .to_json_string();
        }

        if let Err(e) = validate_tool_url(&url) {
            return e.to_json_string();
        }

        let fmt = format.unwrap_or_else(|| "markdown".to_string());
        let opts = ExtractOptions {
            format: fmt.clone(),
            json_prompt,
            json_schema,
            main_content_only: main_content_only.unwrap_or(true),
            wait_for_ms: wait_for_ms.unwrap_or(0),
        };

        let fingerprint = self.pool.provider_fingerprint();
        let cache_params =
            serde_json::json!({ "format": fmt, "main_content_only": opts.main_content_only });
        let ckey = cache_key("extract", &url, &cache_params, &fingerprint);

        if let Some(cached) = self.cache.get(&ckey).await {
            return McpToolOutput::with_timing(cached, start).to_json_string();
        }

        match self.pool.extract(&url, &opts, None).await {
            Ok(result) => {
                emit_tool_span(
                    "web_extract",
                    "ok",
                    start.elapsed().as_millis() as u64,
                    None,
                );
                let output = ExtractOutput {
                    url: result.url,
                    format: result.format,
                    content: result.content,
                    metadata: result.metadata,
                };
                let json = serde_json::to_value(&output)
                    .unwrap_or_else(|_| serde_json::json!({ "error": "serialization failed" }));
                self.cache.insert(ckey, json.clone()).await;
                McpToolOutput::with_timing(json, start).to_json_string()
            }
            Err(e) => {
                emit_tool_span(
                    "web_extract",
                    "error",
                    start.elapsed().as_millis() as u64,
                    Some(&e.kind()),
                );
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Interactive browsing of JS-heavy pages via headless browser")]
    async fn web_browse(
        &self,
        Parameters(BrowseRequest {
            url,
            instruction,
            timeout_secs,
        }): Parameters<BrowseRequest>,
    ) -> String {
        let start = Instant::now();

        if let Err(e) = self.rate_limiter.check("web_browse") {
            return e.to_json_string();
        }

        // Task 5: Input validation bounds
        if url.len() > MAX_URL_LENGTH {
            return McpToolError::invalid_argument(format!(
                "url exceeds maximum length of {} characters",
                MAX_URL_LENGTH
            ))
            .to_json_string();
        }
        if let Some(ref instr) = instruction
            && instr.len() > MAX_INSTRUCTION_LENGTH
        {
            return McpToolError::invalid_argument(format!(
                "instruction exceeds maximum length of {} characters",
                MAX_INSTRUCTION_LENGTH
            ))
            .to_json_string();
        }

        if let Err(e) = validate_tool_url(&url) {
            return e.to_json_string();
        }

        let instr = instruction.unwrap_or_else(|| "Extract page content".to_string());
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(30)).min(Duration::from_secs(120));

        // Task 13: CapabilityContext passed as None for now
        match self.pool.browse(&url, &instr, timeout, None).await {
            Ok(result) => {
                emit_tool_span("web_browse", "ok", start.elapsed().as_millis() as u64, None);
                let output = BrowseOutput {
                    url: result.url,
                    content: result.content,
                    instruction: result.instruction,
                    actions_taken: result.actions_taken,
                };
                McpToolOutput::with_timing(
                    serde_json::to_value(&output)
                        .unwrap_or_else(|_| serde_json::json!({ "error": "serialization failed" })),
                    start,
                )
                .to_json_string()
            }
            Err(e) => {
                emit_tool_span(
                    "web_browse",
                    "error",
                    start.elapsed().as_millis() as u64,
                    Some(&e.kind()),
                );
                McpToolError::from(e).to_json_string()
            }
        }
    }
}

fn load_dotenv() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let paths = [cwd.join(".env")];
    if let Some(parent) = cwd.parent() {
        let more = [parent.join(".env")];
        for path in paths.iter().chain(more.iter()) {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        if !key.is_empty() && !value.is_empty() && std::env::var(key).is_err() {
                            // SAFETY: set_var is called only at startup before any concurrent access
                            unsafe {
                                std::env::set_var(key, value);
                            }
                        }
                    }
                }
                return;
            }
        }
    } else {
        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        if !key.is_empty() && !value.is_empty() && std::env::var(key).is_err() {
                            // SAFETY: set_var is called only at startup before any concurrent access
                            unsafe {
                                std::env::set_var(key, value);
                            }
                        }
                    }
                }
                return;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_dotenv();

    let brave_key = resolve_credential("HKASK_BRAVE_API_KEY").ok();
    let firecrawl_key = resolve_credential("HKASK_FIRECRAWL_API_KEY").ok();
    let tavily_key = resolve_credential("HKASK_TAVILY_API_KEY").ok();
    let serpapi_key = resolve_credential("HKASK_SERPAPI_API_KEY").ok();
    let exa_key = resolve_credential("HKASK_EXA_API_KEY").ok();
    let browserbase_key = resolve_credential("HKASK_BROWSERBASE_API_KEY").ok();

    let mut required_creds = Vec::new();
    if brave_key.is_none()
        && firecrawl_key.is_none()
        && tavily_key.is_none()
        && serpapi_key.is_none()
        && exa_key.is_none()
    {
        required_creds.push(CredentialRequirement::required(
            "HKASK_BRAVE_API_KEY",
            "At least one search provider API key is required \
             (HKASK_BRAVE_API_KEY, HKASK_TAVILY_API_KEY, HKASK_SERPAPI_API_KEY, \
              HKASK_FIRECRAWL_API_KEY, or HKASK_EXA_API_KEY)",
        ));
    }

    run_stdio_server(
        "hkask-mcp-web",
        SERVER_VERSION,
        || {
            WebServer::new(
                brave_key.clone(),
                firecrawl_key.clone(),
                tavily_key.clone(),
                serpapi_key.clone(),
                exa_key.clone(),
                browserbase_key.clone(),
            )
        },
        required_creds,
    )
    .await
}

#[cfg(test)]
mod live_tests {
    use super::*;

    fn load_env() {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut paths = vec![cwd.join(".env")];
        if let Some(parent) = cwd.parent() {
            paths.push(parent.join(".env"));
            if let Some(grandparent) = parent.parent() {
                paths.push(grandparent.join(".env"));
                if let Some(gg) = grandparent.parent() {
                    paths.push(gg.join(".env"));
                }
            }
        }
        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        if !key.is_empty() && !value.is_empty() && std::env::var(key).is_err() {
                            unsafe {
                                std::env::set_var(key, value);
                            }
                        }
                    }
                }
                eprintln!("Loaded .env from {}", path.display());
                break;
            }
        }
    }

    fn make_server() -> WebServer {
        load_env();
        WebServer::new(
            std::env::var("HKASK_BRAVE_API_KEY").ok(),
            std::env::var("HKASK_FIRECRAWL_API_KEY").ok(),
            std::env::var("HKASK_TAVILY_API_KEY").ok(),
            std::env::var("HKASK_SERPAPI_API_KEY").ok(),
            std::env::var("HKASK_EXA_API_KEY").ok(),
            std::env::var("HKASK_BROWSERBASE_API_KEY").ok(),
        )
        .unwrap()
    }

    #[tokio::test]
    #[ignore]
    async fn live_brave_search() {
        let server = make_server();
        let query = SearchQuery {
            query: "Rust programming language".into(),
            num_results: 3,
            include_domains: vec![],
            exclude_domains: vec![],
            freshness: None,
            depth: SearchDepth::Basic,
        };
        let result = server
            .pool
            .search(&query, SearchStrategy::Quick, None)
            .await;
        assert!(result.is_ok(), "Search failed: {:?}", result.err());
        let compound = result.unwrap();
        assert!(!compound.results.is_empty(), "Search returned no results");
    }

    #[tokio::test]
    #[ignore]
    async fn live_exa_search() {
        let server = make_server();
        let query = SearchQuery {
            query: "Rust programming language".into(),
            num_results: 3,
            include_domains: vec![],
            exclude_domains: vec![],
            freshness: None,
            depth: SearchDepth::Basic,
        };
        let result = server.pool.search(&query, SearchStrategy::Web, None).await;
        assert!(result.is_ok(), "Search failed: {:?}", result.err());
        let compound = result.unwrap();
        assert!(!compound.results.is_empty(), "Search returned no results");
    }

    #[tokio::test]
    #[ignore]
    async fn live_exa_find_similar() {
        let server = make_server();
        let result = server
            .pool
            .find_similar("https://www.rust-lang.org", 3, None)
            .await;
        assert!(result.is_ok(), "Exa findSimilar failed: {:?}", result.err());
        let output = result.unwrap();
        assert!(
            !output.results.is_empty(),
            "Exa findSimilar returned no results"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_compound_search() {
        let server = make_server();
        let query = SearchQuery {
            query: "Rust programming language".into(),
            num_results: 5,
            include_domains: vec![],
            exclude_domains: vec![],
            freshness: None,
            depth: SearchDepth::Basic,
        };
        let result = server.pool.search(&query, SearchStrategy::Web, None).await;
        assert!(result.is_ok(), "Compound search failed: {:?}", result.err());
        let compound = result.unwrap();
        assert!(
            !compound.results.is_empty(),
            "Compound search returned no results"
        );
        assert!(
            !compound.providers_succeeded.is_empty(),
            "No providers succeeded"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_extract() {
        let server = make_server();
        let opts = ExtractOptions {
            format: "markdown".into(),
            json_prompt: None,
            json_schema: None,
            main_content_only: true,
            wait_for_ms: 0,
        };
        let result = server
            .pool
            .extract("https://example.com", &opts, None)
            .await;
        assert!(result.is_ok(), "Extract failed: {:?}", result.err());
        let content = result.unwrap();
        assert!(!content.content.is_empty(), "Extracted content is empty");
    }

    #[tokio::test]
    #[ignore]
    async fn live_web_search_quick() {
        let server = make_server();
        let req = SearchRequest {
            query: "Rust programming language".into(),
            num_results: Some(3),
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: Some("quick".into()),
        };
        let output = server.web_search(Parameters(req)).await;
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_default();
        assert!(
            parsed.get("error").is_none(),
            "Search error: {:?}",
            parsed.get("error")
        );
        assert!(
            parsed
                .get("content")
                .and_then(|r| r.get("results"))
                .is_some(),
            "No results in output"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_web_search_deep() {
        let server = make_server();
        let req = SearchRequest {
            query: "Rust web frameworks comparison".into(),
            num_results: Some(3),
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: Some("deep".into()),
        };
        let output = server.web_search(Parameters(req)).await;
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_default();
        assert!(
            parsed.get("error").is_none(),
            "Search error: {:?}",
            parsed.get("error")
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_web_find_similar() {
        let server = make_server();
        let req = FindSimilarRequest {
            url: "https://www.rust-lang.org".into(),
            num_results: Some(3),
        };
        let output = server.web_find_similar(Parameters(req)).await;
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_default();
        assert!(
            parsed.get("error").is_none(),
            "FindSimilar error: {:?}",
            parsed.get("error")
        );
    }
}
