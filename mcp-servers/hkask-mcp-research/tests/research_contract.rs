//! Contract tests for hkask-mcp-research — HTML stripping, cache operations, and request types.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `strip_html`, `ResponseCache`, and request type deserialization (no external API calls).

use hkask_mcp_research::ResearchServer;
use hkask_services_research::{ProviderPool, RateLimiter, ResponseCache, build_provider_pool};
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// ── HTML stripping tests ───────────────────────────────────────────────────

#[test]
fn strip_html_removes_tags() {
    let result = hkask_mcp_research::strip_html::strip_html("<p>Hello <b>World</b></p>");
    assert!(result.contains("Hello"));
    assert!(result.contains("World"));
    assert!(!result.contains("<p>"));
    assert!(!result.contains("<b>"));
}

#[test]
fn strip_html_preserves_plain_text() {
    let input = "Just plain text, no HTML at all.";
    let result = hkask_mcp_research::strip_html::strip_html(input);
    assert_eq!(result.trim(), input.trim());
}

#[test]
fn strip_html_handles_empty_input() {
    let result = hkask_mcp_research::strip_html::strip_html("");
    assert!(result.is_empty());
}

#[test]
fn strip_html_converts_br_to_newline() {
    let result = hkask_mcp_research::strip_html::strip_html("line1<br>line2");
    assert!(result.contains('\n'), "should convert <br> to newline");
}

#[test]
fn strip_html_removes_script_content() {
    let result = hkask_mcp_research::strip_html::strip_html(
        "<p>Visible</p><script>alert('xss')</script><p>Also visible</p>",
    );
    assert!(result.contains("Visible"));
    assert!(result.contains("Also visible"));
    assert!(!result.contains("alert"));
    assert!(!result.contains("xss"));
}

#[test]
fn strip_html_removes_style_content() {
    let result = hkask_mcp_research::strip_html::strip_html(
        "<style>body { color: red; }</style><p>Text</p>",
    );
    assert!(result.contains("Text"));
    assert!(!result.contains("color: red"));
}

#[test]
fn strip_html_handles_nested_tags() {
    let result = hkask_mcp_research::strip_html::strip_html(
        "<div><p><span>Nested <em>content</em></span></p></div>",
    );
    assert!(result.contains("Nested"));
    assert!(result.contains("content"));
}

// ── Cache tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn cache_insert_and_retrieve() {
    let cache = hkask_mcp_research::cache::ResponseCache::new(10, Duration::from_secs(60));
    let key = hkask_mcp_research::cache::CacheKey("test_key".into());
    let value = serde_json::json!({"data": "test_value"});

    cache.insert(key.clone(), value.clone()).await;
    let retrieved = cache.get(&key).await;
    assert_eq!(retrieved, Some(value));
}

#[tokio::test]
async fn cache_returns_none_for_missing_key() {
    let cache = hkask_mcp_research::cache::ResponseCache::new(10, Duration::from_secs(60));
    let key = hkask_mcp_research::cache::CacheKey("missing".into());
    let retrieved = cache.get(&key).await;
    assert_eq!(retrieved, None);
}

#[tokio::test]
async fn cache_expires_entries() {
    let cache = hkask_mcp_research::cache::ResponseCache::new(10, Duration::from_millis(1));
    let key = hkask_mcp_research::cache::CacheKey("ephemeral".into());
    let value = serde_json::json!({"data": "short_lived"});

    cache.insert(key.clone(), value).await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    let retrieved = cache.get(&key).await;
    assert_eq!(retrieved, None, "entry should expire after TTL");
}

// ── Cache key tests ────────────────────────────────────────────────────────

#[test]
fn cache_key_equality() {
    let k1 = hkask_mcp_research::cache::CacheKey("key1".into());
    let k2 = hkask_mcp_research::cache::CacheKey("key1".into());
    let k3 = hkask_mcp_research::cache::CacheKey("key2".into());
    assert!(k1 == k2, "same key should be equal");
    assert!(k1 != k3, "different keys should not be equal");
}

// ── Request type tests ─────────────────────────────────────────────────────

#[test]
fn search_request_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_research::types::SearchRequest>();
    assert!(_type_name.contains("hkask_services_research"));
}

// ── RSS types tests ────────────────────────────────────────────────────────

#[test]
fn subscribe_request_parses_valid_json() {
    let json = serde_json::json!({
        "url": "https://example.com/feed.xml",
        "label": "Example Feed"
    });
    let req: hkask_mcp_research::rss_types::SubscribeRequest =
        serde_json::from_value(json).expect("should parse subscribe request");
    assert_eq!(req.url, "https://example.com/feed.xml");
    assert_eq!(req.label, Some("Example Feed".to_string()));
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// Construct a ResearchServer with an empty provider pool (no API keys).
/// Search tools will return errors, but ping and structural tools work.
fn test_server() -> ResearchServer {
    let pool = build_provider_pool(&HashMap::new()).expect("empty provider pool");
    ResearchServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        Arc::new(pool),
        Arc::new(ResponseCache::new(10, Duration::from_secs(60))),
        RateLimiter::new(30, 60),
        None,
        reqwest::Client::new(),
    )
}

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

/// Extract the `kind` field from an error envelope, if present.
fn error_kind(out: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("kind").and_then(|e| e.as_str()).map(String::from)
}

// REQ: web_ping returns liveness and provider info (P5 Testing Discipline).
// expect: web_ping returns status=ok and version info.
#[tokio::test]
async fn web_ping_returns_status_ok_via_parameters_seam() {
    let server = test_server();
    let out = server.web_ping().await;
    let content = parse_content(&out);
    assert_eq!(content["status"], "ok", "got: {out}");
    assert!(
        content.get("version").is_some(),
        "should have version: {out}"
    );
}

// REQ: web_search rejects an empty query with invalid_argument (P5).
// expect: an empty query string returns kind=invalid_argument.
#[tokio::test]
async fn web_search_rejects_empty_query_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_research::types::SearchRequest = serde_json::from_value(serde_json::json!({
        "query": "",
        "strategy": "quick"
    }))
    .expect("deserialize SearchRequest");
    let out = server.web_search(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for empty query");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: rss_list_subscriptions rejects when no RSS DB is configured (P5).
// expect: without an RSS database, returns kind=unavailable.
#[tokio::test]
async fn rss_list_subscriptions_rejects_without_db_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_research::rss_types::ListSubscriptionsRequest =
        serde_json::from_value(serde_json::json!({"folder": null}))
            .expect("deserialize ListSubscriptionsRequest");
    let out = server.rss_list_subscriptions(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for missing RSS db");
    assert_eq!(kind, "unavailable", "got: {out}");
}

// REQ: rss_export_opml rejects when no RSS DB is configured (P5).
// expect: without an RSS database, returns kind=unavailable.
#[tokio::test]
async fn rss_export_opml_rejects_without_db_via_parameters_seam() {
    let server = test_server();
    let out = server.rss_export_opml().await;
    let kind = error_kind(&out).expect("expected error kind for missing RSS db");
    assert_eq!(kind, "unavailable", "got: {out}");
}
