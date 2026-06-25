//! Multi-provider fundamental research engine.
//!
//! Searches across Exa, Tavily, and Brave for company-specific
//! financial claims. This is a thin, pure search layer — the LLM
//! handles classification and extraction via templates.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Research result types ──────────────────────────────────────────────

/// A single research claim from search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaim {
    /// The claim text (e.g., "App Store revenue grew 12% YoY in Q2")
    pub text: String,
    /// Source URL
    pub source: String,
    /// Source title/domain
    pub source_title: String,
    /// Date if available
    pub date: Option<String>,
    /// Which provider found this claim
    pub provider: String,
}

/// Aggregated research results for a company.
#[derive(Debug, Clone, Serialize)]
pub struct ResearchResult {
    pub query: String,
    pub claims: Vec<ResearchClaim>,
    pub provider_summary: Vec<ProviderSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub provider: String,
    pub claims_found: usize,
    pub status: String, // "ok" or "error: ..."
}

// ── Multi-provider search ──────────────────────────────────────────────

/// Search for company research across all available providers.
/// Returns aggregated, de-duplicated claims.
///
/// Each optional API key enables the corresponding provider. Providers
/// without a key are silently skipped.
pub async fn search_fundamental(
    client: &reqwest::Client,
    symbol: &str,
    company_name: &str,
    research_query: &str,
    exa_key: Option<&str>,
    tavily_key: Option<&str>,
    brave_key: Option<&str>,
) -> ResearchResult {
    let query = format!(
        "{} {} {}",
        symbol.trim(),
        company_name.trim(),
        research_query.trim()
    );

    // Run all available providers in parallel
    let (exa_result, tavily_result, brave_result) = tokio::join!(
        async {
            match exa_key.filter(|k| !k.is_empty()) {
                Some(key) => search_exa(client, &query, key).await,
                None => Ok(Vec::new()),
            }
        },
        async {
            match tavily_key.filter(|k| !k.is_empty()) {
                Some(key) => search_tavily(client, &query, key).await,
                None => Ok(Vec::new()),
            }
        },
        async {
            match brave_key.filter(|k| !k.is_empty()) {
                Some(key) => search_brave(client, &query, key).await,
                None => Ok(Vec::new()),
            }
        },
    );

    // Collect all claims, tracking which providers contributed
    let mut all_claims: Vec<ResearchClaim> = Vec::new();
    let mut provider_summary: Vec<ProviderSummary> = Vec::new();

    for (provider_name, result) in [
        ("exa", exa_result),
        ("tavily", tavily_result),
        ("brave", brave_result),
    ] {
        match result {
            Ok(claims) => {
                let count = claims.len();
                all_claims.extend(claims);
                provider_summary.push(ProviderSummary {
                    provider: provider_name.to_string(),
                    claims_found: count,
                    status: "ok".to_string(),
                });
            }
            Err(e) => {
                provider_summary.push(ProviderSummary {
                    provider: provider_name.to_string(),
                    claims_found: 0,
                    status: format!("error: {e}"),
                });
            }
        }
    }

    // De-duplicate by source URL (keep first occurrence)
    let mut seen_urls = std::collections::HashSet::new();
    let mut deduped_claims: Vec<ResearchClaim> = Vec::new();
    for claim in all_claims {
        if seen_urls.insert(claim.source.clone()) {
            deduped_claims.push(claim);
        }
    }

    ResearchResult {
        query,
        claims: deduped_claims,
        provider_summary,
    }
}

// ── Exa search ────────────────────────────────────────────────────────

/// Search Exa API and extract claims from results.
async fn search_exa(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
) -> Result<Vec<ResearchClaim>, String> {
    let url = "https://api.exa.ai/search";
    let body = serde_json::json!({
        "query": query,
        "numResults": 5,
        "type": "auto",
        "contents": {"text": true}
    });

    let resp = client
        .post(url)
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Exa request failed: {e}"))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("Exa returned {status}: {body_text}"));
    }

    let parsed: Value =
        serde_json::from_str(&body_text).map_err(|e| format!("Exa parse error: {e}"))?;

    parse_exa_results(&parsed)
}

fn parse_exa_results(parsed: &Value) -> Result<Vec<ResearchClaim>, String> {
    let results = parsed["results"]
        .as_array()
        .ok_or("Exa response missing 'results' array")?;

    let mut claims = Vec::new();
    for result in results {
        let text = result["text"].as_str().unwrap_or("").to_string();
        let source = result["url"].as_str().unwrap_or("").to_string();
        let source_title = result["title"].as_str().unwrap_or("").to_string();
        let date = result["publishedDate"].as_str().map(|s| s.to_string());

        if text.is_empty() && source.is_empty() {
            continue;
        }

        // Truncate very long texts to a reasonable snippet length
        let snippet = if text.len() > 2000 {
            format!("{}…", &text[..2000])
        } else {
            text
        };

        claims.push(ResearchClaim {
            text: snippet,
            source,
            source_title,
            date,
            provider: "exa".to_string(),
        });
    }

    Ok(claims)
}

// ── Tavily search ─────────────────────────────────────────────────────

/// Search Tavily API and extract claims from results.
async fn search_tavily(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
) -> Result<Vec<ResearchClaim>, String> {
    let url = "https://api.tavily.com/search";
    let body = serde_json::json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "advanced",
        "max_results": 5,
        "include_raw_content": true
    });

    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Tavily request failed: {e}"))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("Tavily returned {status}: {body_text}"));
    }

    let parsed: Value =
        serde_json::from_str(&body_text).map_err(|e| format!("Tavily parse error: {e}"))?;

    parse_tavily_results(&parsed)
}

fn parse_tavily_results(parsed: &Value) -> Result<Vec<ResearchClaim>, String> {
    let results = parsed["results"]
        .as_array()
        .ok_or("Tavily response missing 'results' array")?;

    let mut claims = Vec::new();
    for result in results {
        let content = result["content"].as_str().unwrap_or("").to_string();
        let source = result["url"].as_str().unwrap_or("").to_string();
        let source_title = result["title"].as_str().unwrap_or("").to_string();

        if content.is_empty() && source.is_empty() {
            continue;
        }

        let snippet = if content.len() > 2000 {
            format!("{}…", &content[..2000])
        } else {
            content
        };

        claims.push(ResearchClaim {
            text: snippet,
            source,
            source_title,
            date: None,
            provider: "tavily".to_string(),
        });
    }

    Ok(claims)
}

// ── Brave search ──────────────────────────────────────────────────────

/// Search Brave Search API and extract claims from results.
async fn search_brave(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
) -> Result<Vec<ResearchClaim>, String> {
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count=5",
        urlencoding(query)
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .send()
        .await
        .map_err(|e| format!("Brave request failed: {e}"))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("Brave returned {status}: {body_text}"));
    }

    let parsed: Value =
        serde_json::from_str(&body_text).map_err(|e| format!("Brave parse error: {e}"))?;

    parse_brave_results(&parsed)
}

fn urlencoding(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push('%');
                encoded.push(hex_char(byte >> 4));
                encoded.push(hex_char(byte & 0x0F));
            }
        }
    }
    encoded
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

fn parse_brave_results(parsed: &Value) -> Result<Vec<ResearchClaim>, String> {
    let results = parsed["web"]["results"]
        .as_array()
        .ok_or("Brave response missing 'web.results' array")?;

    let mut claims = Vec::new();
    for result in results {
        let text = result["description"].as_str().unwrap_or("").to_string();
        let source = result["url"].as_str().unwrap_or("").to_string();
        let source_title = result["title"].as_str().unwrap_or("").to_string();

        if text.is_empty() && source.is_empty() {
            continue;
        }

        let snippet = if text.len() > 2000 {
            format!("{}…", &text[..2000])
        } else {
            text
        };

        claims.push(ResearchClaim {
            text: snippet,
            source,
            source_title,
            date: None,
            provider: "brave".to_string(),
        });
    }

    Ok(claims)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_claim_serialization() {
        let claim = ResearchClaim {
            text: "App Store revenue grew 12% YoY in Q2".to_string(),
            source: "https://example.com/report".to_string(),
            source_title: "Example Report".to_string(),
            date: Some("2025-06-01".to_string()),
            provider: "exa".to_string(),
        };

        let json = serde_json::to_string(&claim).expect("serialization should succeed");
        let parsed: ResearchClaim =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(parsed.text, claim.text);
        assert_eq!(parsed.source, claim.source);
        assert_eq!(parsed.date, Some("2025-06-01".to_string()));
        assert_eq!(parsed.provider, "exa");
    }

    #[test]
    fn provider_summary_construction() {
        let ok_summary = ProviderSummary {
            provider: "exa".to_string(),
            claims_found: 5,
            status: "ok".to_string(),
        };
        assert_eq!(ok_summary.provider, "exa");
        assert_eq!(ok_summary.claims_found, 5);
        assert_eq!(ok_summary.status, "ok");

        let err_summary = ProviderSummary {
            provider: "tavily".to_string(),
            claims_found: 0,
            status: "error: timeout".to_string(),
        };
        assert!(err_summary.status.starts_with("error:"));
    }

    #[test]
    fn parse_exa_results_basic() {
        let json = serde_json::json!({
            "results": [{
                "text": "Revenue grew 15% YoY",
                "url": "https://example.com/1",
                "title": "Report 1",
                "publishedDate": "2025-06-01"
            }]
        });
        let claims = parse_exa_results(&json).expect("should parse");
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].text, "Revenue grew 15% YoY");
        assert_eq!(claims[0].source, "https://example.com/1");
        assert_eq!(claims[0].source_title, "Report 1");
        assert_eq!(claims[0].date, Some("2025-06-01".to_string()));
        assert_eq!(claims[0].provider, "exa");
    }

    #[test]
    fn parse_brave_results_basic() {
        let json = serde_json::json!({
            "web": {
                "results": [{
                    "description": "Apple reported record earnings",
                    "url": "https://example.com/2",
                    "title": "Article 2"
                }]
            }
        });
        let claims = parse_brave_results(&json).expect("should parse");
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].text, "Apple reported record earnings");
        assert_eq!(claims[0].source, "https://example.com/2");
        assert_eq!(claims[0].source_title, "Article 2");
        assert_eq!(claims[0].date, None);
        assert_eq!(claims[0].provider, "brave");
    }

    #[test]
    fn urlencoding_basic() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(
            urlencoding("AAPL revenue guidance"),
            "AAPL+revenue+guidance"
        );
        assert_eq!(urlencoding("test&foo=bar"), "test%26foo%3Dbar");
    }
}
