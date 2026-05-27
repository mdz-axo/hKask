use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::types::*;
use hkask_mcp::server::validate_tool_url;

#[derive(Default)]
pub struct ProviderSearchOutput {
    pub results: Vec<SearchResult>,
    pub answer_box: Option<AnswerBox>,
    pub related_questions: Vec<String>,
    pub content_previews: HashMap<String, String>,
    pub semantic_scores: HashMap<String, f64>,
}

#[async_trait]
pub(crate) trait WebSearchProvider: Send + Sync {
    fn kind(&self) -> &str;
    fn capabilities(&self) -> Vec<SearchCapability>;
    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

// =============================================================================
// Provider-level URL validation (Task 6: SSRF protection)
//
// Every provider's extract() and browse() calls this before making
// outbound requests. This is a capability boundary: the MCP server
// never sends a request to a URL it hasn't validated.
// =============================================================================

pub fn validate_provider_url(url: &str) -> Result<(), WebError> {
    validate_tool_url(url).map_err(|e| WebError::BadArgs(e.message))
}

// =============================================================================
// WebSearchPort — Application core port (hexagonal boundary)
//
// The application core depends on this trait, not on ProviderPool directly.
// ProviderPool implements it as the adapter. This decouples tool handlers
// from concrete provider infrastructure.
// =============================================================================

/// Port trait for web search operations at the application core boundary.
///
/// Tool handlers depend on this trait; `ProviderPool` implements it as the
/// adapter. This keeps provider-specific details (like `pool.exa` direct
/// access) out of the tool layer.
#[async_trait]
pub trait WebSearchPort: Send + Sync {
    async fn search(
        &self,
        query: &SearchQuery,
        strategy: SearchStrategy,
        ctx: Option<&CapabilityContext>,
    ) -> Result<CompoundSearchResult, WebError>;
    async fn find_similar(
        &self,
        url: &str,
        num_results: u32,
        ctx: Option<&CapabilityContext>,
    ) -> Result<ProviderSearchOutput, WebError>;
    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
        ctx: Option<&CapabilityContext>,
    ) -> Result<ExtractedContent, WebError>;
    async fn browse(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
        ctx: Option<&CapabilityContext>,
    ) -> Result<BrowseResult, WebError>;
    async fn health_check(&self) -> Vec<ProviderHealthEntry>;
    fn provider_fingerprint(&self) -> String;
}

#[async_trait]
pub(crate) trait WebExtractProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn extract(&self, url: &str, opts: &ExtractOptions)
    -> Result<ExtractedContent, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
pub(crate) trait WebBrowseProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn browse(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
    ) -> Result<BrowseResult, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

pub struct ProviderPool {
    pub(crate) search_providers: Vec<Box<dyn WebSearchProvider>>,
    pub(crate) extract_providers: Vec<Box<dyn WebExtractProvider>>,
    pub(crate) browse_providers: Vec<Box<dyn WebBrowseProvider>>,
    pub(crate) exa: Option<ExaProvider>,
    #[allow(dead_code)] // Task 2: Wired for future per-request credential resolution
    pub(crate) credential_resolver: Arc<dyn CredentialResolver>,
}

impl ProviderPool {
    /// Construct a new `ProviderPool` with the given providers and credential resolver.
    ///
    /// This is the authoritative constructor — all pool creation should go through
    /// here rather than setting fields directly, to maintain the hexagonal boundary.
    pub(crate) fn new(
        search_providers: Vec<Box<dyn WebSearchProvider>>,
        extract_providers: Vec<Box<dyn WebExtractProvider>>,
        browse_providers: Vec<Box<dyn WebBrowseProvider>>,
        exa: Option<ExaProvider>,
        credential_resolver: Arc<dyn CredentialResolver>,
    ) -> Self {
        Self {
            search_providers,
            extract_providers,
            browse_providers,
            exa,
            credential_resolver,
        }
    }
}

impl ProviderPool {
    pub async fn search_with_fallback(
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
                Ok(output) => return Ok(output.results),
                Err(e) => {
                    tracing::warn!(provider = provider.kind(), error = %e, "Search provider failed, trying next");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    pub async fn search_by_capability(
        &self,
        query: &SearchQuery,
        required_caps: &[SearchCapability],
    ) -> Result<Vec<SearchResult>, WebError> {
        let filtered: Vec<&dyn WebSearchProvider> = self
            .search_providers
            .iter()
            .filter(|p| {
                let p_caps = p.capabilities();
                required_caps.iter().all(|c| p_caps.contains(c))
            })
            .map(|p| p.as_ref())
            .collect();

        let mut last_err = WebError::NoProvider;
        for provider in filtered {
            match provider.search(query).await {
                Ok(output) => return Ok(output.results),
                Err(e) => {
                    tracing::warn!(provider = provider.kind(), error = %e, "Capability search provider failed");
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    pub async fn search_compound(
        &self,
        query: &SearchQuery,
        strategy: SearchStrategy,
    ) -> CompoundSearchResult {
        let filtered: Vec<&dyn WebSearchProvider> = match strategy.provider_filter() {
            ProviderFilter::All => self.search_providers.iter().map(|p| p.as_ref()).collect(),
            ProviderFilter::Capabilities(caps) => self
                .search_providers
                .iter()
                .filter(|p| {
                    let p_caps = p.capabilities();
                    caps.iter().all(|c| p_caps.contains(c))
                })
                .map(|p| p.as_ref())
                .collect(),
            ProviderFilter::Kinds(kinds) => self
                .search_providers
                .iter()
                .filter(|p| kinds.contains(&p.kind()))
                .map(|p| p.as_ref())
                .collect(),
        };

        let providers_queried: Vec<ProviderInfo> = filtered
            .iter()
            .map(|p| ProviderInfo {
                kind: p.kind().to_string(),
                capabilities: p.capabilities(),
            })
            .collect();

        let futures: Vec<_> = filtered
            .iter()
            .map(|p| async { (p.kind().to_string(), p.search(query).await) })
            .collect();

        let results = futures_util::future::join_all(futures).await;

        let mut succeeded: Vec<String> = Vec::new();
        let mut failed: Vec<ProviderError> = Vec::new();
        let mut all_results: Vec<(String, usize, SearchResult)> = Vec::new();
        let mut merged_answer_box: Option<AnswerBox> = None;
        let mut merged_related_questions: Vec<String> = Vec::new();
        let mut merged_content_previews: HashMap<String, String> = HashMap::new();
        let mut merged_semantic_scores: HashMap<String, f64> = HashMap::new();

        for (kind, result) in results {
            match result {
                Ok(output) => {
                    for (rank, item) in output.results.into_iter().enumerate() {
                        all_results.push((kind.clone(), rank, item));
                    }
                    if output.answer_box.is_some() && merged_answer_box.is_none() {
                        merged_answer_box = output.answer_box;
                    }
                    merged_related_questions.extend(output.related_questions);
                    merged_content_previews.extend(output.content_previews);
                    merged_semantic_scores.extend(output.semantic_scores);
                    succeeded.push(kind);
                }
                Err(e) => {
                    tracing::warn!(provider = %kind, error = %e, "Compound search provider failed");
                    failed.push(ProviderError {
                        kind: kind.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        let total_before_dedup = all_results.len();

        struct UrlEntry {
            url_original: String,
            title: String,
            description: Option<String>,
            source: Option<String>,
            published: Option<String>,
            providers: Vec<String>,
            ranks: Vec<usize>,
        }

        let mut url_map: HashMap<String, UrlEntry> = HashMap::new();

        for (provider, rank, result) in all_results {
            let key = result.url.to_lowercase();
            match url_map.get_mut(&key) {
                Some(entry) => {
                    entry.providers.push(provider);
                    entry.ranks.push(rank);
                }
                None => {
                    url_map.insert(
                        key,
                        UrlEntry {
                            url_original: result.url,
                            title: result.title,
                            description: result.description,
                            source: result.source,
                            published: result.published,
                            providers: vec![provider],
                            ranks: vec![rank],
                        },
                    );
                }
            }
        }

        let mut ranked: Vec<RankedResult> = url_map
            .into_iter()
            .map(|(key, entry)| {
                let provider_count = entry.providers.len();
                let best_rank = *entry.ranks.iter().min().unwrap_or(&0);
                let content_preview = merged_content_previews.get(&key).cloned();
                let semantic_score = merged_semantic_scores.get(&key).copied();
                let rrf_score = rrf_score(&entry.ranks);

                RankedResult {
                    title: entry.title,
                    url: entry.url_original,
                    description: entry.description,
                    source: entry.source,
                    published: entry.published,
                    rrf_score,
                    provider_count,
                    providers: entry.providers,
                    best_rank: Some(best_rank),
                    content_preview,
                    semantic_score,
                    extracted_content: None,
                }
            })
            .collect();

        ranked.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let duplicates_removed = total_before_dedup - ranked.len();

        CompoundSearchResult {
            query: query.query.clone(),
            strategy: strategy.to_string(),
            results: ranked,
            answer_box: merged_answer_box,
            related_questions: merged_related_questions,
            providers_queried,
            providers_succeeded: succeeded,
            providers_failed: failed,
            total_before_dedup,
            duplicates_removed,
        }
    }

    pub async fn find_similar(
        &self,
        url: &str,
        num_results: u32,
    ) -> Result<ProviderSearchOutput, WebError> {
        match self.exa {
            Some(ref exa) => exa.find_similar(url, num_results).await,
            None => Err(WebError::NoProvider),
        }
    }

    pub async fn extract_with_fallback(
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

    pub async fn browse_with_fallback(
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

    pub fn search_provider_kinds(&self) -> Vec<String> {
        self.search_providers
            .iter()
            .map(|p| p.kind().to_string())
            .collect()
    }

    pub fn extract_provider_kinds(&self) -> Vec<String> {
        self.extract_providers
            .iter()
            .map(|p| p.kind().to_string())
            .collect()
    }

    pub fn browse_provider_kinds(&self) -> Vec<String> {
        self.browse_providers
            .iter()
            .map(|p| p.kind().to_string())
            .collect()
    }

    pub fn provider_fingerprint(&self) -> String {
        let mut kinds: Vec<String> = self.search_provider_kinds();
        kinds.extend(self.extract_provider_kinds());
        kinds.extend(self.browse_provider_kinds());
        if self.exa.is_some() {
            kinds.push("exa-similar".into());
        }
        kinds.sort();
        kinds.join(",")
    }

    pub async fn health_check_all(&self) -> Vec<ProviderHealthEntry> {
        let mut entries = Vec::new();

        let search_futures: Vec<_> = self
            .search_providers
            .iter()
            .map(|p| async { (p.kind().to_string(), p.health().await) })
            .collect();
        for (kind, result) in futures_util::future::join_all(search_futures).await {
            entries.push(ProviderHealthEntry {
                kind,
                healthy: result.is_ok(),
                error: result.err().map(|e| sanitize_health_error(&e.to_string())),
            });
        }

        let extract_futures: Vec<_> = self
            .extract_providers
            .iter()
            .map(|p| async { (p.kind().to_string(), p.health().await) })
            .collect();
        for (kind, result) in futures_util::future::join_all(extract_futures).await {
            entries.push(ProviderHealthEntry {
                kind,
                healthy: result.is_ok(),
                error: result.err().map(|e| sanitize_health_error(&e.to_string())),
            });
        }

        let browse_futures: Vec<_> = self
            .browse_providers
            .iter()
            .map(|p| async { (p.kind().to_string(), p.health().await) })
            .collect();
        for (kind, result) in futures_util::future::join_all(browse_futures).await {
            entries.push(ProviderHealthEntry {
                kind,
                healthy: result.is_ok(),
                error: result.err().map(|e| sanitize_health_error(&e.to_string())),
            });
        }

        if let Some(ref exa) = self.exa {
            let result = exa.health().await;
            entries.push(ProviderHealthEntry {
                kind: "exa-similar".into(),
                healthy: result.is_ok(),
                error: result.err().map(|e| sanitize_health_error(&e.to_string())),
            });
        }

        entries
    }
}

// =============================================================================
// WebSearchPort implementation - ProviderPool as the adapter
// =============================================================================

#[async_trait]
impl WebSearchPort for ProviderPool {
    async fn search(
        &self,
        query: &SearchQuery,
        strategy: SearchStrategy,
        ctx: Option<&CapabilityContext>,
    ) -> Result<CompoundSearchResult, WebError> {
        // Task 1: CapabilityContext enforcement at port boundary
        if let Some(ctx) = ctx {
            if !ctx.allows("web_search") {
                return Err(WebError::ProviderUnavailable(
                    "capability not authorized".into(),
                ));
            }
        }

        // Task 8: Validation at port boundary (authoritative enforcement point)
        if query.query.is_empty() {
            return Err(WebError::BadArgs("query must not be empty".into()));
        }
        if query.query.len() > MAX_QUERY_LENGTH {
            return Err(WebError::BadArgs(format!(
                "query exceeds maximum length of {} characters",
                MAX_QUERY_LENGTH
            )));
        }

        // Build a query with depth set based on strategy
        let query_with_depth = SearchQuery {
            depth: match strategy {
                SearchStrategy::Deep => SearchDepth::Advanced,
                _ => SearchDepth::Basic,
            },
            ..query.clone()
        };

        let mut compound = if strategy == SearchStrategy::Quick {
            let results = self
                .search_by_capability(&query_with_depth, &[SearchCapability::Keyword])
                .await?;
            let provider_name = self
                .search_providers
                .iter()
                .find(|p| {
                    let caps = p.capabilities();
                    caps.contains(&SearchCapability::Keyword)
                })
                .map(|p| p.kind())
                .unwrap_or("unknown");
            CompoundSearchResult {
                query: query.query.clone(),
                strategy: strategy.to_string(),
                results: results
                    .into_iter()
                    .enumerate()
                    .map(|(i, r)| RankedResult {
                        title: r.title,
                        url: r.url,
                        description: r.description,
                        source: r.source,
                        published: r.published,
                        rrf_score: rrf_score(&[i]),
                        provider_count: 1,
                        providers: vec![provider_name.to_string()],
                        best_rank: Some(i),
                        content_preview: None,
                        semantic_score: None,
                        extracted_content: None,
                    })
                    .collect(),
                answer_box: None,
                related_questions: Vec::new(),
                providers_queried: vec![ProviderInfo {
                    kind: provider_name.to_string(),
                    capabilities: vec![SearchCapability::Keyword],
                }],
                providers_succeeded: vec![provider_name.to_string()],
                providers_failed: Vec::new(),
                total_before_dedup: 0,
                duplicates_removed: 0,
            }
        } else {
            self.search_compound(&query_with_depth, strategy).await
        };

        apply_rerank(&mut compound.results, RerankSignal::Recency);
        apply_rerank(&mut compound.results, RerankSignal::Semantic);
        apply_rerank(&mut compound.results, RerankSignal::ContentQuality);

        Ok(compound)
    }

    async fn find_similar(
        &self,
        url: &str,
        num_results: u32,
        ctx: Option<&CapabilityContext>,
    ) -> Result<ProviderSearchOutput, WebError> {
        // Task 1: CapabilityContext enforcement at port boundary
        if let Some(ctx) = ctx {
            if !ctx.allows("web_find_similar") {
                return Err(WebError::ProviderUnavailable(
                    "capability not authorized".into(),
                ));
            }
        }
        match self.exa {
            Some(ref exa) => exa.find_similar(url, num_results).await,
            None => Err(WebError::NoProvider),
        }
    }

    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
        ctx: Option<&CapabilityContext>,
    ) -> Result<ExtractedContent, WebError> {
        // Task 1: CapabilityContext enforcement at port boundary
        if let Some(ctx) = ctx {
            if !ctx.allows("web_extract") {
                return Err(WebError::ProviderUnavailable(
                    "capability not authorized".into(),
                ));
            }
        }
        self.extract_with_fallback(url, opts).await
    }

    async fn browse(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
        ctx: Option<&CapabilityContext>,
    ) -> Result<BrowseResult, WebError> {
        // Task 1: CapabilityContext enforcement at port boundary
        if let Some(ctx) = ctx {
            if !ctx.allows("web_browse") {
                return Err(WebError::ProviderUnavailable(
                    "capability not authorized".into(),
                ));
            }
        }
        self.browse_with_fallback(url, instruction, timeout).await
    }

    async fn health_check(&self) -> Vec<ProviderHealthEntry> {
        self.health_check_all().await
    }

    fn provider_fingerprint(&self) -> String {
        ProviderPool::provider_fingerprint(self)
    }
}

pub struct BraveProvider {
    client: reqwest::Client,
    api_key: String,
}

impl BraveProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .timeout(Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS))
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
    fn capabilities(&self) -> Vec<SearchCapability> {
        vec![
            SearchCapability::Keyword,
            SearchCapability::News,
            SearchCapability::Freshness,
        ]
    }

    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError> {
        let mut params: Vec<(&str, String)> = vec![
            ("q", query.query.clone()),
            ("count", query.num_results.to_string()),
        ];
        if let Some(ref freshness) = query.freshness {
            params.push(("freshness", freshness_brave(freshness)));
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
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ProviderSearchOutput {
            results,
            ..Default::default()
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        let resp = self
            .client
            .get(format!("{BRAVE_API_BASE}/web/search"))
            .header("X-Subscription-Token", &self.api_key)
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

pub struct FirecrawlProvider {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl FirecrawlProvider {
    pub fn new(api_key: Option<String>) -> Self {
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
    fn capabilities(&self) -> Vec<SearchCapability> {
        vec![SearchCapability::Keyword, SearchCapability::Semantic]
    }

    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError> {
        let auth = self.auth_header()?;
        let payload = serde_json::json!({ "query": query.query, "limit": query.num_results });
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
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ProviderSearchOutput {
            results,
            ..Default::default()
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
        let mut payload = serde_json::json!({ "url": url });
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
            "url": url, "formats": ["markdown"],
            "actions": [{ "type": "wait", "milliseconds": 2000u64 }],
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

        Ok(BrowseResult {
            url: url.to_string(),
            content: parsed["data"]["markdown"]
                .as_str()
                .unwrap_or("")
                .to_string(),
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

pub struct TavilyProvider {
    client: reqwest::Client,
    api_key: String,
}

impl TavilyProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }
}

#[async_trait]
impl WebSearchProvider for TavilyProvider {
    fn kind(&self) -> &str {
        "tavily"
    }
    fn capabilities(&self) -> Vec<SearchCapability> {
        vec![SearchCapability::Keyword, SearchCapability::Semantic]
    }

    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError> {
        let mut payload = serde_json::json!({
            "api_key": self.api_key,
            "query": query.query,
            "max_results": query.num_results,
            "search_depth": "basic",
        });
        if !query.include_domains.is_empty() {
            payload["include_domains"] = serde_json::json!(query.include_domains);
        }
        if !query.exclude_domains.is_empty() {
            payload["exclude_domains"] = serde_json::json!(query.exclude_domains);
        }

        let resp = self
            .client
            .post(format!("{TAVILY_API_BASE}/search"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Tavily request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => WebError::ProviderUnavailable(format!("Tavily auth error: {status}")),
                429 => WebError::RateLimited(format!("Tavily rate limited: {status}")),
                _ => WebError::ProviderError(format!(
                    "Tavily API error {status}: {}",
                    body.chars().take(200).collect::<String>()
                )),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Tavily response: {e}"))
        })?;

        let mut content_previews = HashMap::new();
        let results = parsed["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let url = item["url"].as_str()?;
                        if let Some(content) = item["content"].as_str() {
                            content_previews.insert(url.to_lowercase(), content.to_string());
                        }
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: url.to_string(),
                            description: item["content"]
                                .as_str()
                                .or_else(|| item["snippet"].as_str())
                                .map(|s| s.to_string()),
                            source: None,
                            published: item["published_date"].as_str().map(|s| s.to_string()),
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ProviderSearchOutput {
            results,
            content_previews,
            ..Default::default()
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        Ok(())
    }
}

pub struct SerapiProvider {
    client: reqwest::Client,
    api_key: String,
}

impl SerapiProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }
}

#[async_trait]
impl WebSearchProvider for SerapiProvider {
    fn kind(&self) -> &str {
        "serpapi"
    }
    fn capabilities(&self) -> Vec<SearchCapability> {
        vec![
            SearchCapability::Keyword,
            SearchCapability::News,
            SearchCapability::Freshness,
        ]
    }

    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError> {
        let mut params: Vec<(&str, String)> = vec![
            ("q", query.query.clone()),
            ("api_key", self.api_key.clone()),
            ("engine", "google".to_string()),
            ("num", query.num_results.to_string()),
            ("output", "json".to_string()),
        ];
        if !query.include_domains.is_empty() {
            params.push(("as_sitesearch", query.include_domains.join(",")));
        }
        if let Some(ref freshness) = query.freshness {
            let tbs = freshness_serpapi(freshness);
            if !tbs.is_empty() {
                params.push(("tbs", tbs));
            }
        }

        let resp = self
            .client
            .get(SERPAPI_BASE)
            .query(&params)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("SerpAPI request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => WebError::ProviderUnavailable(format!("SerpAPI auth error: {status}")),
                429 => WebError::RateLimited(format!("SerpAPI rate limited: {status}")),
                _ => WebError::ProviderError(format!(
                    "SerpAPI error {status}: {}",
                    body.chars().take(200).collect::<String>()
                )),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse SerpAPI response: {e}"))
        })?;

        let organic = parsed["organic_results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: item["link"].as_str()?.to_string(),
                            description: item["snippet"].as_str().map(|s| s.to_string()),
                            source: Some("google".to_string()),
                            published: None,
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let news = parsed["news_results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: item["link"].as_str()?.to_string(),
                            description: item["snippet"].as_str().map(|s| s.to_string()),
                            source: item["source"].as_str().map(|s| s.to_string()),
                            published: item["date"].as_str().map(|s| s.to_string()),
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut results = organic;
        results.extend(news);

        let answer_box = parsed["answer_box"].as_object().map(|ab| AnswerBox {
            title: ab
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            snippet: ab
                .get("snippet")
                .or_else(|| ab.get("answer"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url: ab
                .get("link")
                .or_else(|| ab.get("displayed_link"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        });

        let related_questions: Vec<String> = parsed["related_questions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item["question"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ProviderSearchOutput {
            results,
            answer_box,
            related_questions,
            ..Default::default()
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        Ok(())
    }
}

pub struct ExaProvider {
    client: reqwest::Client,
    api_key: String,
}

impl ExaProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }

    pub async fn find_similar(
        &self,
        url: &str,
        num_results: u32,
    ) -> Result<ProviderSearchOutput, WebError> {
        let payload = serde_json::json!({
            "url": url,
            "numResults": num_results,
            "contents": { "text": { "maxCharacters": 300 } },
        });

        let resp = self
            .client
            .post(format!("{EXA_API_BASE}/findSimilar"))
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Exa findSimilar failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => {
                    WebError::ProviderUnavailable(format!("Exa findSimilar auth error: {status}"))
                }
                429 => WebError::RateLimited(format!("Exa findSimilar rate limited: {status}")),
                _ => WebError::ProviderError(format!(
                    "Exa findSimilar error {status}: {}",
                    body.chars().take(200).collect::<String>()
                )),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Exa findSimilar response: {e}"))
        })?;

        let mut semantic_scores = HashMap::new();
        let mut content_previews = HashMap::new();
        let results = parsed["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let url = item["url"].as_str()?;
                        if let Some(score) = item["score"].as_f64() {
                            semantic_scores.insert(url.to_lowercase(), score);
                        }
                        if let Some(text) = item["text"].as_str() {
                            content_previews.insert(url.to_lowercase(), text.to_string());
                        }
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: url.to_string(),
                            description: item["text"].as_str().map(|s| truncate_str(s, 300)),
                            source: item["author"].as_str().map(|s| s.to_string()),
                            published: item["publishedDate"].as_str().map(|s| s.to_string()),
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ProviderSearchOutput {
            results,
            semantic_scores,
            content_previews,
            ..Default::default()
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        Ok(())
    }
}

#[async_trait]
impl WebSearchProvider for ExaProvider {
    fn kind(&self) -> &str {
        "exa"
    }
    fn capabilities(&self) -> Vec<SearchCapability> {
        vec![SearchCapability::Semantic, SearchCapability::Keyword]
    }

    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError> {
        let mut payload = serde_json::json!({
            "query": query.query,
            "numResults": query.num_results,
            "type": "neural",
            "contents": { "text": { "maxCharacters": 300 } },
        });
        if !query.include_domains.is_empty() {
            payload["includeDomains"] = serde_json::json!(query.include_domains);
        }
        if !query.exclude_domains.is_empty() {
            payload["excludeDomains"] = serde_json::json!(query.exclude_domains);
        }

        let resp = self
            .client
            .post(format!("{EXA_API_BASE}/search"))
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("Exa request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => WebError::ProviderUnavailable(format!("Exa auth error: {status}")),
                429 => WebError::RateLimited(format!("Exa rate limited: {status}")),
                _ => WebError::ProviderError(format!(
                    "Exa API error {status}: {}",
                    body.chars().take(200).collect::<String>()
                )),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| WebError::ProviderError(format!("Failed to parse Exa response: {e}")))?;

        let mut semantic_scores = HashMap::new();
        let mut content_previews = HashMap::new();
        let results = parsed["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let url = item["url"].as_str()?;
                        if let Some(score) = item["score"].as_f64() {
                            semantic_scores.insert(url.to_lowercase(), score);
                        }
                        if let Some(text) = item["text"].as_str() {
                            content_previews.insert(url.to_lowercase(), text.to_string());
                        }
                        Some(SearchResult {
                            title: item["title"].as_str()?.to_string(),
                            url: url.to_string(),
                            description: item["text"].as_str().map(|s| truncate_str(s, 300)),
                            source: item["author"].as_str().map(|s| s.to_string()),
                            published: item["publishedDate"].as_str().map(|s| s.to_string()),
                            provider: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ProviderSearchOutput {
            results,
            semantic_scores,
            content_previews,
            ..Default::default()
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        Ok(())
    }
}

pub struct BrowserbaseProvider {
    client: reqwest::Client,
    api_key: String,
}

impl BrowserbaseProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }
}

#[async_trait]
impl WebBrowseProvider for BrowserbaseProvider {
    fn kind(&self) -> &str {
        "browserbase"
    }

    async fn browse(
        &self,
        url: &str,
        instruction: &str,
        timeout: Duration,
    ) -> Result<BrowseResult, WebError> {
        let payload = serde_json::json!({
            "url": url,
            "actions": [{ "type": "wait", "milliseconds": 2000u64 }],
        });

        let resp = self
            .client
            .post(format!("{BROWSERBASE_API_BASE}/sessions"))
            .header("x-bb-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| {
                WebError::ProviderUnavailable(format!("Browserbase request failed: {e}"))
            })?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(WebError::ProviderError(format!(
                "Browserbase error {status}: {}",
                body.chars().take(200).collect::<String>()
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Browserbase response: {e}"))
        })?;

        let content = parsed["content"]
            .as_str()
            .or_else(|| parsed["data"]["markdown"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(BrowseResult {
            url: url.to_string(),
            content,
            instruction: Some(instruction.to_string()),
            actions_taken: vec!["headless_browse".to_string()],
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        // Task 1: Substantive health check — liveness test via example.com
        let resp = self
            .client
            .get("https://api.browserbase.com/v1/sessions")
            .header("x-bb-api-key", &self.api_key)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                WebError::ProviderUnavailable(format!("Browserbase health check failed: {e}"))
            })?;
        let status = resp.status();
        if status.is_success() || status.as_u16() == 429 {
            Ok(())
        } else if status.as_u16() == 401 || status.as_u16() == 403 {
            Err(WebError::ProviderUnavailable(
                "Browserbase authentication failed".to_string(),
            ))
        } else {
            Err(WebError::ProviderUnavailable(format!(
                "Browserbase health check returned {status}"
            )))
        }
    }
}

pub struct RawFetchProvider {
    client: reqwest::Client,
}

impl Default for RawFetchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RawFetchProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("hkask-mcp-web/{SERVER_VERSION}"))
            .timeout(Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS))
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
        // Task 6: Validate URL at provider boundary — RawFetch is the most SSRF-sensitive provider
        validate_provider_url(url)?;
        let resp =
            self.client.get(url).send().await.map_err(|e| {
                WebError::ProviderUnavailable(format!("RawFetch request failed: {e}"))
            })?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| WebError::ProviderError(format!("RawFetch read error: {e}")))?;
        if !status.is_success() {
            return Err(WebError::ProviderError(format!(
                "RawFetch error {status}: {}",
                body.chars().take(200).collect::<String>()
            )));
        }
        Ok(ExtractedContent {
            url: url.to_string(),
            content: super::strip_html::strip_html(&body),
            format: "markdown".to_string(),
            metadata: None,
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        // Task 1: Liveness check — fetch example.com
        let resp = self
            .client
            .get("https://example.com")
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                WebError::ProviderUnavailable(format!("RawFetch health check failed: {e}"))
            })?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(WebError::ProviderUnavailable(format!(
                "RawFetch health check returned {}",
                resp.status()
            )))
        }
    }
}

pub fn truncate_str(s: &str, max_len: usize) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_str_short() {
        assert_eq!(truncate_str("hi", 10), "hi");
    }

    #[test]
    fn truncate_str_long() {
        let result = truncate_str("hello world 12345", 8);
        assert!(result.ends_with('…'));
        assert!(result.len() <= 12);
    }
}
