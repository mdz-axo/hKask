use chrono::Datelike;
use hkask_mcp::server::McpToolError;
use hkask_types::McpErrorKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub freshness: Option<String>,
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

// =============================================================================
// Task 3: Sanitize health error messages to prevent credential leakage
// =============================================================================

/// Regex pattern matching common API key prefixes followed by 8+ alphanumeric chars.
static API_KEY_PATTERN: &str = r"(?:sk-|pk-|fc-|ts-|br-|xai-|ghp_)[a-zA-Z0-9]{8,}";

/// Sanitize a provider health error to prevent credential leakage.
///
/// Replaces detailed error messages with generic categories and strips
/// any substrings that look like API keys (matching common prefix patterns).
pub fn sanitize_health_error(error: &str) -> String {
    let sanitized = regex::Regex::new(API_KEY_PATTERN)
        .map(|re| re.replace_all(error, "[REDACTED]").to_string())
        .unwrap_or_else(|_| error.to_string());

    let lower = sanitized.to_lowercase();
    if lower.contains("401") || lower.contains("403") || lower.contains("auth") {
        "authentication failed".to_string()
    } else if lower.contains("429") || lower.contains("rate") {
        "rate limited".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout".to_string()
    } else if lower.contains("unreachable") || lower.contains("connection") || lower.contains("dns")
    {
        "unreachable".to_string()
    } else if lower.contains("no provider") {
        "no provider available".to_string()
    } else {
        "unhealthy".to_string()
    }
}

// =============================================================================
// Task 9: Freshness normalization per provider
// =============================================================================

/// Normalized freshness values at the MCP boundary.
///
/// Each provider adapter translates these to its own parameter format.
/// This follows the Cockburn principle: the port defines the canonical model,
/// adapters translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Freshness {
    Day,
    Week,
    Month,
    Year,
}

impl std::str::FromStr for Freshness {
    type Err = WebError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "day" | "d" | "1d" | "past_day" | "past day" | "24h" => Ok(Freshness::Day),
            "week" | "w" | "1w" | "past_week" | "past week" | "7d" | "pw" => Ok(Freshness::Week),
            "month" | "m" | "1m" | "past_month" | "past month" | "30d" | "pm" => {
                Ok(Freshness::Month)
            }
            "year" | "y" | "1y" | "past_year" | "past year" | "365d" | "py" => Ok(Freshness::Year),
            _ => Err(WebError::BadArgs(format!(
                "Unknown freshness: {s}. Use: day, week, month, year"
            ))),
        }
    }
}

/// Returns provider-specific key-value pairs for the given freshness value.
///
/// Each provider translates normalized freshness into its own parameter format:
/// - Brave: `freshness=pw` (past week)
/// - Tavily: `days=7`
/// - SerpAPI: `tbs=qdr:w`
pub fn normalize_freshness(freshness: &Freshness) -> Vec<(&'static str, String)> {
    match freshness {
        Freshness::Day => vec![("days", "1".to_string())],
        Freshness::Week => vec![("days", "7".to_string())],
        Freshness::Month => vec![("days", "30".to_string())],
        Freshness::Year => vec![("days", "365".to_string())],
    }
}

/// Map freshness to Brave's parameter format.
pub fn freshness_brave(freshness: &Freshness) -> String {
    match freshness {
        Freshness::Day => "pd".to_string(),
        Freshness::Week => "pw".to_string(),
        Freshness::Month => "pm".to_string(),
        Freshness::Year => "py".to_string(),
    }
}

/// Map freshness to SerpAPI's `tbs` parameter format.
pub fn freshness_serpapi(freshness: &Freshness) -> String {
    match freshness {
        Freshness::Day => "qdr:d".to_string(),
        Freshness::Week => "qdr:w".to_string(),
        Freshness::Month => "qdr:m".to_string(),
        Freshness::Year => "qdr:y".to_string(),
    }
}

impl std::fmt::Display for Freshness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Freshness::Day => write!(f, "day"),
            Freshness::Week => write!(f, "week"),
            Freshness::Month => write!(f, "month"),
            Freshness::Year => write!(f, "year"),
        }
    }
}

// =============================================================================
// Task 13: CapabilityContext — OCAP preparation
// =============================================================================

/// Optional capability context for future OCAP enforcement.
///
/// For now, pass `None` everywhere. When `hkask-keystore` and `hkask-agents`
/// ACP are ready, the MCP server will extract capabilities from the session
/// and pass them through. This follows Miller's principle: design the
/// authority boundary now, populate it later.
#[derive(Debug, Clone, Default)]
pub struct CapabilityContext {
    pub requester_id: Option<String>,
    pub capabilities: Vec<String>,
}

// =============================================================================
// Task 14: CredentialResolver — rotation support
// =============================================================================

/// Async trait for resolving credentials with rotation support.
///
/// The production implementation wraps `resolve_credential()`. A future
/// implementation can call `hkask-keystore` for key rotation without
/// restarting the server.
#[async_trait::async_trait]
pub trait CredentialResolver: Send + Sync {
    async fn get_credential(&self, name: &str) -> Result<String, WebError>;
}

/// Production credential resolver that reads from environment / .env files.
pub struct EnvCredentialResolver;

#[async_trait::async_trait]
impl CredentialResolver for EnvCredentialResolver {
    async fn get_credential(&self, name: &str) -> Result<String, WebError> {
        hkask_mcp::server::resolve_credential(name).map_err(|e| {
            WebError::ProviderUnavailable(format!("Credential '{}' unavailable: {}", name, e))
        })
    }
}

pub fn rrf_score(ranks: &[usize]) -> f64 {
    ranks
        .iter()
        .map(|&r| 1.0 / (RRF_K as f64 + r as f64 + 1.0))
        .sum()
}

pub fn parse_age_to_days(age: &str) -> f64 {
    let lower = age.to_lowercase();
    let lower = lower.trim();

    if lower.is_empty() {
        return -1.0;
    }

    if let Some(rest) = lower.strip_suffix(" ago") {
        let rest = rest.trim();
        return parse_relative_age(rest);
    }

    if let Ok(days) = parse_iso_date_to_days(lower) {
        return days;
    }

    if let Some(rest) = lower.strip_prefix("published ") {
        if let Ok(days) = parse_iso_date_to_days(rest) {
            return days;
        }
        if let Some(rest2) = rest.strip_suffix(" ago") {
            return parse_relative_age(rest2.trim());
        }
    }

    parse_fuzzy_date(lower)
}

fn parse_relative_age(rest: &str) -> f64 {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return -1.0;
    }
    let num: f64 = match parts[0].parse() {
        Ok(n) => n,
        Err(_) => return -1.0,
    };
    match parts[1] {
        s if s.starts_with("second") => num / 86400.0,
        s if s.starts_with("minute") => num / 1440.0,
        s if s.starts_with("hour") => num / 24.0,
        s if s.starts_with("day") => num,
        s if s.starts_with("week") => num * 7.0,
        s if s.starts_with("month") => num * 30.0,
        s if s.starts_with("year") => num * 365.0,
        _ => -1.0,
    }
}

fn parse_iso_date_to_days(s: &str) -> Result<f64, ()> {
    let s = s.trim();
    if s.len() < 10 {
        return Err(());
    }
    let year: i32 = s.get(0..4).ok_or(())?.parse().map_err(|_| ())?;
    let month: i32 = s.get(5..7).ok_or(())?.parse().map_err(|_| ())?;
    let day: i32 = s.get(8..10).ok_or(())?.parse().map_err(|_| ())?;

    if !(2000..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(());
    }

    let now = chrono::Utc::now();
    let now_ordinal = now.ordinal0() as i32 + 1;
    let now_yday = now.year() * 366 + now_ordinal;

    let target_ordinal = ordinal_day(year, month, day);
    let target_yday = year * 366 + target_ordinal;

    let diff = now_yday - target_yday;
    if diff < 0 {
        return Ok(0.0);
    }
    Ok(diff as f64)
}

fn ordinal_day(year: i32, month: i32, day: i32) -> i32 {
    let days_in_months = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let mut ordinal = day;
    for m in 1..month {
        ordinal += days_in_months[m as usize];
        if m == 2 && leap {
            ordinal += 1;
        }
    }
    ordinal
}

fn parse_fuzzy_date(s: &str) -> f64 {
    let parts: Vec<&str> = s.split(|c: char| !c.is_alphanumeric()).collect();
    let mut year: Option<i32> = None;
    let mut month: Option<i32> = None;
    let mut day: Option<i32> = None;
    let month_names = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];

    for part in parts {
        if part.is_empty() {
            continue;
        }
        if let Ok(n) = part.parse::<i32>() {
            if (2000..=2100).contains(&n) && year.is_none() {
                year = Some(n);
            } else if (1..=12).contains(&n) && month.is_none() {
                month = Some(n);
            } else if (1..=31).contains(&n) && day.is_none() {
                day = Some(n);
            }
        } else {
            let lower = part.to_lowercase();
            for (i, name) in month_names.iter().enumerate() {
                if lower.starts_with(name) {
                    month = Some((i + 1) as i32);
                    break;
                }
            }
        }
    }

    if let Some(y) = year {
        let m = month.unwrap_or(1);
        let d = day.unwrap_or(1);
        parse_iso_date_to_days(&format!("{y:04}-{m:02}-{d:02}")).unwrap_or(-1.0)
    } else {
        -1.0
    }
}

pub fn apply_rerank(results: &mut [RankedResult], signal: RerankSignal) {
    match signal {
        RerankSignal::Recency => {
            for r in results.iter_mut() {
                if let Some(ref published) = r.published {
                    let days = parse_age_to_days(published);
                    if days >= 0.0 {
                        let boost = 1.0 / (1.0 + days / 7.0);
                        r.rrf_score += boost * 0.1;
                    }
                }
            }
        }
        RerankSignal::Semantic => {
            for r in results.iter_mut() {
                if let Some(score) = r.semantic_score {
                    r.rrf_score += score * 0.05;
                }
            }
        }
        RerankSignal::ContentQuality => {
            for r in results.iter_mut() {
                if r.content_preview.is_some() || r.extracted_content.is_some() {
                    r.rrf_score += 0.05;
                }
            }
        }
    }
    results.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn dedup_results(existing: &mut Vec<RankedResult>, incoming: Vec<RankedResult>) {
    for r in incoming {
        let key = r.url.to_lowercase();
        if let Some(idx) = existing.iter().position(|e| e.url.to_lowercase() == key) {
            if r.rrf_score > existing[idx].rrf_score {
                existing[idx] = r;
            }
        } else {
            existing.push(r);
        }
    }
    existing.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn normalize_date_bucket(published: &str) -> &'static str {
    let days = parse_age_to_days(published);
    if days < 0.0 {
        return "unknown";
    }
    if days < 1.0 {
        return "today";
    }
    if days < 7.0 {
        return "this week";
    }
    if days < 31.0 {
        return "this month";
    }
    "older"
}

// =============================================================================
// RateLimiter — Token-bucket per-tool rate limiting
//
// Enforces a configurable number of requests per time window per tool name.
// This is the MCP boundary approximation of hKask's energy budget model.
// On rate limit, returns McpToolError with RateLimited kind.
// =============================================================================

use std::collections::HashMap;
use std::sync::Mutex;

pub struct RateLimiter {
    windows: Mutex<HashMap<String, RateWindow>>,
    max_requests: u32,
    window_secs: u64,
}

struct RateWindow {
    count: u32,
    expires_at: std::time::Instant,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            max_requests,
            window_secs,
        }
    }

    /// Check whether a request for the given tool is allowed.
    /// Returns Ok(()) if allowed, or an McpToolError with RateLimited kind if exceeded.
    pub fn check(&self, tool_name: &str) -> Result<(), McpToolError> {
        let mut windows = self.windows.lock().expect("rate limiter lock poisoned");
        let now = std::time::Instant::now();
        let entry = windows.entry(tool_name.to_string()).or_insert(RateWindow {
            count: 0,
            expires_at: now + std::time::Duration::from_secs(self.window_secs),
        });
        if now >= entry.expires_at {
            entry.count = 0;
            entry.expires_at = now + std::time::Duration::from_secs(self.window_secs);
        }
        entry.count += 1;
        if entry.count > self.max_requests {
            Err(McpToolError::new(
                McpErrorKind::RateLimited,
                format!(
                    "Rate limit exceeded for {tool_name}: {} requests per {}s",
                    self.max_requests, self.window_secs
                ),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_parse_aliases() {
        assert_eq!(
            "semantic".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Web
        );
        assert_eq!(
            "research".parse::<SearchStrategy>().unwrap(),
            SearchStrategy::Deep
        );
    }

    #[test]
    fn strategy_parse_invalid() {
        assert!("invalid".parse::<SearchStrategy>().is_err());
    }

    #[test]
    fn rrf_score_single_rank() {
        let score = rrf_score(&[0]);
        let expected = 1.0 / (RRF_K as f64 + 0.0 + 1.0);
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn rrf_score_multiple_ranks() {
        let score = rrf_score(&[0, 2]);
        let expected = 1.0 / (RRF_K as f64 + 1.0) + 1.0 / (RRF_K as f64 + 3.0);
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn rrf_score_agreement_beats_single_high_rank() {
        let agreement = rrf_score(&[5, 5]);
        let single = rrf_score(&[0]);
        assert!(agreement > single);
    }

    #[test]
    fn parse_age_to_days_relative() {
        let hours = parse_age_to_days("2 hours ago");
        assert!((hours - 2.0 / 24.0).abs() < 0.01);
        let days = parse_age_to_days("3 days ago");
        assert!((days - 3.0).abs() < 0.01);
        let weeks = parse_age_to_days("1 week ago");
        assert!((weeks - 7.0).abs() < 0.01);
    }

    #[test]
    fn parse_age_to_days_iso_date() {
        let days = parse_age_to_days("2024-01-15");
        assert!(days > 0.0);
    }

    #[test]
    fn recency_rerank_boosts_recent() {
        let mut results = vec![
            RankedResult {
                title: "old".into(),
                url: "http://old".into(),
                description: None,
                source: None,
                published: Some("1 year ago".into()),
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            },
            RankedResult {
                title: "recent".into(),
                url: "http://recent".into(),
                description: None,
                source: None,
                published: Some("1 day ago".into()),
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(1),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            },
        ];
        apply_rerank(&mut results, RerankSignal::Recency);
        assert_eq!(results[0].title, "recent");
    }

    #[test]
    fn semantic_rerank_boosts_high_scores() {
        let mut results = vec![
            RankedResult {
                title: "low".into(),
                url: "http://low".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: Some(0.1),
                extracted_content: None,
            },
            RankedResult {
                title: "high".into(),
                url: "http://high".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(1),
                content_preview: None,
                semantic_score: Some(0.9),
                extracted_content: None,
            },
        ];
        apply_rerank(&mut results, RerankSignal::Semantic);
        assert_eq!(results[0].title, "high");
    }

    #[test]
    fn content_quality_rerank_boosts_previews() {
        let mut results = vec![
            RankedResult {
                title: "no-preview".into(),
                url: "http://no".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            },
            RankedResult {
                title: "with-preview".into(),
                url: "http://with".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(1),
                content_preview: Some("content".into()),
                semantic_score: None,
                extracted_content: None,
            },
        ];
        apply_rerank(&mut results, RerankSignal::ContentQuality);
        assert_eq!(results[0].title, "with-preview");
    }

    #[test]
    fn dedup_across_iterations() {
        let mut existing = vec![RankedResult {
            title: "t".into(),
            url: "http://x".into(),
            description: None,
            source: None,
            published: None,
            rrf_score: 0.3,
            provider_count: 1,
            providers: vec!["a".into()],
            best_rank: Some(0),
            content_preview: None,
            semantic_score: None,
            extracted_content: None,
        }];
        let incoming = vec![RankedResult {
            title: "t2".into(),
            url: "http://X".into(),
            description: None,
            source: None,
            published: None,
            rrf_score: 0.5,
            provider_count: 1,
            providers: vec!["b".into()],
            best_rank: Some(0),
            content_preview: None,
            semantic_score: None,
            extracted_content: None,
        }];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 1);
        assert!((existing[0].rrf_score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn normalize_date_bucket_unknown() {
        assert_eq!(normalize_date_bucket(""), "unknown");
        assert_eq!(normalize_date_bucket("gibberish"), "unknown");
    }

    #[test]
    fn normalize_date_bucket_today() {
        assert_eq!(normalize_date_bucket("1 hour ago"), "today");
        assert_eq!(normalize_date_bucket("2 hours ago"), "today");
    }

    #[test]
    fn normalize_date_bucket_this_week() {
        assert_eq!(normalize_date_bucket("3 days ago"), "this week");
    }

    #[test]
    fn normalize_date_bucket_this_month() {
        assert_eq!(normalize_date_bucket("2 weeks ago"), "this month");
    }

    #[test]
    fn normalize_date_bucket_older() {
        assert_eq!(normalize_date_bucket("2 months ago"), "older");
    }

    #[test]
    fn search_output_excludes_internals() {
        let ranked = RankedResult {
            title: "t".into(),
            url: "http://x".into(),
            description: Some("d".into()),
            source: None,
            published: None,
            rrf_score: 0.95,
            provider_count: 3,
            providers: vec!["a".into(), "b".into()],
            best_rank: Some(0),
            content_preview: None,
            semantic_score: Some(0.8),
            extracted_content: None,
        };
        let output = SearchResultOutput::from(&ranked);
        let json = serde_json::to_value(&output).unwrap();
        assert!(json.get("rrf_score").is_none());
        assert!(json.get("provider_count").is_none());
        assert!(json.get("providers").is_none());
        assert!(json.get("best_rank").is_none());
        assert!(json.get("semantic_score").is_none());
        assert_eq!(json["title"], "t");
    }

    #[test]
    fn search_metadata_captures_internals() {
        let compound = CompoundSearchResult {
            query: "q".into(),
            strategy: "web".into(),
            results: vec![RankedResult {
                title: "t".into(),
                url: "http://x".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 2,
                providers: vec!["a".into(), "b".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            }],
            answer_box: None,
            related_questions: vec![],
            providers_queried: vec![ProviderInfo {
                kind: "a".into(),
                capabilities: vec![],
            }],
            providers_succeeded: vec!["a".into()],
            providers_failed: vec![],
            total_before_dedup: 5,
            duplicates_removed: 1,
        };
        let meta = SearchMetadata::from(&compound);
        assert_eq!(meta.total_before_dedup, 5);
        assert_eq!(meta.duplicates_removed, 1);
        assert_eq!(meta.top_rrf_scores.len(), 1);
    }
}
