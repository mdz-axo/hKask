//! Core types, constants, and re-exports for the hKask MCP Web crate.

mod freshness;
mod ranking;
mod rate_limiter;
mod validation;

use hkask_mcp::server::McpToolError;
use hkask_types::McpErrorKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Constants ────────────────────────────────────────────────────────────────

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BRAVE_API_BASE: &str = "https://api.search.brave.com/res/v1";
pub const FIRECRAWL_API_BASE: &str = "https://api.firecrawl.dev/v2";
pub const TAVILY_API_BASE: &str = "https://api.tavily.com";
pub const SERPAPI_BASE: &str = "https://serpapi.com/search";
pub const EXA_API_BASE: &str = "https://api.exa.ai";
pub const BROWSERBASE_API_BASE: &str = "https://api.browserbase.com/v1";
pub const DEFAULT_CACHE_TTL_SECS: u64 = 300;
pub const MAX_CACHE_TTL_SECS: u64 = 7200;
pub const DEFAULT_CACHE_MAX_ENTRIES: usize = 50;
pub const MAX_CACHE_MAX_ENTRIES: usize = 200;
pub const MAX_CACHE_VALUE_BYTES: usize = 1_048_576;
pub const RRF_K: u64 = 60;
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;
pub const RATE_LIMIT_MAX_REQUESTS: u32 = 30;

// --- Task 2: Request timeout for all provider calls ---
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

// --- Task 5: Input validation bounds ---
pub const MAX_QUERY_LENGTH: usize = 400;
pub const MAX_URL_LENGTH: usize = 2048;
pub const MAX_INSTRUCTION_LENGTH: usize = 2000;
pub const MAX_JSON_PROMPT_LENGTH: usize = 4000;
pub const MAX_JSON_SCHEMA_BYTES: usize = 32_768;

// --- Task 8: Firecrawl API version ---
pub const FIRECRAWL_API_VERSION: &str = "v2";

// ── Re-exports from submodules ───────────────────────────────────────────────

pub use freshness::{Freshness, freshness_brave, freshness_serpapi, normalize_freshness};
pub use hkask_memory::ranking::parse_age_to_days;
pub use ranking::{apply_rerank, dedup_results, normalize_date_bucket, rrf_score};
pub use rate_limiter::RateLimiter;
pub use validation::{
    COMPOUND_PROVIDER_TIMEOUT_SECS, sanitize_health_error, validate_browse_request,
    validate_extract_request, validate_search_request,
};

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query: String,
    pub num_results: Option<u32>,
    pub include_domains: Option<Vec<String>>,
    pub exclude_domains: Option<Vec<String>>,
    pub freshness: Option<String>,
    pub strategy: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindSimilarRequest {
    pub url: String,
    pub num_results: Option<u32>,
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

// ── Result types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub published: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
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
    pub freshness: Option<Freshness>,
    /// Search depth hint for providers that support it (e.g., Tavily "basic"/"advanced").
    /// Set by `ProviderPool` based on `SearchStrategy` before calling individual providers.
    pub depth: SearchDepth,
}

/// Search depth hint derived from `SearchStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchDepth {
    Basic,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub format: String,
    pub json_prompt: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub main_content_only: bool,
    pub wait_for_ms: u64,
}

// ── Error type ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("Bad arguments: {0}")]
    BadArgs(String),
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    /// External API boundary rate limit (HTTP 429 from upstream providers).
    /// Distinct from internal energy budget tracking.
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("No provider available")]
    NoProvider,
}

impl WebError {
    pub fn kind(&self) -> McpErrorKind {
        match self {
            WebError::BadArgs(_) => McpErrorKind::InvalidArgument,
            WebError::ProviderUnavailable(_) => McpErrorKind::Unavailable,
            WebError::ProviderError(_) => McpErrorKind::Internal,
            WebError::RateLimited(_) => McpErrorKind::RateLimited,
            WebError::NoProvider => McpErrorKind::Unavailable,
        }
    }
}

impl From<WebError> for McpToolError {
    fn from(e: WebError) -> Self {
        McpToolError::new(e.kind(), e.to_string())
    }
}

// ── Capability / provider types ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchCapability {
    Keyword,
    News,
    Freshness,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub published: Option<String>,
    pub rrf_score: f64,
    pub provider_count: usize,
    pub providers: Vec<String>,
    pub best_rank: Option<usize>,
    pub content_preview: Option<String>,
    pub semantic_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extracted_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerBox {
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub kind: String,
    pub capabilities: Vec<SearchCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderError {
    pub kind: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct CompoundSearchResult {
    pub query: String,
    pub strategy: String,
    pub results: Vec<RankedResult>,
    pub answer_box: Option<AnswerBox>,
    pub related_questions: Vec<String>,
    pub providers_queried: Vec<ProviderInfo>,
    pub providers_succeeded: Vec<String>,
    pub providers_failed: Vec<ProviderError>,
    pub total_before_dedup: usize,
    pub duplicates_removed: usize,
}

// ── Strategy & filter types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchStrategy {
    Quick,
    Web,
    News,
    Deep,
}

impl SearchStrategy {
    pub fn provider_filter(&self) -> ProviderFilter {
        match self {
            SearchStrategy::Quick => ProviderFilter::Capabilities(vec![SearchCapability::Keyword]),
            SearchStrategy::Web => ProviderFilter::All,
            SearchStrategy::News => ProviderFilter::Capabilities(vec![SearchCapability::News]),
            SearchStrategy::Deep => ProviderFilter::All,
        }
    }
}

pub enum ProviderFilter {
    All,
    Capabilities(Vec<SearchCapability>),
    Kinds(Vec<&'static str>),
}

impl ProviderFilter {
    pub fn matches(&self, provider_kind: &str) -> bool {
        match self {
            ProviderFilter::All => true,
            ProviderFilter::Capabilities(_caps) => true, // capabilities filtering is done separately
            ProviderFilter::Kinds(kinds) => kinds.contains(&provider_kind),
        }
    }
}

impl std::fmt::Display for SearchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchStrategy::Quick => write!(f, "quick"),
            SearchStrategy::Web => write!(f, "web"),
            SearchStrategy::News => write!(f, "news"),
            SearchStrategy::Deep => write!(f, "deep"),
        }
    }
}

impl std::str::FromStr for SearchStrategy {
    type Err = WebError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "quick" => Ok(SearchStrategy::Quick),
            "web" | "semantic" => Ok(SearchStrategy::Web),
            "news" => Ok(SearchStrategy::News),
            "deep" | "research" => Ok(SearchStrategy::Deep),
            _ => Err(WebError::BadArgs(format!(
                "Unknown strategy: {s}. Use: quick, web, news, deep"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerankSignal {
    Recency,
    Semantic,
    ContentQuality,
}

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultOutput {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub published: Option<String>,
    pub content_preview: Option<String>,
}

impl From<&RankedResult> for SearchResultOutput {
    fn from(r: &RankedResult) -> Self {
        Self {
            title: r.title.clone(),
            url: r.url.clone(),
            description: r.description.clone(),
            source: r.source.clone(),
            published: r.published.clone(),
            content_preview: r.content_preview.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOutput {
    pub query: String,
    pub strategy: String,
    pub results: Vec<SearchResultOutput>,
    pub answer_box: Option<AnswerBox>,
    pub related_questions: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    pub strategy: String,
    pub providers_queried: Vec<ProviderInfo>,
    pub providers_succeeded: Vec<String>,
    pub providers_failed: Vec<ProviderError>,
    pub total_before_dedup: usize,
    pub duplicates_removed: usize,
    pub top_rrf_scores: Vec<f64>,
}

impl From<&CompoundSearchResult> for SearchMetadata {
    fn from(c: &CompoundSearchResult) -> Self {
        Self {
            strategy: c.strategy.clone(),
            providers_queried: c.providers_queried.clone(),
            providers_succeeded: c.providers_succeeded.clone(),
            providers_failed: c.providers_failed.clone(),
            total_before_dedup: c.total_before_dedup,
            duplicates_removed: c.duplicates_removed,
            top_rrf_scores: c.results.iter().take(5).map(|r| r.rrf_score).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarResultOutput {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub published: Option<String>,
    pub semantic_score: Option<f64>,
    pub content_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarOutput {
    pub source_url: String,
    pub results: Vec<FindSimilarResultOutput>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOutput {
    pub url: String,
    pub format: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseOutput {
    pub url: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    pub actions_taken: Vec<String>,
}

// ── Health / ping types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthEntry {
    pub kind: String,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingOutput {
    pub status: String,
    pub version: String,
    pub providers: Vec<ProviderHealthEntry>,
}

// ── Capability context ───────────────────────────────────────────────────────

/// Capability context for OCAP-style enforcement at the port boundary.
///
/// When `Some(ctx)` is provided, each `WebSearchPort` method checks
/// `ctx.allows(tool_name)` before proceeding. When `None`, all
/// capabilities are allowed (current default behavior).
///
/// When `hkask-keystore` and `hkask-agents` ACP are ready, the MCP server
/// will extract capabilities from the session and pass them through.
#[derive(Debug, Clone, Default)]
pub struct CapabilityContext {
    pub requester_id: Option<String>,
    pub capabilities: Vec<String>,
}

impl CapabilityContext {
    /// Check whether the given tool name is in the capability set.
    ///
    /// If `capabilities` is empty, allows all (open policy).
    /// Otherwise, requires an explicit match.
    pub fn allows(&self, tool: &str) -> bool {
        if self.capabilities.is_empty() {
            return true;
        }
        self.capabilities.iter().any(|c| c == tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CapabilityContext ───────────────────────────────────────────────────

    // P8 invariant: empty capabilities allows all (open policy)
    #[test]
    fn capability_context_empty_allows_all() {
        let ctx = CapabilityContext::default();
        assert!(ctx.allows("web_search"));
        assert!(ctx.allows("web_extract"));
    }

    // P8 invariant: explicit capabilities only allow listed tools
    #[test]
    fn capability_context_explicit_allows_listed() {
        let ctx = CapabilityContext {
            requester_id: None,
            capabilities: vec!["web_search".to_string()],
        };
        assert!(ctx.allows("web_search"));
        assert!(!ctx.allows("web_extract"));
    }

    // P8 invariant: multiple capabilities allow any listed tool
    #[test]
    fn capability_context_multiple_capabilities() {
        let ctx = CapabilityContext {
            requester_id: None,
            capabilities: vec!["web_search".to_string(), "web_extract".to_string()],
        };
        assert!(ctx.allows("web_search"));
        assert!(ctx.allows("web_extract"));
        assert!(!ctx.allows("web_browse"));
    }

    // ── SearchStrategy ──────────────────────────────────────────────────────

    // P8 invariant: canonical names parse correctly
    #[test]
    fn search_strategy_parses_canonical() {
        assert_eq!(
            "quick".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Quick
        );
        assert_eq!(
            "web".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Web
        );
        assert_eq!(
            "news".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::News
        );
        assert_eq!(
            "deep".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Deep
        );
    }

    // P8 invariant: aliases parse correctly
    #[test]
    fn search_strategy_parses_aliases() {
        assert_eq!(
            "semantic".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Web
        );
        assert_eq!(
            "research".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Deep
        );
    }

    // P8 invariant: case-insensitive parsing
    #[test]
    fn search_strategy_case_insensitive() {
        assert_eq!(
            "QUICK".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Quick
        );
        assert_eq!(
            "Web".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Web
        );
    }

    // P8 invariant: invalid strategy is rejected
    #[test]
    fn search_strategy_rejects_invalid() {
        assert!("invalid".parse::<SearchStrategy>().is_err());
    }

    // P8 invariant: Display round-trips
    #[test]
    fn search_strategy_display_round_trips() {
        for variant in [
            SearchStrategy::Quick,
            SearchStrategy::Web,
            SearchStrategy::News,
            SearchStrategy::Deep,
        ] {
            let s = variant.to_string();
            assert_eq!(
                s.parse::<SearchStrategy>().unwrap(),
                variant,
                "Display must round-trip"
            );
        }
    }

    // ── ProviderFilter ─────────────────────────────────────────────────────

    // P8 invariant: All filter matches everything
    #[test]
    fn provider_filter_all_matches_everything() {
        let filter = ProviderFilter::All;
        assert!(filter.matches("brave"));
        assert!(filter.matches("tavily"));
    }

    // P8 invariant: Kinds filter only matches listed kinds
    #[test]
    fn provider_filter_kinds_matches_listed() {
        let filter = ProviderFilter::Kinds(vec!["brave", "tavily"]);
        assert!(filter.matches("brave"));
        assert!(filter.matches("tavily"));
        assert!(!filter.matches("exa"));
    }

    // ── WebError::kind() ─────────────────────────────────────────────────────

    // P8 invariant: each variant maps to the correct McpErrorKind
    #[test]
    fn web_error_kind_mapping() {
        assert_eq!(
            WebError::BadArgs("x".into()).kind(),
            McpErrorKind::InvalidArgument
        );
        assert_eq!(
            WebError::ProviderUnavailable("x".into()).kind(),
            McpErrorKind::Unavailable
        );
        assert_eq!(
            WebError::ProviderError("x".into()).kind(),
            McpErrorKind::Internal
        );
        assert_eq!(
            WebError::RateLimited("x".into()).kind(),
            McpErrorKind::RateLimited
        );
        assert_eq!(WebError::NoProvider.kind(), McpErrorKind::Unavailable);
    }

    // ── SearchResultOutput From<RankedResult> ───────────────────────────────

    // P8 invariant: From\u003cRankedResult\u003e preserves all mapped fields
    #[test]
    fn search_result_output_from_ranked() {
        let ranked = RankedResult {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            description: Some("desc".to_string()),
            source: Some("brave".to_string()),
            published: Some("today".to_string()),
            rrf_score: 0.5,
            provider_count: 1,
            providers: vec!["brave".to_string()],
            best_rank: Some(0),
            content_preview: Some("preview".to_string()),
            semantic_score: None,
            extracted_content: None,
        };
        let output = SearchResultOutput::from(&ranked);
        assert_eq!(output.title, "Test");
        assert_eq!(output.url, "https://example.com");
        assert_eq!(output.description, Some("desc".to_string()));
        assert_eq!(output.source, Some("brave".to_string()));
        assert_eq!(output.published, Some("today".to_string()));
        assert_eq!(output.content_preview, Some("preview".to_string()));
    }
}
