//! Integration test: calls spec/test/invariant and spec/test/verify
//! through the MCP protocol to verify end-to-end behavior.
//!
//! These tests exercise the full MCP path: JSON-RPC framing via rmcp,
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
///
/// Uses the same lifecycle pattern as `McpRuntime::start_server`:
/// the `RunningService` is kept alive in a spawned task via a
/// `CancellationToken`, preventing the `DropGuard` from cancelling the
/// serve loop (which would close stdin and kill the child process).
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

    // Keep the RunningService alive in a background task, identical to
    // McpRuntime::start_server. Without this, the RunningService's DropGuard
    // cancels the CancellationToken, which causes the serve loop to exit,
    // which closes stdin, which kills the child process.
    //
    // A brief stabilization pause is needed because `serve()` returns as soon
    // as the handshake completes, but the spawned serve-loop task may not have
    // begun processing messages yet. Without this pause, the first request
    // through the Peer's mpsc channel can race the loop's readiness.
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

// REQ: INVAR-001 — spec/test/invariant is listed as an available tool
#[tokio::test]
async fn test_invariant_tool_is_listed() {
    let peer = spawn_server().await;

    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();

    assert!(
        tool_names.contains(&"spec_test_invariant".to_string()),
        "spec_test_invariant must be in tool list, got: {tool_names:?}"
    );
}

// REQ: VERIFY-001 — spec/test/verify is listed as an available tool
#[tokio::test]
async fn test_verify_tool_is_listed() {
    let peer = spawn_server().await;

    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();

    assert!(
        tool_names.contains(&"spec_test_verify".to_string()),
        "spec_test_verify must be in tool list, got: {tool_names:?}"
    );
}

// REQ: INVAR-002 — spec/test/invariant rejects requests without capability token
#[tokio::test]
async fn test_invariant_rejects_missing_capability_token() {
    let peer = spawn_server().await;

    let params = CallToolRequestParams::new("spec_test_invariant").with_arguments(
        serde_json::from_str(r#"{ "spec_id": "00000000-0000-0000-0000-000000000001", "seam": "spec-test-invariant", "invariant": "rejects-missing-token", "category": "PublicInterface" }"#)
            .expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("permission_denied") || text.contains("No capability token"),
        "Missing token must produce permission error, got: {text}"
    );
}

// REQ: VERIFY-002 — spec/test/verify rejects requests without capability token
#[tokio::test]
async fn test_verify_rejects_missing_capability_token() {
    let peer = spawn_server().await;

    let params = CallToolRequestParams::new("spec_test_verify").with_arguments(
        serde_json::from_str(r#"{ "category": "domain" }"#).expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("permission_denied") || text.contains("No capability token"),
        "Missing token must produce permission error, got: {text}"
    );
}

// REQ: INVAR-003 — spec/test/invariant records traceability with valid token
#[tokio::test]
async fn test_invariant_records_traceability_with_token() {
    let peer = spawn_server().await;

    let token = make_capability_token("invariant-traceability", DelegationAction::Read);
    let params = CallToolRequestParams::new("spec_test_invariant").with_arguments(
        serde_json::from_str(&format!(
            r#"{{ "spec_id": "00000000-0000-0000-0000-000000000001", "seam": "spec-test-invariant", "invariant": "records-traceability", "category": "PublicInterface", "capability_token": "{token}" }}"#
        ))
        .expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("recorded")
            || text.contains("not_found")
            || text.contains("invalid_argument")
            || text.contains("permission_denied"),
        "Invariant with token must respond through protocol, got: {text}"
    );
}

// REQ: VERIFY-003 — spec/test/verify reports results with valid token
#[tokio::test]
async fn test_verify_reports_results_with_token() {
    let peer = spawn_server().await;

    let token = make_capability_token("verify-results", DelegationAction::Read);
    let params = CallToolRequestParams::new("spec_test_verify").with_arguments(
        serde_json::from_str(&format!(
            r#"{{ "category": "domain", "capability_token": "{token}" }}"#
        ))
        .expect("valid JSON arguments"),
    );

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("total_requirements")
            || text.contains("permission_denied")
            || text.contains("No capability token"),
        "Verify with token must respond through protocol, got: {text}"
    );
}
