//! hKask MCP Web — Outbound port traits and provider implementations

use async_trait::async_trait;
use std::time::Duration;

use super::types::*;

#[async_trait]
#[allow(dead_code)]
pub trait WebSearchProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
#[allow(dead_code)]
pub trait WebExtractProvider: Send + Sync {
    fn kind(&self) -> &str;
    async fn extract(&self, url: &str, opts: &ExtractOptions)
    -> Result<ExtractedContent, WebError>;
    async fn health(&self) -> Result<(), WebError>;
}

#[async_trait]
#[allow(dead_code)]
pub trait WebBrowseProvider: Send + Sync {
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
    pub search_providers: Vec<Box<dyn WebSearchProvider>>,
    pub extract_providers: Vec<Box<dyn WebExtractProvider>>,
    pub browse_providers: Vec<Box<dyn WebBrowseProvider>>,
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
                Ok(results) => return Ok(results),
                Err(e) => {
                    tracing::warn!(provider = provider.kind(), error = %e, "Search provider failed, trying next");
                    last_err = e;
                }
            }
        }
        Err(last_err)
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
}

pub struct BraveProvider {
    client: reqwest::Client,
    api_key: String,
}

impl BraveProvider {
    pub fn new(api_key: String) -> Self {
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

        Ok(parsed["web"]["results"]
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
            .unwrap_or_default())
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
                401 | 403 => WebError::ProviderUnavailable(format!("Firecrawl auth error: {status}")),
                429 => WebError::RateLimited(format!("Firecrawl rate limited: {status}")),
                _ => WebError::ProviderError(format!("Firecrawl API error {status}")),
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Firecrawl response: {e}"))
        })?;

        Ok(parsed["data"]
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
            .unwrap_or_default())
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
            parsed["data"]["markdown"].as_str().unwrap_or("").to_string()
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
            return Err(WebError::ProviderError(format!("Firecrawl browse error {status}")));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            WebError::ProviderError(format!("Failed to parse Firecrawl browse response: {e}"))
        })?;
        let content = parsed["data"]["markdown"].as_str().unwrap_or("").to_string();

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

pub struct RawFetchProvider {
    client: reqwest::Client,
}

impl RawFetchProvider {
    pub fn new() -> Self {
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
        let resp = self.client.get(url).send().await.map_err(|e| {
            WebError::ProviderUnavailable(format!("RawFetch request failed: {e}"))
        })?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            WebError::ProviderError(format!("RawFetch read error: {e}"))
        })?;
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
        Ok(())
    }
}
