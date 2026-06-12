use async_trait::async_trait;

use super::{ProviderSearchOutput, WebError, WebSearchProvider};
use crate::types::*;

/// arXiv preprint search provider.
///
/// Free: no API key required. Queries `export.arxiv.org/api/query` which returns
/// Atom XML. Parses entry titles, authors, summaries (abstracts), PDF links,
/// and published dates into SearchResult structs.
pub struct ArxivProvider {
    client: reqwest::Client,
}

impl ArxivProvider {
    pub fn new() -> Self {
        Self {
            client: super::provider_http_client(),
        }
    }
}

impl Default for ArxivProvider {
    fn default() -> Self {
        Self::new()
    }
}

const ARXIV_API_BASE: &str = "https://export.arxiv.org/api/query";

#[async_trait]
impl WebSearchProvider for ArxivProvider {
    fn kind(&self) -> &str {
        "arxiv"
    }
    fn capabilities(&self) -> Vec<SearchCapability> {
        vec![SearchCapability::Keyword, SearchCapability::Semantic]
    }

    async fn search(&self, query: &SearchQuery) -> Result<ProviderSearchOutput, WebError> {
        // arXiv API uses a Lucene-like search syntax. We pass the raw query
        // and let the user formulate author/title searches (e.g. 'au:dunning').
        let params: Vec<(&str, String)> = vec![
            ("search_query", query.query.clone()),
            ("max_results", query.num_results.to_string()),
            ("sortBy", "relevance".to_string()),
        ];

        let resp = self
            .client
            .get(ARXIV_API_BASE)
            .query(&params)
            .send()
            .await
            .map_err(|e| WebError::ProviderUnavailable(format!("arXiv request failed: {e}")))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(match status.as_u16() {
                429 => WebError::RateLimited(format!("arXiv rate limited: {status}")),
                503 => WebError::ProviderUnavailable(format!(
                    "arXiv temporarily unavailable: {status}"
                )),
                _ => WebError::ProviderError(format!(
                    "arXiv error {status}: {}",
                    body.chars().take(200).collect::<String>()
                )),
            });
        }

        // Parse Atom XML — extract entries manually to avoid heavy XML deps
        let results = parse_arxiv_atom(&body);

        Ok(ProviderSearchOutput {
            results,
            ..Default::default()
        })
    }

    async fn health(&self) -> Result<(), WebError> {
        let resp = self
            .client
            .get(ARXIV_API_BASE)
            .query(&[("search_query", "all:test"), ("max_results", "1")])
            .send()
            .await
            .map_err(|e| {
                WebError::ProviderUnavailable(format!("arXiv health check failed: {e}"))
            })?;
        if resp.status().is_success() || resp.status().as_u16() == 503 {
            // 503 is arXiv's "busy" response — still alive
            Ok(())
        } else {
            Err(WebError::ProviderUnavailable(format!(
                "arXiv health check returned {}",
                resp.status()
            )))
        }
    }
}

/// Parse arXiv Atom XML response into SearchResults.
///
/// Avoids pulling in a full XML parser — arXiv's Atom format is predictable
/// enough for simple string extraction between known tags.
fn parse_arxiv_atom(xml: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Split on <entry> tags to isolate each paper
    for entry_str in xml.split("<entry>").skip(1) {
        // Close the entry at </entry>
        let entry = match entry_str.split("</entry>").next() {
            Some(e) => e,
            None => continue,
        };

        let title = extract_tag(entry, "title");
        let summary = extract_tag(entry, "summary");
        let published = extract_tag(entry, "published");

        // Extract authors from <author><name>...</name></author> blocks
        let authors: Vec<String> = entry_str
            .split("<author>")
            .skip(1)
            .filter_map(|auth_block| {
                let name = extract_tag(auth_block, "name");
                if name.is_empty() { None } else { Some(name) }
            })
            .collect();

        // arXiv ID from <id> tag (full URL like https://arxiv.org/abs/...)
        let arxiv_url = extract_tag(entry, "id");
        // PDF link from <link title="pdf" href="..."/>
        let pdf_url = entry
            .lines()
            .find(|line| line.contains("title=\"pdf\""))
            .and_then(|line| {
                let start = line.find("href=\"")? + 6;
                let end = line[start..].find('"')?;
                Some(line[start..start + end].to_string())
            });

        let url = if !pdf_url.as_ref().is_none_or(|u| u.is_empty()) {
            pdf_url.unwrap_or(arxiv_url)
        } else {
            arxiv_url
        };

        if title.is_empty() {
            continue;
        }

        // Build description: authors + date + abstract snippet
        let mut desc_parts: Vec<String> = Vec::new();
        if !authors.is_empty() {
            desc_parts.push(authors.join(", "));
        }
        if !published.is_empty() {
            // Extract just the date part (YYYY-MM-DD)
            let date = published
                .split('T')
                .next()
                .unwrap_or(&published)
                .to_string();
            desc_parts.push(format!("({date})"));
        }
        if !summary.is_empty() {
            let short_summary: String = summary.chars().take(300).collect();
            desc_parts.push(short_summary);
        }

        let description = if desc_parts.is_empty() {
            None
        } else {
            Some(desc_parts.join(" — "))
        };

        results.push(SearchResult {
            title,
            url,
            description,
            source: Some("arXiv".to_string()),
            published: if published.is_empty() {
                None
            } else {
                Some(published.split('T').next().unwrap_or("").to_string())
            },
            provider: Some("arxiv".to_string()),
        });
    }

    results
}

/// Extract text content between XML tags like <title>...</title>.
/// Strips leading/trailing whitespace and normalizes newlines.
fn extract_tag(xml: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");

    let start = match xml.find(&open) {
        Some(pos) => pos + open.len(),
        None => return String::new(),
    };
    let end = match xml[start..].find(&close) {
        Some(pos) => start + pos,
        None => return String::new(),
    };

    xml[start..end]
        .trim()
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
