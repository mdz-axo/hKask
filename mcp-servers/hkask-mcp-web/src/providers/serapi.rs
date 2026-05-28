use async_trait::async_trait;

use super::{ProviderSearchOutput, WebSearchProvider, WebError};
use crate::types::*;

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
