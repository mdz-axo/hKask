//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` via `AgentService::build()` and starts the
//! axum HTTP server. CLI commands issued while the server is running
//! operate on the same shared state.


use hkask_api::ApiState;
use hkask_mcp::runtime::McpRuntime;

/// MCP servers to start for the API.
///
/// Each entry maps `(server_id, binary_name)`. The binary must be on PATH
/// or specified via the `HKASK_MCP_{ID}_BIN` environment variable.
const API_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("spec", "hkask-mcp-spec"),
    ("research", "hkask-mcp-research"),
    ("companies", "hkask-mcp-companies"),
    ("communication", "hkask-mcp-communication"),
    ("fal", "hkask-mcp-media"),
    ("docproc", "hkask-mcp-docproc"),
    ("training", "hkask-mcp-training"),
    ("replica", "hkask-mcp-replica"),
];

/// Run the API server, sharing state with the CLI.
///
/// Resolves configuration from the keystore and environment, builds a
/// `AgentService` with all shared infrastructure, starts API MCP servers
/// on the AgentService's runtime, and creates an `ApiState` from it.
