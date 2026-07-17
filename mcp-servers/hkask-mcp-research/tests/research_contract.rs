//! Contract tests for hkask-mcp-research — HTML stripping, cache operations, and request types.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `strip_html`, `ResponseCache`, and request type deserialization (no external API calls).

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
