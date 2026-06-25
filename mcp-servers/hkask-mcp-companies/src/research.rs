//! Multi-provider fundamental research engine.
//!
//! Searches across Exa, Tavily, and Brave for company-specific
//! financial claims, competitive signals, and management guidance.
//! All extracted claims are structured for direct mapping to
//! DCF assumption adjustments or scenario probability shifts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Research result types ──────────────────────────────────────────────

/// A single research claim extracted from search results.
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
    /// Direction: "positive", "negative", or "neutral" for the company
    pub direction: String,
    /// Which DCF assumption this claim affects, if any
    pub affected_assumption: Option<String>,
    /// Estimated magnitude (e.g., 0.03 for 3% impact on revenue growth)
    pub magnitude: Option<f64>,
    /// Which provider found this claim
    pub provider: String,
}

/// Aggregated research results for a company.
#[derive(Debug, Clone, Serialize)]
pub struct ResearchResult {
    pub query: String,
    pub claims: Vec<ResearchClaim>,
    pub provider_summary: Vec<ProviderSummary>,
    /// Summary of assumption impacts aggregated across claims
    pub assumption_impacts: Vec<AssumptionImpact>,
    /// Key competitive signals found
    pub competitive_signals: Vec<String>,
    /// Management guidance statements found
    pub guidance_statements: Vec<GuidanceStatement>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub provider: String,
    pub claims_found: usize,
    pub status: String, // "ok" or "error: ..."
}

#[derive(Debug, Clone, Serialize)]
pub struct AssumptionImpact {
    pub assumption: String,
    pub direction: String, // "up", "down", "neutral"
    pub estimated_adjustment: f64,
    pub confidence: f64,
    pub supporting_claims: Vec<usize>, // indices into claims
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidanceStatement {
    pub text: String,
    pub source: String,
    pub metric: String, // "revenue", "margin", "eps", etc.
    pub low_value: Option<f64>,
    pub high_value: Option<f64>,
    pub guidance_type: String, // "management_outlook", "analyst_estimate", "conference_call"
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

    // Run extraction heuristics
    let assumption_impacts = extract_assumption_impacts(&deduped_claims);
    let competitive_signals = extract_competitive_signals(&deduped_claims);
    let guidance_statements = extract_guidance(&deduped_claims);

    ResearchResult {
        query,
        claims: deduped_claims,
        provider_summary,
        assumption_impacts,
        competitive_signals,
        guidance_statements,
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

    parse_exa_results(&parsed, query)
}

fn parse_exa_results(parsed: &Value, _query: &str) -> Result<Vec<ResearchClaim>, String> {
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

        let direction = classify_claim_direction(&snippet);
        let affected_assumption = classify_assumption(&snippet);

        claims.push(ResearchClaim {
            text: snippet,
            source,
            source_title,
            date,
            direction,
            affected_assumption,
            magnitude: None,
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

        let direction = classify_claim_direction(&snippet);
        let affected_assumption = classify_assumption(&snippet);

        claims.push(ResearchClaim {
            text: snippet,
            source,
            source_title,
            date: None,
            direction,
            affected_assumption,
            magnitude: None,
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

        let direction = classify_claim_direction(&snippet);
        let affected_assumption = classify_assumption(&snippet);

        claims.push(ResearchClaim {
            text: snippet,
            source,
            source_title,
            date: None,
            direction,
            affected_assumption,
            magnitude: None,
            provider: "brave".to_string(),
        });
    }

    Ok(claims)
}

// ── Extraction heuristics ─────────────────────────────────────────────

/// Classify a claim's directional sentiment for the company.
pub fn classify_claim_direction(text: &str) -> String {
    let lower = text.to_lowercase();

    let positive_keywords = [
        "grew",
        "increased",
        "raised",
        "beat",
        "exceeded",
        "strong",
        "accelerated",
        "improved",
        "gains",
        "growth",
        "record",
        "expanded",
        "upgraded",
        "outperformed",
    ];
    let negative_keywords = [
        "declined",
        "decreased",
        "missed",
        "weakened",
        "pressure",
        "headwind",
        "loss",
        "dropped",
        "fell",
        "downgraded",
        "struggling",
        "layoff",
        "layoffs",
        "restructuring",
        "impairment",
    ];

    let has_positive = positive_keywords.iter().any(|kw| lower.contains(kw));
    let has_negative = negative_keywords.iter().any(|kw| lower.contains(kw));

    if has_positive && !has_negative {
        "positive".to_string()
    } else if has_negative && !has_positive {
        "negative".to_string()
    } else {
        "neutral".to_string()
    }
}

/// Map claim text to the DCF assumption it most likely affects.
fn classify_assumption(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    let revenue_keywords = [
        "revenue",
        "sales",
        "top line",
        "top-line",
        "demand",
        "customer",
        "subscriber",
        "market share grew",
        "growth accelerated",
    ];
    let margin_keywords = [
        "margin",
        "gross profit",
        "cost",
        "pricing",
        "supply chain",
        "input cost",
        "raw material",
    ];
    let capex_keywords = [
        "capital expenditure",
        "capex",
        "investment",
        "spending",
        "buildout",
        "expansion",
    ];
    let discount_keywords = [
        "interest rate",
        "fed",
        "wacc",
        "cost of capital",
        "risk premium",
        "borrowing cost",
    ];

    // Check in priority order — first match wins
    for kw in &revenue_keywords {
        if lower.contains(kw) {
            return Some("revenue_growth".to_string());
        }
    }
    for kw in &margin_keywords {
        if lower.contains(kw) {
            return Some("gross_margin".to_string());
        }
    }
    for kw in &capex_keywords {
        if lower.contains(kw) {
            return Some("capex_to_revenue".to_string());
        }
    }
    for kw in &discount_keywords {
        if lower.contains(kw) {
            return Some("discount_rate".to_string());
        }
    }

    None
}

/// Extract competitive signals from claims.
fn extract_competitive_signals(claims: &[ResearchClaim]) -> Vec<String> {
    let competitive_keywords = [
        "competitor",
        "market share",
        "entering",
        "disruption",
        "pricing pressure",
        "competitive",
        "rival",
    ];

    let mut signals = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for claim in claims {
        let lower = claim.text.to_lowercase();
        if competitive_keywords.iter().any(|kw| lower.contains(kw)) {
            // Use first 200 chars as signal text
            let signal = if claim.text.len() > 200 {
                claim.text[..200].to_string()
            } else {
                claim.text.clone()
            };
            if seen.insert(signal.clone()) {
                signals.push(signal);
            }
        }
    }

    signals
}

/// Extract aggregated assumption impacts from claims.
pub fn extract_assumption_impacts(claims: &[ResearchClaim]) -> Vec<AssumptionImpact> {
    let assumption_names = [
        "revenue_growth",
        "gross_margin",
        "capex_to_revenue",
        "discount_rate",
    ];

    let mut impacts = Vec::new();

    for &assumption in &assumption_names {
        let supporting: Vec<(usize, f64)> = claims
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.affected_assumption
                    .as_deref()
                    .map_or(false, |a| a == assumption)
            })
            .map(|(i, c)| {
                let weight = match c.direction.as_str() {
                    "positive" => 0.01, // ~1% adjustment per positive claim
                    "negative" => -0.01,
                    _ => 0.0,
                };
                (i, weight)
            })
            .collect();

        if supporting.is_empty() {
            continue;
        }

        let estimated_adjustment: f64 = supporting.iter().map(|(_, w)| w).sum();
        // Clamp to reasonable range
        let estimated_adjustment = estimated_adjustment.clamp(-0.10, 0.10);

        let direction = if estimated_adjustment > 0.005 {
            "up"
        } else if estimated_adjustment < -0.005 {
            "down"
        } else {
            "neutral"
        }
        .to_string();

        // Confidence scales with number of supporting claims (0.0–1.0)
        let confidence = (supporting.len() as f64 / 5.0).min(1.0);
        let supporting_claims: Vec<usize> = supporting.iter().map(|(i, _)| *i).collect();

        impacts.push(AssumptionImpact {
            assumption: assumption.to_string(),
            direction,
            estimated_adjustment,
            confidence,
            supporting_claims,
        });
    }

    impacts
}

/// Extract forward-looking guidance statements from claims.
pub fn extract_guidance(claims: &[ResearchClaim]) -> Vec<GuidanceStatement> {
    let guidance_keywords = [
        "guidance",
        "outlook",
        "expects",
        "forecasts",
        "projects",
        "sees",
        "predicts",
    ];

    let metric_keywords: &[(&str, &str)] = &[
        ("revenue", "revenue"),
        ("margin", "margin"),
        ("eps", "eps"),
        ("ebitda", "ebitda"),
        ("earnings", "earnings"),
    ];

    let guidance_type_map: &[(&str, &str)] = &[
        ("guidance", "management_outlook"),
        ("outlook", "management_outlook"),
        ("expects", "management_outlook"),
        ("sees", "management_outlook"),
        ("forecasts", "analyst_estimate"),
        ("projects", "analyst_estimate"),
        ("predicts", "analyst_estimate"),
    ];

    let mut statements = Vec::new();

    for claim in claims {
        let lower = claim.text.to_lowercase();
        if !guidance_keywords.iter().any(|kw| lower.contains(kw)) {
            continue;
        }

        // Determine metric
        let metric = metric_keywords
            .iter()
            .find(|(kw, _)| lower.contains(kw))
            .map(|(_, m)| m.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Determine guidance type
        let guidance_type = guidance_type_map
            .iter()
            .find(|(kw, _)| lower.contains(kw))
            .map(|(_, t)| t.to_string())
            .unwrap_or_else(|| "management_outlook".to_string());

        statements.push(GuidanceStatement {
            text: claim.text.clone(),
            source: claim.source.clone(),
            metric,
            low_value: None,
            high_value: None,
            guidance_type,
        });
    }

    statements
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
            direction: "positive".to_string(),
            affected_assumption: Some("revenue_growth".to_string()),
            magnitude: Some(0.03),
            provider: "exa".to_string(),
        };

        let json = serde_json::to_string(&claim).expect("serialization should succeed");
        let parsed: ResearchClaim =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(parsed.text, claim.text);
        assert_eq!(parsed.source, claim.source);
        assert_eq!(parsed.direction, "positive");
        assert_eq!(
            parsed.affected_assumption,
            Some("revenue_growth".to_string())
        );
        assert_eq!(parsed.magnitude, Some(0.03));
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
    fn classify_claim_direction_positive() {
        assert_eq!(
            classify_claim_direction("Revenue grew 15% year over year, beating estimates"),
            "positive"
        );
        assert_eq!(
            classify_claim_direction("The company reported record earnings and raised guidance"),
            "positive"
        );
        assert_eq!(
            classify_claim_direction("Strong demand accelerated growth in Q3"),
            "positive"
        );
    }

    #[test]
    fn classify_claim_direction_negative() {
        assert_eq!(
            classify_claim_direction("Revenue declined 8% amid weakening demand"),
            "negative"
        );
        assert_eq!(
            classify_claim_direction("The company missed expectations and announced layoffs"),
            "negative"
        );
        assert_eq!(
            classify_claim_direction("Margin pressure from rising input costs created headwinds"),
            "negative"
        );
    }

    #[test]
    fn classify_claim_direction_neutral() {
        assert_eq!(
            classify_claim_direction("The company reported quarterly results today"),
            "neutral"
        );
        assert_eq!(
            classify_claim_direction("CEO spoke at an industry conference about technology trends"),
            "neutral"
        );
    }

    #[test]
    fn classify_claim_direction_mixed() {
        // When both positive and negative keywords appear, we go neutral
        assert_eq!(
            classify_claim_direction("Revenue grew but margins declined"),
            "neutral"
        );
    }

    #[test]
    fn classify_assumption_revenue() {
        assert_eq!(
            classify_assumption("Revenue grew 12% driven by strong subscriber demand"),
            Some("revenue_growth".to_string())
        );
        assert_eq!(
            classify_assumption("Sales beat expectations as market share expanded"),
            Some("revenue_growth".to_string())
        );
        assert_eq!(
            classify_assumption("Top-line growth accelerated in the second half"),
            Some("revenue_growth".to_string())
        );
    }

    #[test]
    fn classify_assumption_margin() {
        assert_eq!(
            classify_assumption("Gross margin improved due to better pricing power"),
            Some("gross_margin".to_string())
        );
        assert_eq!(
            classify_assumption("Supply chain costs rose, pressuring margins"),
            Some("gross_margin".to_string())
        );
        assert_eq!(
            classify_assumption("Raw material input costs declined sequentially"),
            Some("gross_margin".to_string())
        );
    }

    #[test]
    fn classify_assumption_capex() {
        assert_eq!(
            classify_assumption("Capital expenditure increased for data center buildout"),
            Some("capex_to_revenue".to_string())
        );
        assert_eq!(
            classify_assumption("Capex guidance raised to support expansion plans"),
            Some("capex_to_revenue".to_string())
        );
    }

    #[test]
    fn classify_assumption_discount_rate() {
        assert_eq!(
            classify_assumption("Fed rate hikes raise the WACC for the sector"),
            Some("discount_rate".to_string())
        );
        assert_eq!(
            classify_assumption("Risk premium increased due to geopolitical uncertainty"),
            Some("discount_rate".to_string())
        );
    }

    #[test]
    fn classify_assumption_none() {
        assert_eq!(
            classify_assumption("The CEO gave an interview about company culture"),
            None
        );
        assert_eq!(
            classify_assumption("The office opened a new location in Austin"),
            None
        );
    }

    #[test]
    fn extract_assumption_impacts_aggregation() {
        let claims = vec![
            ResearchClaim {
                text: "Revenue grew 12% YoY".to_string(),
                source: "https://a.com/1".to_string(),
                source_title: "Article 1".to_string(),
                date: None,
                direction: "positive".to_string(),
                affected_assumption: Some("revenue_growth".to_string()),
                magnitude: None,
                provider: "exa".to_string(),
            },
            ResearchClaim {
                text: "Revenue beat estimates".to_string(),
                source: "https://b.com/2".to_string(),
                source_title: "Article 2".to_string(),
                date: None,
                direction: "positive".to_string(),
                affected_assumption: Some("revenue_growth".to_string()),
                magnitude: None,
                provider: "tavily".to_string(),
            },
            ResearchClaim {
                text: "Margins under pressure".to_string(),
                source: "https://c.com/3".to_string(),
                source_title: "Article 3".to_string(),
                date: None,
                direction: "negative".to_string(),
                affected_assumption: Some("gross_margin".to_string()),
                magnitude: None,
                provider: "brave".to_string(),
            },
        ];

        let impacts = extract_assumption_impacts(&claims);
        assert!(!impacts.is_empty());

        // Should have revenue_growth and gross_margin impacts
        let revenue_impact = impacts
            .iter()
            .find(|i| i.assumption == "revenue_growth")
            .expect("should have revenue_growth impact");
        assert!(revenue_impact.estimated_adjustment > 0.0);
        assert_eq!(revenue_impact.supporting_claims.len(), 2);

        let margin_impact = impacts
            .iter()
            .find(|i| i.assumption == "gross_margin")
            .expect("should have gross_margin impact");
        assert!(margin_impact.estimated_adjustment < 0.0);
        assert_eq!(margin_impact.supporting_claims.len(), 1);
    }

    #[test]
    fn extract_assumption_impacts_empty() {
        let claims: Vec<ResearchClaim> = vec![];
        let impacts = extract_assumption_impacts(&claims);
        assert!(impacts.is_empty());
    }

    #[test]
    fn extract_guidance_finds_statements() {
        let claims = vec![
            ResearchClaim {
                text: "Management raised revenue guidance for the full year, now expects $10-11B"
                    .to_string(),
                source: "https://a.com/1".to_string(),
                source_title: "Article 1".to_string(),
                date: None,
                direction: "positive".to_string(),
                affected_assumption: Some("revenue_growth".to_string()),
                magnitude: None,
                provider: "exa".to_string(),
            },
            ResearchClaim {
                text: "Analyst forecasts project 15% EPS growth next year".to_string(),
                source: "https://b.com/2".to_string(),
                source_title: "Article 2".to_string(),
                date: None,
                direction: "positive".to_string(),
                affected_assumption: None,
                magnitude: None,
                provider: "tavily".to_string(),
            },
            ResearchClaim {
                text: "Company opened a new office".to_string(),
                source: "https://c.com/3".to_string(),
                source_title: "Article 3".to_string(),
                date: None,
                direction: "neutral".to_string(),
                affected_assumption: None,
                magnitude: None,
                provider: "brave".to_string(),
            },
        ];

        let guidance = extract_guidance(&claims);
        assert_eq!(guidance.len(), 2);

        let first = &guidance[0];
        assert!(first.text.contains("raised revenue guidance"));
        assert_eq!(first.metric, "revenue");
        assert_eq!(first.guidance_type, "management_outlook");

        let second = &guidance[1];
        assert!(second.text.contains("EPS growth"));
        assert_eq!(second.metric, "eps");
        assert_eq!(second.guidance_type, "analyst_estimate");
    }

    #[test]
    fn extract_guidance_empty() {
        let claims = vec![ResearchClaim {
            text: "The CEO visited the factory floor today".to_string(),
            source: "https://x.com/1".to_string(),
            source_title: "Article".to_string(),
            date: None,
            direction: "neutral".to_string(),
            affected_assumption: None,
            magnitude: None,
            provider: "exa".to_string(),
        }];

        let guidance = extract_guidance(&claims);
        assert!(guidance.is_empty());
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

    #[test]
    fn parse_exa_empty_results() {
        let json = serde_json::json!({"results": []});
        let claims = parse_exa_results(&json, "test").expect("should parse empty results");
        assert!(claims.is_empty());
    }

    #[test]
    fn parse_tavily_empty_results() {
        let json = serde_json::json!({"results": []});
        let claims = parse_tavily_results(&json).expect("should parse empty results");
        assert!(claims.is_empty());
    }

    #[test]
    fn parse_brave_empty_results() {
        let json = serde_json::json!({"web": {"results": []}});
        let claims = parse_brave_results(&json).expect("should parse empty results");
        assert!(claims.is_empty());
    }
}
