//! Integration test: calls spec/test/invariant and spec/test/verify
//! through the MCP protocol to verify end-to-end behavior.
//!
//! These tests exercise the full MCP path: JSON-RPC framing,
//! credential resolution, #[tool] macro dispatch, handler invocation,
//! and response serialization.
//!
//! Uses raw JSON-RPC over pipes instead of the rmcp client library,
//! because the rmcp TokioChildProcess transport has a lifecycle bug
//! that closes immediately after the MCP handshake.
//!
//! Run with: cargo test -p hkask-mcp-spec --test mcp_protocol

use hkask_types::{DelegationAction, DelegationResource, DelegationToken, WebID};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicU16, Ordering};

/// Global counter for JSON-RPC request IDs.
static NEXT_ID: AtomicU16 = AtomicU16::new(1);

/// Next available request ID.
fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) as u64
}

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

/// Spawn the spec server as a child process with piped stdin/stdout.
///
/// Sets `HKASK_OCAP_SECRET` to all-zeros hex so the capability token
/// derivation is deterministic. Also sets `RUST_LOG=off` to prevent
/// any log output from interfering with the JSON-RPC stream on stderr
/// (belt-and-suspenders after the tracing-to-stderr fix).
///
/// Sets `HKASK_WEBID` to a known UUID so that capability tokens
/// constructed with the matching WebID will be accepted by the server.
fn spawn_server() -> Child {
    let binary = std::env::var("CARGO_BIN_EXE_hkask_mcp_spec").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "..".to_string());
        format!("{}/../../target/debug/hkask-mcp-spec", manifest_dir)
    });

    Command::new(&binary)
        .env("HKASK_OCAP_SECRET", TEST_OCAP_SECRET_HEX)
        .env("HKASK_WEBID", TEST_WEBID_STR)
        .env("RUST_LOG", "off")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn hkask-mcp-spec")
}

/// MCP client that communicates with a child process via newline-delimited
/// JSON-RPC over stdin/stdout pipes.
struct McpClient {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl McpClient {
    /// Spawn the server and create a new MCP client.
    fn new() -> Self {
        let mut child = spawn_server();
        let stdout = child.stdout.take().expect("stdout pipe");
        let reader = BufReader::new(stdout);
        Self { child, reader }
    }

    /// Send a JSON-RPC request and read one response line.
    fn send_request(&mut self, method: &str, params: Value) -> Value {
        let id = next_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let line = format!("{}\n", serde_json::to_string(&request).expect("serialize"));
        let stdin = self.child.stdin.as_mut().expect("stdin pipe");
        stdin.write_all(line.as_bytes()).expect("write to stdin");
        stdin.flush().expect("flush stdin");

        self.read_response()
    }

    /// Send a JSON-RPC notification (no id, no response expected).
    fn send_notification(&mut self, method: &str, params: Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let line = format!(
            "{}\n",
            serde_json::to_string(&notification).expect("serialize")
        );
        let stdin = self.child.stdin.as_mut().expect("stdin pipe");
        stdin
            .write_all(line.as_bytes())
            .expect("write notification");
        stdin.flush().expect("flush");
    }

    /// Read one JSON-RPC response line from stdout.
    fn read_response(&mut self) -> Value {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf) {
            Ok(0) => {
                // Server closed stdout — check stderr for diagnostics
                let mut stderr_buf = String::new();
                if let Some(mut stderr) = self.child.stderr.take() {
                    let _ = std::io::Read::read_to_string(&mut stderr, &mut stderr_buf);
                }
                panic!("Server closed stdout before sending a response.\nStderr:\n{stderr_buf}");
            }
            Ok(_) => {}
            Err(e) => panic!("Failed to read from server stdout: {e}"),
        }

        serde_json::from_str(buf.trim()).unwrap_or_else(|e| {
            panic!("Failed to parse JSON-RPC response: {e}\nRaw: {buf}");
        })
    }

    /// Perform the MCP initialization handshake.
    ///
    /// Sends an `initialize` request and then a `notifications/initialized`
    /// notification. Returns the server's initialize result.
    fn initialize(&mut self) -> Value {
        let response = self.send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "spec-integration-test",
                    "version": "0.1.0"
                }
            }),
        );

        // Send initialized notification (no response expected per MCP spec)
        self.send_notification("notifications/initialized", json!({}));

        response
    }

    /// Shut down the server process.
    fn shutdown(&mut self) {
        // Close stdin to signal the server to shut down
        let _ = self.child.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Call a tool by name with the given arguments and return the raw JSON-RPC response.
    fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Value {
        self.send_request(
            "tools/call",
            json!({
                "name": tool_name,
                "arguments": arguments,
            }),
        )
    }

    /// Extract the inner JSON payload from an MCP tool response.
    ///
    /// The rmcp tool handler wraps its output in `McpToolOutput { content, metadata }`,
    /// so the text content of the response is `{"content": {...}}`. This helper
    /// parses the text content and returns the `"content"` field.
    fn extract_content(&mut self, tool_name: &str, arguments: Value) -> Value {
        let response = self.call_tool(tool_name, arguments);
        let text = response["result"]["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("");

        let parsed: Value = serde_json::from_str(text).unwrap_or_else(|e| {
            panic!("Failed to parse {tool_name} response as JSON: {e}\nRaw: {text}")
        });

        // Unwrap the McpToolOutput wrapper: {"content": {...}}
        if parsed.is_object() && parsed.get("content").is_some() {
            parsed["content"].clone()
        } else {
            parsed
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// REQ: INVAR-001 — spec/test/invariant is listed as an available tool
#[test]
fn test_invariant_tool_is_listed() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let response = client.send_request("tools/list", json!({}));

    let tool_names: Vec<String> = response["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|t| {
            t["name"]
                .as_str()
                .expect("tool name should be a string")
                .to_string()
        })
        .collect();

    assert!(
        tool_names.contains(&"spec_test_invariant".to_string()),
        "spec_test_invariant must be in tool list, got: {tool_names:?}"
    );
}

// REQ: VERIFY-001 — spec/test/verify is listed as an available tool
#[test]
fn test_verify_tool_is_listed() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let response = client.send_request("tools/list", json!({}));

    let tool_names: Vec<String> = response["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|t| {
            t["name"]
                .as_str()
                .expect("tool name should be a string")
                .to_string()
        })
        .collect();

    assert!(
        tool_names.contains(&"spec_test_verify".to_string()),
        "spec_test_verify must be in tool list, got: {tool_names:?}"
    );
}

// REQ: INVAR-002 — spec/test/invariant rejects requests without capability token
#[test]
fn test_invariant_rejects_missing_capability_token() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let response = client.call_tool(
        "spec_test_invariant",
        json!({
            "spec_id": "00000000-0000-0000-0000-000000000001",
            "seam": "spec-test-invariant",
            "invariant": "rejects-missing-token",
            "category": "PublicInterface"
        }),
    );

    // The response content should contain a permission_denied error
    let content_text = response["result"]["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("");

    assert!(
        content_text.contains("permission_denied") || content_text.contains("No capability token"),
        "Missing token must produce permission error, got: {content_text}"
    );
}

// REQ: VERIFY-002 — spec/test/verify rejects requests without capability token
#[test]
fn test_verify_rejects_missing_capability_token() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let response = client.call_tool(
        "spec_test_verify",
        json!({
            "category": "domain"
        }),
    );

    let content_text = response["result"]["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("");

    assert!(
        content_text.contains("permission_denied") || content_text.contains("No capability token"),
        "Missing token must produce permission error, got: {content_text}"
    );
}

// ── Happy-path integration tests ──────────────────────────────────────

// REQ: INVAR-003 — spec/test/invariant with a valid capability token creates a traceability record
#[test]
fn test_invariant_creates_traceability() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let token = make_capability_token("test/invariant", DelegationAction::Write);

    // First, capture a spec so we have a valid spec_id to reference
    let capture_token = make_capability_token("capture", DelegationAction::Write);
    let capture_content = client.extract_content(
        "spec_goal_capture",
        json!({
            "description": "Test spec for invariant traceability",
            "category": "domain",
            "domain_anchor": "hkask",
            "capability_token": capture_token,
        }),
    );

    let spec_id = capture_content["spec_id"]
        .as_str()
        .expect("spec_goal_capture must return a spec_id")
        .to_string();

    // Now call spec/test/invariant with a valid capability token
    let result = client.extract_content(
        "spec_test_invariant",
        json!({
            "spec_id": spec_id,
            "seam": "test-seam",
            "invariant": "creates-traceability-record",
            "category": "PublicInterface",
            "capability_token": token,
        }),
    );

    assert!(
        result.get("invariant_id").is_some(),
        "Response must contain invariant_id, got: {result}"
    );
    assert_eq!(
        result["status"].as_str(),
        Some("recorded"),
        "Status must be 'recorded', got: {result}"
    );
}

// REQ: VERIFY-003 — spec/test/verify with a valid capability token reports coverage
#[test]
fn test_verify_reports_coverage() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let token = make_capability_token("test/verify", DelegationAction::Read);

    let result = client.extract_content(
        "spec_test_verify",
        json!({
            "category": "domain",
            "capability_token": token,
        }),
    );

    // Verify response must contain the expected fields
    assert!(
        result.get("total_requirements").is_some(),
        "Response must contain total_requirements, got: {result}"
    );
    assert!(
        result.get("tested").is_some(),
        "Response must contain tested, got: {result}"
    );
    assert!(
        result.get("gaps").is_some(),
        "Response must contain gaps, got: {result}"
    );
    assert!(
        result.get("complete").is_some(),
        "Response must contain complete, got: {result}"
    );
}

// REQ: VERIFY-004 — spec/test/verify with no specs returns complete=false and total_requirements=0
#[test]
fn test_verify_empty_store_reports_no_requirements() {
    let mut client = McpClient::new();
    let _ = client.initialize();

    let token = make_capability_token("test/verify", DelegationAction::Read);

    let result = client.extract_content(
        "spec_test_verify",
        json!({
            "capability_token": token,
        }),
    );

    assert_eq!(
        result["total_requirements"].as_u64(),
        Some(0),
        "Empty store must have 0 total_requirements, got: {result}"
    );
    assert_eq!(
        result["complete"].as_bool(),
        Some(false),
        "Empty store must not be complete, got: {result}"
    );
}
