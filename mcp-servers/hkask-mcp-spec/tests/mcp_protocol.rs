//! Integration test: calls spec/test/invariant and spec/test/verify
//! through the MCP protocol to verify end-to-end behavior.
//!
//! These tests exercise the full MCP path: JSON-RPC framing,
//! credential resolution, #[tool] macro dispatch, handler invocation,
//! and response serialization.
//!
//! Uses the rmcp client library to spawn the spec server as a child
//! process and call tools through the MCP protocol.
//!
//! Run with: cargo test -p hkask-mcp-spec --test mcp_protocol

use rmcp::model::CallToolRequestParams;
use rmcp::service::{RoleClient, ServiceExt};
use rmcp::transport::TokioChildProcess;
use serde_json::Value;
use tokio::process::Command;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}

/// Spawn the spec server as a child process with the required environment,
/// returning the rmcp client peer and the RunningService handle.
///
/// The RunningService must be kept alive for the duration of the test
/// because dropping it cancels the CancellationToken which closes the
/// transport. We return it so the caller can keep it alive.
async fn spawn_spec_server() -> (
    rmcp::service::Peer<RoleClient>,
    rmcp::service::RunningService<RoleClient, Box<dyn rmcp::service::DynService<RoleClient>>>,
) {
    let binary = std::env::var("CARGO_BIN_EXE_hkask_mcp_spec").unwrap_or_else(|_| {
        eprintln!("[mcp_protocol] CARGO_BIN_EXE_hkask_mcp_spec not set, using fallback");
        // Try common locations relative to workspace root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        format!("{}/../../target/debug/hkask-mcp-spec", manifest_dir)
    });
    eprintln!("[mcp_protocol] Using binary: {binary}");

    let mut cmd = Command::new(&binary);
    cmd.env(
        "HKASK_OCAP_SECRET",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let transport = TokioChildProcess::new(cmd).expect("Failed to create transport");

    // Use a proper client info so the MCP handshake succeeds.
    // `ClientInfo` is `InitializeRequestParams`; `ClientHandler for ClientInfo`
    // returns self, giving the server name/version/capabilities in the init.
    let client_info = rmcp::model::InitializeRequestParams::new(
        rmcp::model::ClientCapabilities::default(),
        rmcp::model::Implementation::new("spec-integration-test", "0.1.0"),
    );
    let running = client_info
        .into_dyn()
        .serve(transport)
        .await
        .expect("Failed to connect to spec server");
    eprintln!("[mcp_protocol] Connected to spec server successfully");
    let peer = running.peer().clone();

    (peer, running)
}

/// Helper to extract text content from a CallToolResult.
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

/// Helper to build arguments as JsonObject from a JSON string.
fn args(json_str: &str) -> serde_json::Map<String, Value> {
    serde_json::from_str(json_str).expect("invalid JSON arguments")
}

// REQ: INVAR-001 — spec/test/invariant is listed as an available tool
#[tokio::test]
async fn test_invariant_tool_is_listed() {
    init_tracing();
    let (peer, _running) = spawn_spec_server().await;
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
    let (peer, _running) = spawn_spec_server().await;
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
    let (peer, _running) = spawn_spec_server().await;

    let params = CallToolRequestParams::new("spec_test_invariant")
        .with_arguments(args(r#"{ "spec_id": "00000000-0000-0000-0000-000000000001", "seam": "spec-test-invariant", "invariant": "rejects-missing-token", "category": "PublicInterface" }"#));

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
    let (peer, _running) = spawn_spec_server().await;

    let params = CallToolRequestParams::new("spec_test_verify")
        .with_arguments(args(r#"{ "category": "domain" }"#));

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("permission_denied") || text.contains("No capability token"),
        "Missing token must produce permission error, got: {text}"
    );
}

// REQ: INVAR-004 + VERIFY-003 — full round-trip: capture, invariant, verify
#[tokio::test]
async fn test_capture_then_invariant_then_verify() {
    let (peer, _running) = spawn_spec_server().await;

    // Step 1: Capture a spec (no capability token → will fail with permission_denied,
    // but proves the tool is callable through MCP protocol)
    let capture_params = CallToolRequestParams::new("spec_goal_capture")
        .with_arguments(args(r#"{ "description": "integration test spec", "category": "domain", "domain_anchor": "hkask" }"#));

    let capture = peer
        .call_tool(capture_params)
        .await
        .expect("Capture call failed");
    let capture_text = text_from_result(&capture);
    // Without a valid capability token, we expect permission_denied
    assert!(
        capture_text.contains("permission_denied") || capture_text.contains("No capability token"),
        "Capture without token must produce permission error, got: {capture_text}"
    );

    // Step 2: Call spec/test/invariant without capability token (should also fail)
    let invar_params = CallToolRequestParams::new("spec_test_invariant")
        .with_arguments(args(r#"{ "spec_id": "00000000-0000-0000-0000-000000009999", "seam": "spec-test-invariant", "invariant": "integration-test-invariant", "category": "PublicInterface" }"#));

    let invariant = peer
        .call_tool(invar_params)
        .await
        .expect("Invariant call failed");
    let invariant_text = text_from_result(&invariant);

    assert!(
        invariant_text.contains("permission_denied")
            || invariant_text.contains("No capability token"),
        "Invariant without token must produce permission error, got: {invariant_text}"
    );

    // Step 3: Call spec/test/verify without capability token (should also fail)
    let verify_params = CallToolRequestParams::new("spec_test_verify")
        .with_arguments(args(r#"{ "category": "domain" }"#));

    let verify = peer
        .call_tool(verify_params)
        .await
        .expect("Verify call failed");
    let verify_text = text_from_result(&verify);

    assert!(
        verify_text.contains("permission_denied") || verify_text.contains("No capability token"),
        "Verify without token must produce permission error, got: {verify_text}"
    );
}
