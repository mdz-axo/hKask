//! hKask MCP Web — Unified web search, content extraction, and interactive browsing
//!
//! Provides multi-provider routing across Brave (search) and Firecrawl (search/extract/browse)
//! with a RawFetch fallback for basic extraction. Strategy engine supports quick, semantic,
//! extract, deep, and fetch modes. In-memory cache with TTL-based eviction.

use async_trait::async_trait;
use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, emit_tool_span,
    resolve_credential, run_stdio_server, validate_tool_url,
};
use hkask_types::McpErrorKind;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const BRAVE_API_BASE: &str = "https://api.search.brave.com/res/v1";
const FIRECRAWL_API_BASE: &str = "https://api.firecrawl.dev/v1";

const DEFAULT_CACHE_TTL_SECS: u64 = 300;
const MAX_CACHE_TTL_SECS: u64 = 7200;
const DEFAULT_CACHE_MAX_ENTRIES: usize = 50;
const MAX_CACHE_MAX_ENTRIES: usize = 200;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query: String,
    pub num_results: Option<u32>,
    pub include_domains: Option<Vec<String>>,
    pub exclude_domains: Option<Vec<String>>,
    pub freshness: Option<String>,
    pub search_type: Option<String>,
    pub strategy: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractRequest {
    pub url: String,
    pub format: Option<String>,
    pub json_prompt: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub main_content_only: Option<bool>,
    pub wait_for_ms: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BrowseRequest {
    pub url: String,
    pub instruction: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResearchRequest {
    pub query: String,
    pub max_pages: Option<u32>,
    pub include_domains: Option<Vec<String>>,
    pub exclude_domains: Option<Vec<String>>,
    pub freshness: Option<String>,
}

// =============================================================================
// Domain types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub published: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub url: String,
    pub content: String,
    pub format: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseResult {
    pub url: String,
    pub content: String,
    pub instruction: Option<String>,
    pub actions_taken: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub num_results: u32,
    pub include_domains: Vec<String>,
    pub exclude_domains: Vec<String>,
    pub freshness: Option<String>,
    pub search_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub format: String,
    pub json_prompt: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub main_content_only: bool,
    pub wait_for_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("Bad arguments: {0}")]
    BadArgs(String),
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("No provider available")]
    NoProvider,
    #[error("Cascade failed: {0}")]
    CascadeFailed(String),
}

impl WebError {
    pub fn kind(&self) -> McpErrorKind {
        match self {
            WebError::BadArgs(_) => McpErrorKind::InvalidArgument,
            WebError::ProviderUnavailable(_) => McpErrorKind::Unavailable,
            WebError::ProviderError(_) => McpErrorKind::Internal,
            WebError::RateLimited(_) => McpErrorKind::RateLimited,
            WebError::NoProvider => McpErrorKind::Unavailable,
            WebError::CascadeFailed(_) => McpErrorKind::Internal,
        }
    }
}

impl From<WebError> for McpToolError {
    fn from(e: WebError) -> Self {
        McpToolError::new(e.kind(), e.to_string())
    }
}

// =============================================================================
// Outbound port traits
// =============================================================================

#[async_trait]
trait WebSearchProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
trait WebExtractProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn extract(&self, url: &str, opts: &ExtractOptions)
    -> Result<ExtractedContent, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
trait WebBrowseProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn browse(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
    ) -> Result<BrowseResult, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

// =============================================================================
// Brave provider
// =============================================================================

struct BraveProvider {
    client: reqwest::Client,
    api_key: String,
}

impl BraveProvider {
    fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }
}

#[async_trait]
impl WebSearchProvider for BraveProvider {
    fn kind(&self) -> &str {
        "brave"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, WebError> {
        let mut params: Vec<(&str, String)> = vec![
            ("q", query.query.clone()),
            ("count", query.num_results.to_string()),
        ];
        if let Some(ref freshness) = query.freshness {
            params.push(("freshness", freshness.clone()));
        }
        if let Some(ref search_type) = query.search_type {
            params.push(("search_type", search_type.clone()));
        }

        let resp = self
            .client
            .get(format!("{BRAVE_API_BASE}/web/search"))
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Brave request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => WebError::ProviderUnavailable(format!("Brave auth error: {status}")),
                429 => WebError::RateLimited(format!("Brave rate limited: {status}")),
                _ => WebError::ProviderError(format!(
                    "Brave API error {status}: {}",
                    body.chars().take(200).collect::<String>()
                )),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| WebError::ProviderError(format!("Failed to parse Brave response: {e}")))?;

        let results = parsed["web"]["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: item["url"].as_str()?.to_string(),
                            description: item["description"].as_str().map(|s| s.to_string()),
                            source: item["source"].as_str().map(|s| s.to_string()),
                            published: item["age"].as_str().map(|s| s.to_string()),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn health(&self) -> Result<(), WebError> {
        let resp = self
            .client
            .get(format!("{BRAVE_API_BASE}/web/search"))
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&[("q", "test"), ("count", "1")])
            .send()
            .await
            .map_err(|e| {
                WebError::ProviderUnavailable(format!("Brave health check failed: {e}"))
            })?;

        if resp.status().is_success() || resp.status().as_u16() == 429 {
            Ok(())
        } else {
            Err(WebError::ProviderUnavailable(format!(
                "Brave health check returned {}",
                resp.status()
            )))
        }
    }
}

// =============================================================================
// Firecrawl provider
// =============================================================================

struct FirecrawlProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl FirecrawlProvider {
    fn new(api_key: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }

    fn auth_header(&self) -> Result<String, WebError> {
        self.api_key
            .as_ref()
            .map(|k| format!("Bearer {k}"))
            .ok_or(WebError::NoProvider)
    }
}

#[async_trait]
impl WebSearchProvider for FirecrawlProvider {
    fn kind(&self) -> &str {
        "firecrawl"
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, WebError> {
        let auth = self.auth_header()?;
        let payload = serde_json::json!({
            "query": query.query,
            "limit": query.num_results,
        });

        let resp = self
            .client
            .post(format!("{FIRECRAWL_API_BASE}/search"))
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Firecrawl request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => {
                    WebError::ProviderUnavailable(format!("Firecrawl auth error: {status}"))
                }
                429 => WebError::RateLimited(format!("Firecrawl rate limited: {status}")),
                _ => WebError::ProviderError(format!("Firecrawl API error {status}")),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Firecrawl response: {e}"))
        })?;

        let results = parsed["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: item["url"].as_str()?.to_string(),
                            description: item["description"]
                                .as_str()
                                .or_else(|| item["snippet"].as_str())
                                .map(|s| s.to_string()),
                            source: None,
                            published: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn health(&self) -> Result<(), WebError> {
        if self.api_key.is_none() {
            return Err(WebError::NoProvider);
        }
        Ok(())
    }
}

#[async_trait]
impl WebExtractProvider for FirecrawlProvider {
    fn kind(&self) -> &str {
        "firecrawl"
    }

    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<ExtractedContent, WebError> {
        let auth = self.auth_header()?;
        let mut payload = serde_json::json!({
            "url": url,
        });

        match opts.format.as_str() {
            "json" => {
                payload["formats"] = serde_json::json!(["json"]);
                if let Some(ref prompt) = opts.json_prompt {
                    payload["jsonOptions"] = serde_json::json!({ "prompt": prompt });
                }
            }
            _ => {
                payload["formats"] = serde_json::json!(["markdown"]);
            }
        }

        if opts.main_content_only {
            payload["onlyMainContent"] = serde_json::json!(true);
        }

        if opts.wait_for_ms > 0 {
            payload["waitFor"] = serde_json::json!(opts.wait_for_ms);
        }

        let resp = self
            .client
            .post(format!("{FIRECRAWL_API_BASE}/scrape"))
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Firecrawl extract failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(WebError::ProviderError(format!(
                "Firecrawl extract error {status}: {}",
                body.chars().take(200).collect::<String>()
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Firecrawl extract response: {e}"))
        })?;

        let content = if opts.format == "json" {
            parsed["data"]["json"].to_string()
        } else {
            parsed["data"]["markdown"]
                .as_str()
                .unwrap_or("")
                .to_string()
        };

        let metadata = parsed["data"]["metadata"]
            .as_object()
            .map(|m| serde_json::Value::Object(m.clone()));

        Ok(ExtractedContent {
            url: url.to_string(),
            content,
            format: opts.format.clone(),
            metadata,
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        if self.api_key.is_none() {
            return Err(WebError::NoProvider);
        }
        Ok(())
    }
}

#[async_trait]
impl WebBrowseProvider for FirecrawlProvider {
    fn kind(&self) -> &str {
        "firecrawl"
    }

    async fn browse(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
    ) -> Result<BrowseResult, WebError> {
        let auth = self.auth_header()?;
        let payload = serde_json::json!({
            "url": url,
            "formats": ["markdown"],
            "actions": [{
                "type": "wait",
                "milliseconds": 2000u64,
            }],
        });

        let resp = self
            .client
            .post(format!("{FIRECRAWL_API_BASE}/scrape"))
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Firecrawl browse failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(WebError::ProviderError(format!(
                "Firecrawl browse error {status}"
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Firecrawl browse response: {e}"))
        })?;

        let content = parsed["data"]["markdown"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(BrowseResult {
            url: url.to_string(),
            content,
            instruction: Some(instruction.to_string()),
            actions_taken: vec!["scrape".to_string()],
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        if self.api_key.is_none() {
            return Err(WebError::NoProvider);
        }
        Ok(())
    }
}

// =============================================================================
// RawFetch provider (extract fallback, no API key needed)
// =============================================================================

struct RawFetchProvider {
    client: reqwest::Client,
}

impl RawFetchProvider {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }
}

#[async_trait]
impl WebExtractProvider for RawFetchProvider {
    fn kind(&self) -> &str {
        "rawfetch"
    }

    async fn extract(
        &self,
        url: &str,
        _opts: &ExtractOptions,
    ) -> Result<ExtractedContent, WebError> {
        let resp =
            self.client.get(url).send().await.map_err(|e| {
                WebError::ProviderUnavailable(format!("RawFetch request failed: {e}"))
            })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            WebError::ProviderError(format!("RawFetch read error: {e}"))
        })?;
        if !status.is_success() {
            return Err(WebError::ProviderError(format!("RawFetch error {status}: {}", body.chars().take(200).collect::<String>())));
        }

        let content = strip_html(&body);

        Ok(ExtractedContent {
            url: url.to_string(),
            content,
            format: "markdown".to_string(),
            metadata: None,
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        Ok(())
    }
}

fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut tag_name = String::new();
    let mut collecting_tag = false;

    for ch in html.chars() {
        if in_tag {
            if ch == '>' {
                let tag_lower = tag_name.to_lowercase();
                if tag_lower == "script" || tag_lower == "style" {
                    in_script = true;
                } else if tag_lower == "/script" || tag_lower == "/style" {
                    in_script = false;
                } else if tag_lower == "br" || tag_lower.starts_with("br ") {
                    result.push('\n');
                } else if tag_lower == "p" || tag_lower.starts_with("p ") {
                    result.push('\n');
                } else if tag_lower == "/p" {
                    result.push('\n');
                } else if tag_lower == "h1"
                    || tag_lower.starts_with("h1 ")
                    || tag_lower == "h2"
                    || tag_lower.starts_with("h2 ")
                    || tag_lower == "h3"
                    || tag_lower.starts_with("h3 ")
                {
                    result.push_str("\n## ");
                } else if tag_lower == "/h1" || tag_lower == "/h2" || tag_lower == "/h3" {
                    result.push('\n');
                } else if tag_lower == "li" || tag_lower.starts_with("li ") {
                    result.push_str("- ");
                } else if tag_lower == "a" || tag_lower.starts_with("a ") {
                    // skip
                }
                in_tag = false;
                collecting_tag = false;
                tag_name.clear();
            } else if collecting_tag {
                if ch == ' ' || ch == '\n' || ch == '\r' || ch == '\t' {
                    collecting_tag = false;
                } else {
                    tag_name.push(ch);
                }
            } else if tag_name.is_empty() && (ch == '/' || ch.is_alphabetic()) {
                collecting_tag = true;
                tag_name.push(ch);
            }
            continue;
        }
        if in_script {
            if ch == '<' {
                in_tag = true;
                tag_name.clear();
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
            continue;
        }
        result.push(ch);
    }

    let lines: Vec<&str> = result
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

// =============================================================================
// Provider pool & strategy engine
// =============================================================================

struct ProviderPool {
    search_providers: Vec<Box<dyn WebSearchProvider>>,
    extract_providers: Vec<Box<dyn WebExtractProvider>>,
    browse_providers: Vec<Box<dyn WebBrowseProvider>>,
}

impl ProviderPool {
    async fn search_with_fallback(
        &self,
        query: &SearchQuery,
        primary: &str,
    ) -> Result<Vec<SearchResult>, WebError> {
        let mut ordered: Vec<&dyn WebSearchProvider> = Vec::new();
        for p in &self.search_providers {
            if p.kind() == primary {
                ordered.insert(0, p.as_ref());
            } else {
                ordered.push(p.as_ref());
            }
        }

        let mut last_err = WebError::NoProvider;
        for provider in ordered {
            match provider.search(query).await {
                Ok(results) => return Ok(results),
                Err(e) => {
                    tracing::warn!(provider = provider.kind(), error = %e, "Search provider failed, trying next");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn extract_with_fallback(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<ExtractedContent, WebError> {
        let mut last_err = WebError::NoProvider;
        for provider in &self.extract_providers {
            match provider.extract(url, opts).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(provider = provider.kind(), error = %e, "Extract provider failed, trying next");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn browse_with_fallback(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
    ) -> Result<BrowseResult, WebError> {
        let mut last_err = WebError::NoProvider;
        for provider in &self.browse_providers {
            match provider.browse(url, instruction, timeout).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(provider = provider.kind(), error = %e, "Browse provider failed, trying next");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum SearchStrategy {
    Quick,
    Semantic,
    Extract,
    Deep,
    Fetch,
}

impl std::fmt::Display for SearchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchStrategy::Quick => write!(f, "quick"),
            SearchStrategy::Semantic => write!(f, "semantic"),
            SearchStrategy::Extract => write!(f, "extract"),
            SearchStrategy::Deep => write!(f, "deep"),
            SearchStrategy::Fetch => write!(f, "fetch"),
        }
    }
}

impl std::str::FromStr for SearchStrategy {
    type Err = WebError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "quick" => Ok(SearchStrategy::Quick),
            "semantic" => Ok(SearchStrategy::Semantic),
            "extract" => Ok(SearchStrategy::Extract),
            "deep" => Ok(SearchStrategy::Deep),
            "fetch" => Ok(SearchStrategy::Fetch),
            _ => Err(WebError::BadArgs(format!(
                "Unknown strategy: {s}. Use: quick, semantic, extract, deep, fetch"
            ))),
        }
    }
}

// =============================================================================
// Cache
// =============================================================================

#[derive(Clone)]
struct CacheEntry {
    data: serde_json::Value,
    inserted_at: std::time::Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct CacheKey(String);

struct ResponseCache {
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    max_entries: usize,
    default_ttl: Duration,
}

impl ResponseCache {
    fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            default_ttl,
        }
    }

    async fn get(&self, key: &CacheKey) -> Option<serde_json::Value> {
        let entries = self.entries.read().await;
        entries.get(key).and_then(|e| {
            if e.is_expired() {
                None
            } else {
                Some(e.data.clone())
            }
        })
    }

    async fn insert(&self, key: CacheKey, data: serde_json::Value) {
        let mut entries = self.entries.write().await;
        if entries.len() >= self.max_entries {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }
        entries.insert(
            key,
            CacheEntry {
                data,
                inserted_at: std::time::Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }
}

fn cache_key(strategy: &str, query: &str, params: &serde_json::Value) -> CacheKey {
    let hash = blake3::hash(
        format!(
            "{strategy}:{query}:{}",
            serde_json::to_string(params).unwrap_or_default()
        )
        .as_bytes(),
    );
    CacheKey(hash.to_hex().to_string())
}

// =============================================================================
// WebServer
// =============================================================================

pub struct WebServer {
    pool: ProviderPool,
    cache: Arc<ResponseCache>,
}

impl WebServer {
    fn new(
        brave_api_key: Option<String>,
        firecrawl_api_key: Option<String>,
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

        let mut search_providers: Vec<Box<dyn WebSearchProvider>> = Vec::new();
        let mut extract_providers: Vec<Box<dyn WebExtractProvider>> = Vec::new();
        let mut browse_providers: Vec<Box<dyn WebBrowseProvider>> = Vec::new();

        if let Some(ref key) = brave_api_key {
            search_providers.push(Box::new(BraveProvider::new(key.clone())));
        }

        if let Some(ref key) = firecrawl_api_key {
            let fc = FirecrawlProvider::new(Some(key.clone()));
            search_providers.push(Box::new(FirecrawlProvider {
                client: fc.client.clone(),
                api_key: fc.api_key.clone(),
            }));
            extract_providers.push(Box::new(FirecrawlProvider::new(Some(key.clone()))));
            browse_providers.push(Box::new(FirecrawlProvider::new(Some(key.clone()))));
        }

        extract_providers.push(Box::new(RawFetchProvider::new()));

        if search_providers.is_empty() {
            anyhow::bail!(
                "No search providers configured. Set HKASK_BRAVE_API_KEY or HKASK_FIRECRAWL_API_KEY."
            );
        }

        Ok(Self {
            pool: ProviderPool {
                search_providers,
                extract_providers,
                browse_providers,
            },
            cache: Arc::new(ResponseCache::new(
                cache_max,
                Duration::from_secs(cache_ttl),
            )),
        })
    }
}

#[tool_router(server_handler)]
impl WebServer {
    #[tool(description = "Liveness and provider health check")]
    async fn web_ping(&self) -> String {
        let mut providers = Vec::new();
        for p in &self.pool.search_providers {
            providers.push(format!("search:{}", p.kind()));
        }
        for p in &self.pool.extract_providers {
            providers.push(format!("extract:{}", p.kind()));
        }
        for p in &self.pool.browse_providers {
            providers.push(format!("browse:{}", p.kind()));
        }
        McpToolOutput::new(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "providers": providers,
        }))
        .to_json_string()
    }

    #[tool(
        description = "Search the web using keyword/semantic queries with multi-provider routing"
    )]
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

        let primary = match strat {
            SearchStrategy::Quick | SearchStrategy::Extract | SearchStrategy::Fetch => "brave",
            SearchStrategy::Semantic | SearchStrategy::Deep => "firecrawl",
        };

        match self.pool.search_with_fallback(&search_query, primary).await {
            Ok(results) => {
                emit_tool_span("web_search", "ok", start.elapsed().as_millis() as u64, None);
                let output = serde_json::json!({
                    "query": query,
                    "strategy": strat.to_string(),
                    "results": results,
                    "count": results.len(),
                });
                self.cache.insert(ckey, output.clone()).await;
                McpToolOutput::with_timing(output, start).to_json_string()
            }
            Err(e) => {
                emit_tool_span(
                    "web_search",
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

        let cache_params =
            serde_json::json!({ "format": fmt, "main_content_only": opts.main_content_only });
        let ckey = cache_key("extract", &url, &cache_params);

        if let Some(cached) = self.cache.get(&ckey).await {
            return McpToolOutput::with_timing(cached, start).to_json_string();
        }

        match self.pool.extract_with_fallback(&url, &opts).await {
            Ok(result) => {
                emit_tool_span(
                    "web_extract",
                    "ok",
                    start.elapsed().as_millis() as u64,
                    None,
                );
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

    #[tool(description = "Interactive browsing of JS-heavy pages")]
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

    #[tool(description = "Deep multi-step research cascade across search and extraction")]
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

        let search_results = match self.pool.search_with_fallback(&search_query, "brave").await {
            Ok(r) => r,
            Err(e) => {
                emit_tool_span(
                    "web_research",
                    "error",
                    start.elapsed().as_millis() as u64,
                    Some(&e.kind()),
                );
                return McpToolError::from(e).to_json_string();
            }
        };

        let mut pages = Vec::new();
        let mut urls_seen = std::collections::HashSet::new();

        for result in search_results.iter().take(max_pages) {
            if urls_seen.contains(&result.url) {
                continue;
            }
            urls_seen.insert(result.url.clone());

            if let Err(_) = validate_tool_url(&result.url) {
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

        emit_tool_span(
            "web_research",
            "ok",
            start.elapsed().as_millis() as u64,
            None,
        );
        McpToolOutput::with_timing(
            serde_json::json!({
                "query": query,
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

    let mut required_creds = Vec::new();
    if brave_key.is_none() && firecrawl_key.is_none() {
        required_creds.push(CredentialRequirement::required(
            "HKASK_BRAVE_API_KEY",
            "Brave Search API key (or HKASK_FIRECRAWL_API_KEY for Firecrawl)",
        ));
    }

    run_stdio_server(
        "hkask-mcp-web",
        SERVER_VERSION,
        || WebServer::new(brave_key, firecrawl_key),
        required_creds,
    )
    .await
}
