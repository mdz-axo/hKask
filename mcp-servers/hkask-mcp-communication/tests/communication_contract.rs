//! Contract tests for hkask-mcp-communication — agent registry invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `AgentRegistry` (in-memory HashMap, no external dependencies).

use hkask_communication::matrix::MatrixTransport;
use hkask_communication::matrix::UserId;
use hkask_mcp_communication::CommunicationServer;
use hkask_mcp_communication::agent_registration::AgentRegistry;
use hkask_mcp_communication::types::ListVoicesRequest;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

// ── Registration contract tests ─────────────────────────────────────────────

// [P1] Goal: User Sovereignty — agents communicate through user-owned channels
// [P12] Constraining: both WebID and UserId carry authenticated identity
#[tokio::test]
async fn record_mapping_stores_webid_to_userid() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let user_id = UserId::new("@alice:localhost");

    registry.record_mapping(&webid, &user_id).await;

    let found = registry.resolve(&webid).await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().as_str(), "@alice:localhost");
}

#[tokio::test]
async fn record_mapping_is_idempotent() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let first = UserId::new("@alice:localhost");
    let second = UserId::new("@alice-v2:localhost");

    registry.record_mapping(&webid, &first).await;
    registry.record_mapping(&webid, &second).await;

    let found = registry.resolve(&webid).await;
    assert_eq!(found.unwrap().as_str(), "@alice-v2:localhost");
}

#[tokio::test]
async fn deregister_removes_mapping() {
    let registry = AgentRegistry::new();
    let webid = hkask_types::WebID::new();
    let user_id = UserId::new("@bob:localhost");

    registry.record_mapping(&webid, &user_id).await;
    registry
        .deregister(&webid)
        .await
        .expect("deregister should succeed");

    let found = registry.resolve(&webid).await;
    assert!(found.is_none());
}

#[tokio::test]
async fn deregister_nonexistent_is_ok() {
    let registry = AgentRegistry::new();
    let unknown = hkask_types::WebID::new();

    let result = registry.deregister(&unknown).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn lookup_unregistered_returns_none() {
    let registry = AgentRegistry::new();
    let unknown = hkask_types::WebID::new();

    let found = registry.resolve(&unknown).await;
    assert!(found.is_none());
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// Construct a CommunicationServer with a non-connected MatrixTransport.
/// The TTS tools and voice listing work without a Matrix connection.
fn test_server() -> CommunicationServer {
    let matrix = Arc::new(MatrixTransport::new("http://localhost:0"));
    let registry = Arc::new(AgentRegistry::new());
    CommunicationServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        matrix,
        registry,
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

// REQ: tts_list_voices returns the voice list (P5 Testing Discipline).
// expect: returns a non-empty voices array with engine=espeak.
#[tokio::test]
async fn tts_list_voices_returns_voices_via_parameters_seam() {
    let server = test_server();
    let out = server
        .tts_list_voices(Parameters(ListVoicesRequest { language: None }))
        .await;
    let content = parse_content(&out);
    assert!(
        content["voices"].is_array(),
        "voices should be an array: {out}"
    );
    assert!(
        content["total"].as_u64().unwrap_or(0) > 0,
        "should have voices: {out}"
    );
    assert_eq!(content["engine"], "espeak");
}

// REQ: tts_list_voices filters by language prefix (P5).
// expect: filtering by "en" returns only English voices.
#[tokio::test]
async fn tts_list_voices_filters_by_language_via_parameters_seam() {
    let server = test_server();
    let out = server
        .tts_list_voices(Parameters(ListVoicesRequest {
            language: Some("en".into()),
        }))
        .await;
    let content = parse_content(&out);
    let voices = content["voices"].as_array().expect("voices array");
    assert!(!voices.is_empty(), "should have English voices: {out}");
    // Every returned voice should start with "en"
    for v in voices {
        let lang = v["language"].as_str().unwrap_or("");
        assert!(
            lang.starts_with("en"),
            "voice language should start with 'en': {lang}"
        );
    }
}

// REQ: tts_list_voices with a non-matching language returns empty (P5).
// expect: filtering by a non-existent language returns total=0.
#[tokio::test]
async fn tts_list_voices_empty_for_unknown_language_via_parameters_seam() {
    let server = test_server();
    let out = server
        .tts_list_voices(Parameters(ListVoicesRequest {
            language: Some("zz".into()),
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["total"], 0, "should have no voices for 'zz': {out}");
}
