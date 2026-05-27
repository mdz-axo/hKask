mod cache;
mod providers;
mod strip_html;
mod types;

use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, emit_tool_span, resolve_credential,
    run_stdio_server, validate_tool_url,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cache::{cache_key, ResponseCache};
use providers::{
    BraveProvider, BrowserbaseProvider, ExaProvider, FirecrawlProvider, ProviderPool,
    RawFetchProvider, SerapiProvider, TavilyProvider,
};
use types::*;

pub struct WebServer {
    pool: ProviderPool,
    cache: Arc<ResponseCache>,
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
            pool: ProviderPool {
                search_providers,
                extract_providers,
                browse_providers,
            },
            cache: Arc::new(ResponseCache::new(cache_max, Duration::from_secs(cache_ttl))),
        })
    }
}

#[tool_router(server_handler)]
impl WebServer {
    #[tool(description = "Liveness and provider health check")]
    async fn web_ping(&self) -> String {
        McpToolOutput::new(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "search_providers": self.pool.search_provider_kinds(),
            "extract_providers": self.pool.extract_provider_kinds(),
            "browse_providers": self.pool.browse_provider_kinds(),
        }))
        .to_json_string()
    }

    #[tool(description = "Search the web — compound multi-provider with consensus ranking or single-provider fallback")]
    async fn web_search(
        &self,
        Parameters(SearchRequest {
            query,
            num_results,
            include_domains,
            exclude_domains,
            freshness,
            search_type,
            strategy,
        }): Parameters<SearchRequest>,
    ) -> String {
        let start = Instant::now();
        if query.is_empty() {
            return McpToolError::invalid_argument("query must not be empty").to_json_string();
        }

        let strat = strategy
            .as_deref()
            .and_then(|s| s.parse::<SearchStrategy>().ok())
            .unwrap_or(SearchStrategy::Quick);

        let search_query = SearchQuery {
            query: query.clone(),
            num_results: num_results.unwrap_or(10).min(50),
            include_domains: include_domains.unwrap_or_default(),
            exclude_domains: exclude_domains.unwrap_or_default(),
            freshness: freshness.clone(),
            search_type: search_type.clone(),
        };

        let cache_params = serde_json::json!({
            "num_results": search_query.num_results,
            "include_domains": search_query.include_domains,
            "exclude_domains": search_query.exclude_domains,
            "freshness": search_query.freshness,
            "search_type": search_query.search_type,
        });
        let ckey = cache_key(&strat.to_string(), &query, &cache_params);

        if let Some(cached) = self.cache.get(&ckey).await {
            return McpToolOutput::with_timing(cached, start).to_json_string();
        }

        match strat {
            SearchStrategy::Quick => {
                let primary = "brave";
                match self.pool.search_with_fallback(&search_query, primary).await {
                    Ok(results) => {
                        emit_tool_span("web_search", "ok", start.elapsed().as_millis() as u64, None);
                        let output = serde_json::json!({
                            "query": query,
                            "strategy": strat.to_string(),
                            "mode": "fallback",
                            "results": results,
                            "count": results.len(),
                        });
                        self.cache.insert(ckey, output.clone()).await;
                        McpToolOutput::with_timing(output, start).to_json_string()
                    }
                    Err(e) => {
                        emit_tool_span("web_search", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                        McpToolError::from(e).to_json_string()
                    }
                }
            }
            SearchStrategy::Semantic
            | SearchStrategy::News
            | SearchStrategy::Research
            | SearchStrategy::Deep => {
                let compound = self.pool.search_compound(&search_query, strat).await;
                emit_tool_span("web_search", "ok", start.elapsed().as_millis() as u64, None);
                let mut output = serde_json::to_value(&compound).unwrap_or_else(|_| serde_json::json!({}));
                if let Some(obj) = output.as_object_mut() {
                    obj.insert("mode".to_string(), serde_json::json!("compound"));
                    obj.insert("count".to_string(), serde_json::json!(compound.results.len()));
                }
                self.cache.insert(ckey, output.clone()).await;
                McpToolOutput::with_timing(output, start).to_json_string()
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

        let cache_params = serde_json::json!({ "format": fmt, "main_content_only": opts.main_content_only });
        let ckey = cache_key("extract", &url, &cache_params);

        if let Some(cached) = self.cache.get(&ckey).await {
            return McpToolOutput::with_timing(cached, start).to_json_string();
        }

        match self.pool.extract_with_fallback(&url, &opts).await {
            Ok(result) => {
                emit_tool_span("web_extract", "ok", start.elapsed().as_millis() as u64, None);
                let output = serde_json::json!({
                    "url": result.url,
                    "format": result.format,
                    "content": result.content,
                    "metadata": result.metadata,
                });
                self.cache.insert(ckey, output.clone()).await;
                McpToolOutput::with_timing(output, start).to_json_string()
            }
            Err(e) => {
                emit_tool_span("web_extract", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
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
        if let Err(e) = validate_tool_url(&url) {
            return e.to_json_string();
        }

        let instr = instruction.unwrap_or_else(|| "Extract page content".to_string());
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(30)).min(Duration::from_secs(120));

        match self.pool.browse_with_fallback(&url, &instr, timeout).await {
            Ok(result) => {
                emit_tool_span("web_browse", "ok", start.elapsed().as_millis() as u64, None);
                McpToolOutput::with_timing(
                    serde_json::json!({
                        "url": result.url,
                        "content": result.content,
                        "instruction": result.instruction,
                        "actions_taken": result.actions_taken,
                    }),
                    start,
                )
                .to_json_string()
            }
            Err(e) => {
                emit_tool_span("web_browse", "error", start.elapsed().as_millis() as u64, Some(&e.kind()));
                McpToolError::from(e).to_json_string()
            }
        }
    }

    #[tool(description = "Deep multi-step research cascade: compound search → extract → cross-ref")]
    async fn web_research(
        &self,
        Parameters(ResearchRequest {
            query,
            max_pages,
            include_domains,
            exclude_domains,
            freshness,
        }): Parameters<ResearchRequest>,
    ) -> String {
        let start = Instant::now();
        if query.is_empty() {
            return McpToolError::invalid_argument("query must not be empty").to_json_string();
        }

        let max_pages = max_pages.unwrap_or(5).min(10) as usize;

        let search_query = SearchQuery {
            query: query.clone(),
            num_results: max_pages as u32 * 3,
            include_domains: include_domains.unwrap_or_default(),
            exclude_domains: exclude_domains.unwrap_or_default(),
            freshness: freshness.clone(),
            search_type: None,
        };

        let compound = self.pool.search_compound(&search_query, SearchStrategy::Research).await;

        let mut pages = Vec::new();
        let mut urls_seen = std::collections::HashSet::new();

        for result in compound.results.iter().take(max_pages) {
            if urls_seen.contains(&result.url) {
                continue;
            }
            urls_seen.insert(result.url.clone());

            if validate_tool_url(&result.url).is_err() {
                continue;
            }

            let opts = ExtractOptions {
                format: "markdown".to_string(),
                json_prompt: None,
                json_schema: None,
                main_content_only: true,
                wait_for_ms: 0,
            };

            match self.pool.extract_with_fallback(&result.url, &opts).await {
                Ok(extracted) => {
                    pages.push(serde_json::json!({
                        "url": extracted.url,
                        "title": result.title.clone(),
                        "content_length": extracted.content.len(),
                        "content_preview": truncate_str(&extracted.content, 2000),
                    }));
                }
                Err(e) => {
                    tracing::warn!(url = %result.url, error = %e, "Extract failed during research cascade");
                }
            }
        }

        emit_tool_span("web_research", "ok", start.elapsed().as_millis() as u64, None);
        McpToolOutput::with_timing(
            serde_json::json!({
                "query": query,
                "mode": "compound",
                "search_providers_queried": compound.providers_queried,
                "search_providers_succeeded": compound.providers_succeeded,
                "search_providers_failed": compound.providers_failed,
                "pages_extracted": pages.len(),
                "pages": pages,
            }),
            start,
        )
        .to_json_string()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let brave_key = resolve_credential("HKASK_BRAVE_API_KEY").ok();
    let firecrawl_key = resolve_credential("HKASK_FIRECRAWL_API_KEY").ok();
    let tavily_key = resolve_credential("HKASK_TAVILY_API_KEY").ok();
    let serpapi_key = resolve_credential("HKASK_SERPAPI_API_KEY").ok();
    let exa_key = resolve_credential("HKASK_EXA_API_KEY").ok();
    let browserbase_key = resolve_credential("HKASK_BROWSERBASE_API_KEY").ok();

    let mut required_creds = Vec::new();
    if brave_key.is_none() && firecrawl_key.is_none() && tavily_key.is_none()
        && serpapi_key.is_none() && exa_key.is_none()
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
