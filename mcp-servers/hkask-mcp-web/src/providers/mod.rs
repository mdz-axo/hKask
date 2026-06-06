use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;

use crate::types::*;
use hkask_mcp::server::validate_tool_url;

mod brave;
mod browserbase;
mod exa;
mod firecrawl;
mod raw_fetch;
mod serapi;
mod tavily;

pub use brave::BraveProvider;
pub use browserbase::BrowserbaseProvider;
pub use exa::ExaProvider;
pub use firecrawl::FirecrawlProvider;
pub use raw_fetch::{RawFetchProvider, truncate_str};
pub use serapi::SerapiProvider;
pub use tavily::TavilyProvider;

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
}

impl ProviderPool {
    /// Construct a new `ProviderPool` with the given providers.
    ///
    /// This is the authoritative constructor — all pool creation should go through
    /// here rather than setting fields directly, to maintain the hexagonal boundary.
    pub(crate) fn new(
        search_providers: Vec<Box<dyn WebSearchProvider>>,
        extract_providers: Vec<Box<dyn WebExtractProvider>>,
        browse_providers: Vec<Box<dyn WebBrowseProvider>>,
        exa: Option<ExaProvider>,
    ) -> Self {
        Self {
            search_providers,
            extract_providers,
            browse_providers,
            exa,
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
                let rrf_score = rrf_score(RRF_K, &entry.ranks);

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

        for p in &self.search_providers {
            let kind = p.kind().to_string();
            let result = p.health().await;
            entries.push(health_entry(kind, result));
        }

        for p in &self.extract_providers {
            let kind = p.kind().to_string();
            let result = p.health().await;
            entries.push(health_entry(kind, result));
        }

        for p in &self.browse_providers {
            let kind = p.kind().to_string();
            let result = p.health().await;
            entries.push(health_entry(kind, result));
        }

        if let Some(ref exa) = self.exa {
            let result = exa.health().await;
            entries.push(health_entry("exa-similar".into(), result));
        }

        entries
    }
}

fn health_entry(kind: String, result: Result<(), WebError>) -> ProviderHealthEntry {
    ProviderHealthEntry {
        kind,
        healthy: result.is_ok(),
        error: result.err().map(|e| sanitize_health_error(&e.to_string())),
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
        if let Some(ctx) = ctx
            && !ctx.allows("web_search")
        {
            return Err(WebError::ProviderUnavailable(
                "capability not authorized".into(),
            ));
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
                        rrf_score: rrf_score(RRF_K, &[i]),
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
        if let Some(ctx) = ctx
            && !ctx.allows("web_find_similar")
        {
            return Err(WebError::ProviderUnavailable(
                "capability not authorized".into(),
            ));
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
        if let Some(ctx) = ctx
            && !ctx.allows("web_extract")
        {
            return Err(WebError::ProviderUnavailable(
                "capability not authorized".into(),
            ));
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
        if let Some(ctx) = ctx
            && !ctx.allows("web_browse")
        {
            return Err(WebError::ProviderUnavailable(
                "capability not authorized".into(),
            ));
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
