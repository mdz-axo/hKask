//! Integration test: calls spec/test/invariant and spec/test/verify
//! through the MCP protocol to verify end-to-end behavior.
//!
//! These tests exercise the full MCP path: JSON-RPC framing,
//! credential resolution, #[tool] macro dispatch, handler invocation,
//! and response serialization.
//!
//! **NOTE:** These tests are currently ignored because the rmcp client
//! transport closes immediately after the MCP handshake. The root cause
//! is that the server process's CancellationToken is cancelled right after
//! initialization, causing "task cancelled" and the service loop to exit.
//! This needs investigation in the rmcp library's TokioChildProcess
//! transport implementation. The server works correctly when called via
//! `kask mcp invoke` or pipe-based JSON-RPC — the issue is specific to
//! the rmcp client's `serve()` → `RunningService` lifecycle.
//!
//! Run with: cargo test -p hkask-mcp-spec --test mcp_protocol -- --ignored

use rmcp::model::CallToolRequestParams;
use rmcp::service::{RoleClient, ServiceExt};
use rmcp::transport::TokioChildProcess;
use serde_json::Value;
use tokio::process::Command;

/// Spawn the spec server as a child process with the required environment,
/// returning the RunningService (which must be kept alive for the transport
/// to remain open).
async fn spawn_spec_server()
-> rmcp::service::RunningService<RoleClient, Box<dyn rmcp::service::DynService<RoleClient>>> {
    let binary = std::env::var("CARGO_BIN_EXE_hkask_mcp_spec").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        format!("{}/../../target/debug/hkask-mcp-spec", manifest_dir)
    });

    let mut cmd = Command::new(&binary);
    cmd.env(
        "HKASK_OCAP_SECRET",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let transport = TokioChildProcess::new(cmd).expect("Failed to create transport");

    let client_info = rmcp::model::InitializeRequestParams::new(
        rmcp::model::ClientCapabilities::default(),
        rmcp::model::Implementation::new("spec-integration-test", "0.1.0"),
    );
    client_info
        .into_dyn()
        .serve(transport)
        .await
        .expect("Failed to connect to spec server")
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
//
// TODO: Ignored due to rmcp client transport lifecycle issue.
// The server works correctly via pipe-based JSON-RPC, but the rmcp
// RunningService transport closes after initialization.
#[tokio::test]
#[ignore = "rmcp client transport closes after MCP handshake — needs investigation"]
async fn test_invariant_tool_is_listed() {
    let running = spawn_spec_server().await;
    let peer = running.peer();
    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();

    assert!(
        tool_names.contains(&"spec_test_invariant".to_string()),
        "spec_test_invariant must be in tool list, got: {tool_names:?}"
    );
}

// REQ: VERIFY-001 — spec/test/verify is listed as an available tool
#[tokio::test]
#[ignore = "rmcp client transport closes after MCP handshake — needs investigation"]
async fn test_verify_tool_is_listed() {
    let running = spawn_spec_server().await;
    let peer = running.peer();
    let tools = peer.list_all_tools().await.expect("Failed to list tools");
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();

    assert!(
        tool_names.contains(&"spec_test_verify".to_string()),
        "spec_test_verify must be in tool list, got: {tool_names:?}"
    );
}

// REQ: INVAR-002 — spec/test/invariant rejects requests without capability token
#[tokio::test]
#[ignore = "rmcp client transport closes after MCP handshake — needs investigation"]
async fn test_invariant_rejects_missing_capability_token() {
    let running = spawn_spec_server().await;
    let peer = running.peer();

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
#[ignore = "rmcp client transport closes after MCP handshake — needs investigation"]
async fn test_verify_rejects_missing_capability_token() {
    let running = spawn_spec_server().await;
    let peer = running.peer();

    let params = CallToolRequestParams::new("spec_test_verify")
        .with_arguments(args(r#"{ "category": "domain" }"#));

    let result = peer.call_tool(params).await.expect("Tool call failed");
    let text = text_from_result(&result);

    assert!(
        text.contains("permission_denied") || text.contains("No capability token"),
        "Missing token must produce permission error, got: {text}"
    );
}
