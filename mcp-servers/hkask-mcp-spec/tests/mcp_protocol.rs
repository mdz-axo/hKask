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

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};

/// Global counter for JSON-RPC request IDs.
static NEXT_ID: AtomicU16 = AtomicU16::new(1);

/// Next available request ID.
fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) as u64
}

/// Spawn the spec server as a child process with piped stdin/stdout.
///
/// Sets `HKASK_OCAP_SECRET` to all-zeros hex so the capability token
/// derivation is deterministic. Also sets `RUST_LOG=off` to prevent
/// any log output from interfering with the JSON-RPC stream on stderr
/// (belt-and-suspenders after the tracing-to-stderr fix).
fn spawn_server() -> Child {
    let binary = std::env::var("CARGO_BIN_EXE_hkask_mcp_spec").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        format!("{}/../../target/debug/hkask-mcp-spec", manifest_dir)
    });

    Command::new(&binary)
        .env(
            "HKASK_OCAP_SECRET",
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
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

    let response = client.send_request(
        "tools/call",
        json!({
            "name": "spec_test_invariant",
            "arguments": {
                "spec_id": "00000000-0000-0000-0000-000000000001",
                "seam": "spec-test-invariant",
                "invariant": "rejects-missing-token",
                "category": "PublicInterface"
            }
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

    let response = client.send_request(
        "tools/call",
        json!({
            "name": "spec_test_verify",
            "arguments": {
                "category": "domain"
            }
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
