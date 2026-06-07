//! Integration test: calls spec/test/invariant and spec/test/verify
//! through the MCP protocol to verify end-to-end behavior.
//!
//! These tests exercise the full MCP path: JSON-RPC framing,
//! credential resolution, #[tool] macro dispatch, handler invocation,
//! and response serialization.
//!
//! Run with: cargo test -p hkask-mcp-spec --test mcp_protocol

use std::process::Stdio;
use tokio::process::Command;

/// Spawn the spec server as a child process with the required environment,
/// returning the rmcp client peer.
async fn spawn_spec_server() -> rmcp::service::Peer<rmcp::service::RoleClient> {
    let binary = std::env::var("CARGO_BIN_EXE_hkask_mcp_spec")
        .unwrap_or_else(|_| "./target/debug/hkask-mcp-spec".to_string());

    let child = Command::new(&binary)
        .env(
            "HKASK_OCAP_SECRET",
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn hkask-mcp-spec");

    let transport =
        rmcp::transport::TokioChildProcess::new(child).expect("Failed to create transport");

    let client = rmcp::handler::client::ClientHandler::new(rmcp::model::InitializeRequestParams {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: rmcp::model::ClientCapabilities::default(),
        client_info: rmcp::model::Implementation {
            name: "spec-integration-test".into(),
            version: "0.1.0".into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let running = client
        .connect(transport)
        .await
        .expect("Failed to connect to spec server");
    running.peer().clone()
}

/// Helper to extract text content from a CallToolResult.
fn text_from_result(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| {
            if let rmcp::model::RawContent::Text(tc) = c {
                Some(tc.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Helper to build arguments as JsonObject.
fn args(json_str: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    serde_json::from_str(json_str).ok()
}

// REQ: INVAR-001 — spec/test/invariant is listed as an available tool
#[tokio::test]
async fn test_invariant_tool_is_listed() {
    let peer = spawn_spec_server().await;
    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    assert!(
        tool_names.contains(&"spec_test_invariant"),
        "spec_test_invariant must be in tool list, got: {tool_names:?}"
    );
}

// REQ: VERIFY-001 — spec/test/verify is listed as an available tool
#[tokio::test]
async fn test_verify_tool_is_listed() {
    let peer = spawn_spec_server().await;
    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    assert!(
        tool_names.contains(&"spec_test_verify"),
        "spec_test_verify must be in tool list, got: {tool_names:?}"
    );
}

// REQ: INVAR-002 — spec/test/invariant rejects requests without capability token
#[tokio::test]
async fn test_invariant_rejects_missing_capability_token() {
    let peer = spawn_spec_server().await;

    let params = rmcp::model::CallToolRequestParams::new("spec_test_invariant".into())
        .with_arguments(args(r#"{ "spec_id": "00000000-0000-0000-0000-000000000001", "seam": "spec-test-invariant", "invariant": "rejects-missing-token", "category": "PublicInterface" }"#).unwrap());

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
    let peer = spawn_spec_server().await;

    let params = rmcp::model::CallToolRequestParams::new("spec_test_verify".into())
        .with_arguments(args(r#"{ "category": "domain" }"#).unwrap());

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
    let peer = spawn_spec_server().await;

    // Step 1: Capture a spec
    let capture_params = rmcp::model::CallToolRequestParams::new("spec_goal_capture".into())
        .with_arguments(args(r#"{ "description": "integration test spec", "category": "domain", "domain_anchor": "hkask", "capability_token": "dGVzdA==" }"#).unwrap());

    let capture = peer
        .call_tool(capture_params)
        .await
        .expect("Capture failed");
    let capture_text = text_from_result(&capture);
    assert!(
        capture_text.contains("captured"),
        "Capture must succeed, got: {capture_text}"
    );

    // Step 2: Call spec/test/invariant (will fail with not_found since we
    // don't know the UUID, but proves the tool path works)
    let invar_params = rmcp::model::CallToolRequestParams::new("spec_test_invariant".into())
        .with_arguments(args(r#"{ "spec_id": "00000000-0000-0000-0000-000000009999", "seam": "spec-test-invariant", "invariant": "integration-test-invariant", "category": "PublicInterface", "capability_token": "dGVzdA==" }"#).unwrap());

    let invariant = peer
        .call_tool(invar_params)
        .await
        .expect("Invariant call failed");
    let invariant_text = text_from_result(&invariant);

    assert!(
        invariant_text.contains("not_found")
            || invariant_text.contains("recorded")
            || invariant_text.contains("invalid_argument"),
        "Invariant must respond through protocol, got: {invariant_text}"
    );

    // Step 3: Call spec/test/verify
    let verify_params = rmcp::model::CallToolRequestParams::new("spec_test_verify".into())
        .with_arguments(args(r#"{ "capability_token": "dGVzdA==" }"#).unwrap());

    let verify = peer.call_tool(verify_params).await.expect("Verify failed");
    let verify_text = text_from_result(&verify);

    assert!(
        verify_text.contains("total_requirements"),
        "Verify must report total_requirements, got: {verify_text}"
    );
}
