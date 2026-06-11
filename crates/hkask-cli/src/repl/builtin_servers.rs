//! Start built-in MCP servers at REPL boot.
//!
//! Each server is spawned as a child process via `McpRuntime::start_server()`,
//! which performs the MCP handshake and discovers tools dynamically.
//! Servers that fail to start (binary not found, missing credentials) are
//! logged and skipped — their tools simply won't be available.

use hkask_mcp::runtime::McpRuntime;

/// MCP servers to start at REPL boot.
///
/// Each entry maps `(server_id, binary_name)`. The binary must be on PATH
/// or specified via the `HKASK_MCP_{ID}_BIN` environment variable.
const BUILTIN_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("spec", "hkask-mcp-spec"),
    ("research", "hkask-mcp-research"),
    ("fmp", "hkask-mcp-fmp"),
    ("telnyx", "hkask-mcp-telnyx"),
    ("fal", "hkask-mcp-fal"),
    ("doc-knowledge", "hkask-mcp-doc-knowledge"),
    ("markitdown", "hkask-mcp-markitdown"),
    ("replica", "hkask-mcp-replica"),
];

/// Start all built-in MCP servers and discover their tools.
///
/// Returns the number of servers that started successfully.
/// Servers that fail to start are logged and skipped.
pub async fn start_builtin_servers(runtime: &McpRuntime) -> usize {
    let mut started = 0;

    for (server_id, command) in BUILTIN_SERVERS {
        match runtime.start_server(server_id, command).await {
            Ok(()) => {
                started += 1;
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.repl",
                    server_id = %server_id,
                    error = %e,
                    "Failed to start MCP server — tools will be unavailable"
                );
            }
        }
    }

    started
}
