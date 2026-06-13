//! Integration test: calls MDS §3 spec tools through the MCP protocol
//! to verify end-to-end behavior.
//!
//! Tests exercise the full MCP path: JSON-RPC framing via rmcp,
//! credential resolution, #[tool] macro dispatch, handler invocation,
//! and response serialization.
//!
//! Uses rmcp's `TokioChildProcess` transport with the same lifecycle
//! pattern as production code (`McpRuntime::start_server`): the
//! `RunningService` is kept alive in a background task so that its
//! `DropGuard` doesn't cancel the serve loop.
//!
//! Run with: cargo test -p hkask-mcp-spec --test mcp_protocol

use hkask_types::{DelegationAction, DelegationResource, DelegationToken, WebID};
use rmcp::model::CallToolRequestParams;
use rmcp::service::{RoleClient, ServiceExt};
use rmcp::transport::TokioChildProcess;
use std::str::FromStr;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

/// All-zeros hex secret used in integration tests (matches HKASK_OCAP_SECRET env).
const TEST_OCAP_SECRET_HEX: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Known WebID used for both the server identity and the token delegate.
const TEST_WEBID_STR: &str = "00000000-0000-0000-0000-000000000001";

/// Construct a valid `DelegationToken` for integration tests.
///
/// Uses the all-zeros secret and a known WebID so the token passes
/// the server's `verify_capability` check.
fn make_capability_token(resource_id: &str, action: DelegationAction) -> String {
    let secret = hex::decode(TEST_OCAP_SECRET_HEX).expect("valid hex");
    let webid = WebID::from_str(TEST_WEBID_STR).expect("valid UUID");
    let token = DelegationToken::new(
        DelegationResource::Registry,
        resource_id.to_string(),
        action,
        webid, // delegated_from = same as to (root delegation)
        webid, // delegated_to = server's WebID
        &secret,
    );
    token.to_base64().expect("base64 encode")
}

/// Spawn the spec server as a child process via rmcp and return a connected
/// peer for making MCP calls.
async fn spawn_server() -> rmcp::service::Peer<RoleClient> {
    let binary = std::env::var("CARGO_BIN_EXE_hkask_mcp_spec").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "..".to_string());
        format!("{}/../../target/debug/hkask-mcp-spec", manifest_dir)
    });

    let mut cmd = Command::new(&binary);
    cmd.env("HKASK_OCAP_SECRET", TEST_OCAP_SECRET_HEX)
        .env("HKASK_WEBID", TEST_WEBID_STR)
        .env("RUST_LOG", "off");

    let transport = TokioChildProcess::new(cmd).expect("Failed to create transport");

    let client_info = rmcp::model::InitializeRequestParams::new(
        rmcp::model::ClientCapabilities::default(),
        rmcp::model::Implementation::new("spec-integration-test", "0.1.0"),
    );

    let running = client_info
        .into_dyn()
        .serve(transport)
        .await
        .expect("Failed to connect to spec server");

    let peer = running.peer().clone();

    let cancel = CancellationToken::new();
    let _guard = cancel.drop_guard();
    tokio::spawn(async move {
        let _ = running.waiting().await;
    });
    tokio::task::yield_now().await;

    peer
}

/// Helper to extract text content from a `CallToolResult`.
fn text_from_result(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| {
            if let rmcp::model::RawContent::Text(tc) = &**c {
                Some(tc.text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Tool listing tests (MDS §3: 5 tools) ──────────────────────

/// REQ: MDS-SRV-001 — all 5 MDS §3 tools are listed
#[tokio::test]
async fn all_mds_tools_are_listed() {
    let peer = spawn_server().await;

    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();

    let expected = &[
        "spec_goal_capture",
        "spec_goal_decompose",
        "spec_require_writing_quality",
        "spec_graph_query",
        "spec_graph_coherence",
        "spec_replica_rewrite",
    ];

    for name in expected {
        assert!(
            tool_names.contains(&name.to_string()),
            "{name} must be in tool list, got: {tool_names:?}"
        );
    }

    // Verify no stale tools from DDMVSS remain
    let banned = &[
        "spec_test_invariant",
        "spec_test_verify",
        "spec_require_bind",
        "spec_curate_evaluate",
        "spec_curate_reconcile",
        "spec_curate_cultivate",
        "spec_curate_writing_excellence",
        "spec_graph_validate",
    ];
    for name in banned {
        assert!(
            !tool_names.contains(&name.to_string()),
            "{name} must NOT be in tool list (deleted per MDS §3), got: {tool_names:?}"
        );
    }
}

// ── Capability enforcement tests ───────────────────────────────

/// REQ: MDS-CAP-001 — spec_goal_capture rejects missing capability token
#[tokio::test]
async fn capture_rejects_missing_capability_token() {
    let peer = spawn_server().await;

    let params = CallToolRequestParams::new("spec_goal_capture").with_arguments(
        serde_json::from_str(r#"{"description": "Test goal", "context": "domain context"}"#)
            .expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("permission_denied") || text.contains("No capability token"),
        "Missing token must produce permission error, got: {text}"
    );
}

/// REQ: MDS-CAP-002 — spec_graph_query rejects missing capability token
#[tokio::test]
async fn graph_query_rejects_missing_capability_token() {
    let peer = spawn_server().await;

    let params = CallToolRequestParams::new("spec_graph_query").with_arguments(
        serde_json::from_str(r#"{"query": "test"}"#).expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("permission_denied") || text.contains("No capability token"),
        "Missing token must produce permission error, got: {text}"
    );
}

// ── Happy-path tests ───────────────────────────────────────────

/// REQ: MDS-CAP-003 — spec_goal_capture records a spec with valid token
#[tokio::test]
async fn capture_records_spec_with_token() {
    let peer = spawn_server().await;

    let token = make_capability_token("capture", DelegationAction::Write);
    let params = CallToolRequestParams::new("spec_goal_capture").with_arguments(
        serde_json::from_str(&format!(
            r#"{{"description": "Test capture spec", "context": "trust security boundary", "capability_token": "{token}"}}"#
        ))
        .expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("goal_id"),
        "Capture must return goal_id, got: {text}"
    );
    assert!(
        text.contains("requirements"),
        "Capture must return requirements, got: {text}"
    );
}

/// REQ: MDS-CAP-004 — spec_graph_coherence returns coherence score with valid token
#[tokio::test]
async fn coherence_returns_score_with_token() {
    let peer = spawn_server().await;

    let token = make_capability_token("coherence", DelegationAction::Read);
    let params = CallToolRequestParams::new("spec_graph_coherence").with_arguments(
        serde_json::from_str(&format!(r#"{{"capability_token": "{token}"}}"#))
            .expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("coherence_score"),
        "Coherence must return coherence_score, got: {text}"
    );
}

/// REQ: MDS-CAP-005 — spec_require_writing_quality returns quality assessment
#[tokio::test]
async fn writing_quality_assesses_spec_with_token() {
    let peer = spawn_server().await;

    // First capture a spec so we have something to assess
    let token = make_capability_token("capture", DelegationAction::Write);
    let capture_params = CallToolRequestParams::new("spec_goal_capture").with_arguments(
        serde_json::from_str(&format!(
            r#"{{"description": "A well-defined goal with clear acceptance criteria.", "context": "composition interface api", "capability_token": "{token}"}}"#
        ))
        .expect("valid JSON arguments"),
    );
    let result = peer
        .call_tool(capture_params)
        .await
        .expect("Tool call failed");
    let text = text_from_result(&result);

    // Extract the goal_id from capture response (wrapped in {"content": {...}} envelope)
    let goal_id: String = {
        let v: serde_json::Value =
            serde_json::from_str(&text).expect("capture response must be valid JSON");
        v["content"]["goal_id"].as_str().unwrap_or("").to_string()
    };
    assert!(!goal_id.is_empty(), "Capture must return a valid goal_id");

    // Now assess writing quality
    let read_token = make_capability_token(&goal_id, DelegationAction::Read);
    let q_params = CallToolRequestParams::new("spec_require_writing_quality").with_arguments(
        serde_json::from_str(&format!(
            r#"{{"spec_id": "{goal_id}", "capability_token": "{read_token}"}}"#
        ))
        .expect("valid JSON arguments"),
    );

    let q_result = peer.call_tool(q_params).await.expect("Tool call failed");
    let q_text = text_from_result(&q_result);
    assert!(
        q_text.contains("dimensions_passing"),
        "Writing quality must return dimensions_passing, got: {q_text}"
    );
}
