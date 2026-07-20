//! MCP runtime for hKask
//!
//! Manages MCP server connections, tool discovery, and lifecycle.
//! Servers are spawned as child processes via `start_server()`, which
//! performs the MCP handshake, discovers tools dynamically, and stores
//! live `Peer<RoleClient>` connections. `shutdown_all()` terminates
//! all managed processes.

use hkask_ports::ToolInfo;
use rmcp::model::CallToolRequestParams;
use rmcp::service::{Peer, RoleClient, ServiceExt};
use rmcp::transport::TokioChildProcess;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::info;

/// MCP tool definition
#[derive(Debug, Clone)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
    /// MCP server that provides this tool
    pub server_id: String,
}

impl McpTool {
    /// Validate tool input arguments against the tool's JSON Schema.
    ///
    /// pre:  input is a valid JSON Value
    /// post: returns Ok(()) if input conforms to self.input_schema
    /// post: returns Err with validation errors if input violates schema
    /// post: returns Ok(()) if input_schema is empty or not a valid JSON Schema (graceful)
    #[must_use = "result must be used"]
    pub fn validate_input(&self, input: &Value) -> Result<(), Vec<String>> {
        // If schema is empty or not an object, skip validation (graceful degradation)
        if !self.input_schema.is_object()
            || self
                .input_schema
                .as_object()
                .map(|o| o.is_empty())
                .unwrap_or(true)
        {
            return Ok(());
        }

        match jsonschema::validator_for(&self.input_schema) {
            Ok(validator) => {
                let errors: Vec<String> = validator
                    .iter_errors(input)
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
            Err(_) => {
                // Schema compilation failed — graceful degradation
                Ok(())
            }
        }
    }
}

/// MCP server registration
#[derive(Debug, Clone)]
pub struct McpServer {
    /// Server ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Tools provided by this server
    pub tools: Vec<McpTool>,
}

/// Error type for MCP server startup.
#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum ServerStartError {
    #[error("Failed to spawn MCP server process: {0}")]
    SpawnFailed(String),
    #[error("Failed to connect to MCP server (handshake): {0}")]
    ConnectFailed(String),
    #[error("Failed to discover tools from server: {0}")]
    DiscoveryFailed(String),
}

/// Resolve the binary path for an MCP server.
///
/// 1. Check `HKASK_MCP_{SERVER_ID_UPPER}_BIN` environment variable.
///    Example: `HKASK_MCP_FILESYSTEM_BIN` for server_id="filesystem".
/// 2. Fall back to the provided command name (PATH-based resolution).
///
/// This is the implementation of the contract documented in
/// `crates/hkask-cli/src/repl/builtin_servers.rs`.
fn resolve_mcp_binary(server_id: &str, command: &str) -> String {
    let env_var = format!("HKASK_MCP_{}_BIN", server_id.to_uppercase());
    if let Ok(explicit_path) = std::env::var(&env_var)
        && !explicit_path.is_empty()
    {
        tracing::info!(
            target: "hkask.mcp",
            server_id = %server_id,
            env_var = %env_var,
            binary = %explicit_path,
            "MCP binary resolved via env var"
        );
        return explicit_path;
    }
    command.to_string()
}

/// MCP runtime manager
#[derive(Clone)]
pub struct McpRuntime {
    /// Registered MCP servers (metadata)
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    /// Tool registry (tool_name -> server_id)
    tool_registry: Arc<RwLock<HashMap<String, String>>>,
    /// Live connections to MCP server processes, keyed by server ID
    connections: Arc<RwLock<HashMap<String, Peer<RoleClient>>>>,
    /// Cancellation tokens for managed server processes
    cancellation_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl McpRuntime {
    /// Create a new MCP runtime.
    ///
    /// post: returns McpRuntime with empty servers, tool_registry, connections
    #[must_use]
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tool_registry: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            cancellation_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an MCP server (metadata only, no live connection).
    pub async fn register_server(&self, server: McpServer) {
        let mut servers = self.servers.write().await;
        let mut tool_registry = self.tool_registry.write().await;

        info!(
            target: "hkask.mcp",
            server_id = %server.id,
            server_name = %server.name,
            tools = server.tools.len(),
            "Registering MCP server"
        );

        // Register tools
        for tool in &server.tools {
            tool_registry.insert(tool.name.clone(), server.id.clone());
        }

        servers.insert(server.id.clone(), server);
    }

    /// Start an MCP server process and connect via rmcp stdio transport.
    ///
    /// Spawns the server as a child process, performs the MCP handshake,
    /// discovers tools via `list_all_tools()`, stores the live connection,
    /// and registers the discovered tools in the runtime.
    ///
    /// `extra_env` is a map of environment variables to set on the child
    /// process (e.g., `HKASK_MCP_HOST`). These override inherited env vars.
    ///
    /// If a server with the same ID is already connected, returns `Ok(())`.
    #[allow(private_interfaces)]
    #[must_use = "result must be used"]
    pub async fn start_server(
        &self,
        server_id: &str,
        command: &str,
    ) -> Result<(), ServerStartError> {
        self.start_server_with_env(server_id, command, std::collections::HashMap::new())
            .await
    }

    /// Like `start_server`, but with extra environment variables for the child process.
    #[must_use = "result must be used"]
    pub async fn start_server_with_env(
        &self,
        server_id: &str,
        command: &str,
        extra_env: std::collections::HashMap<String, String>,
    ) -> Result<(), ServerStartError> {
        // Acquire write lock first to prevent TOCTOU races.
        let mut connections = self.connections.write().await;
        if connections.contains_key(server_id) {
            info!(
                target: "hkask.mcp",
                server_id = %server_id,
                "Server already connected"
            );
            return Ok(());
        }

        // Resolve the binary path: check HKASK_MCP_{ID}_BIN first, then fall back
        // to PATH-based resolution. The env var allows pointing at a specific build
        // (e.g., target/debug/hkask-mcp-filesystem) without polluting PATH.
        //
        // P12 replicant-host-mandate: the binary path is not a secret — it's a
        // deployment-time configuration, not an ambient authority.
        let binary = resolve_mcp_binary(server_id, command);

        let mut cmd = Command::new(&binary);
        for (key, value) in &extra_env {
            cmd.env(key, value);
        }
        let transport = TokioChildProcess::new(cmd)
            .map_err(|e| ServerStartError::SpawnFailed(e.to_string()))?;

        let running = ().into_dyn().serve(transport).await.map_err(|e| {
            ServerStartError::ConnectFailed(format!("Handshake with '{}' failed: {}", server_id, e))
        })?;

        let peer = running.peer().clone();
        let cancel = CancellationToken::new();

        // Keep the RunningService alive in a background task.
        // When `cancel` fires, the service loop exits and the child
        // process is cleaned up by rmcp's DropGuard.
        let bg_cancel = cancel.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = running.waiting() => {}
                _ = bg_cancel.cancelled() => {}
            }
        });

        // Discover tools from the live server
        let tools = peer.list_all_tools().await.map_err(|e| {
            ServerStartError::DiscoveryFailed(format!(
                "list_all_tools from '{}' failed: {}",
                server_id, e
            ))
        })?;

        // Insert into the already-held write lock
        connections.insert(server_id.to_string(), peer);
        // Drop the write lock before acquiring the cancellation_tokens lock
        drop(connections);

        self.cancellation_tokens
            .write()
            .await
            .insert(server_id.to_string(), cancel);

        // Register the server and its discovered tools
        let server = McpServer {
            id: server_id.to_string(),
            name: server_id.to_string(),
            tools: tools
                .into_iter()
                .map(|t| McpTool {
                    name: t.name.to_string(),
                    description: t.description.map(|d| d.to_string()).unwrap_or_default(),
                    input_schema: Value::Object((*t.input_schema).clone()),
                    server_id: server_id.to_string(),
                })
                .collect(),
        };

        info!(
            target: "hkask.mcp",
            server_id = %server_id,
            tools = server.tools.len(),
            "MCP server started and tools discovered"
        );

        self.register_server(server).await;

        Ok(())
    }

    /// Get a live Peer connection for a server (if connected).
    pub(crate) async fn get_peer(&self, server_id: &str) -> Option<Peer<RoleClient>> {
        self.connections.read().await.get(server_id).cloned()
    }

    /// Call a tool on a connected server directly via the Peer.
    ///
    /// Lower-level than `RawMcpToolPort::invoke` — no governance membrane.
    /// Used internally by `RawMcpToolPort` and by external callers (QA runner).
    #[must_use = "result must be used"]
    pub async fn call_tool(
        &self,
        server_id: &str,
        tool: &str,
        arguments: serde_json::Map<String, Value>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::service::ServiceError> {
        let peer = self
            .get_peer(server_id)
            .await
            .ok_or_else(|| rmcp::service::ServiceError::TransportClosed)?;

        let params = CallToolRequestParams::new(tool.to_string()).with_arguments(arguments);
        peer.call_tool(params).await
    }

    /// Shut down all managed server processes.
    pub async fn shutdown_all(&self) {
        let mut tokens = self.cancellation_tokens.write().await;
        for (_, cancel) in tokens.drain() {
            cancel.cancel();
        }
        drop(tokens);
        self.connections.write().await.clear();
    }

    /// Discover tools from all registered servers
    #[must_use]
    pub async fn discover_tools(&self) -> Vec<String> {
        let tool_registry = self.tool_registry.read().await;
        tool_registry.keys().cloned().collect()
    }

    /// Get tool definition
    #[must_use]
    pub async fn get_tool(&self, tool_name: &str) -> Option<McpTool> {
        let tool_registry = self.tool_registry.read().await;
        let server_id = tool_registry.get(tool_name)?;

        let servers = self.servers.read().await;
        let server = servers.get(server_id)?;

        server.tools.iter().find(|t| t.name == tool_name).cloned()
    }

    /// Get tool information with metadata
    #[must_use]
    pub async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        let tool_registry = self.tool_registry.read().await;
        let server_id = tool_registry.get(tool_name)?;

        let servers = self.servers.read().await;
        let server = servers.get(server_id)?;

        server
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .map(|t| ToolInfo {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                server_id: server_id.clone(),
                required_capability: hkask_capability::capability_from_server_id(server_id),
                taint: hkask_types::ToolTaint::Pure,
            })
    }

    /// Check if a tool exists
    pub(crate) async fn tool_exists(&self, tool_name: &str) -> bool {
        let tool_registry = self.tool_registry.read().await;
        tool_registry.contains_key(tool_name)
    }

    /// List all registered servers
    #[must_use]
    pub async fn list_servers(&self) -> Vec<McpServer> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Get all registered servers as a name→server map (for health checks).
    pub async fn servers(&self) -> HashMap<String, McpServer> {
        self.servers.read().await.clone()
    }

    /// Count live Peer connections (for health checks).
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get live connection map (for health checks).
    pub async fn connections(&self) -> HashMap<String, Peer<RoleClient>> {
        self.connections.read().await.clone()
    }
}

impl Default for McpRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ── ToolPort implementation ──────────────────────────────────────────────
//
// McpRuntime implements ToolPort directly. This eliminates the former
// RawMcpToolPort adapter layer — GovernedTool<McpRuntime> is now the single
// tool surface. The invoke logic (live-connection check, error-flag check,
// result parsing) lives here, not in a separate wrapper struct.

impl hkask_ports::ToolPort for McpRuntime {
    fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: Value,
        _token: &hkask_capability::DelegationToken,
    ) -> hkask_ports::ToolFuture<'_, Result<Value, hkask_ports::ToolPortError>> {
        Box::pin(async move {
            // Try the live connection first
            if self.get_peer(server).await.is_some() {
                let arguments = args.as_object().cloned().unwrap_or_default();
                let result = self
                    .call_tool(server, tool, arguments)
                    .await
                    .map_err(|e| hkask_ports::ToolPortError::InvocationFailed(e.to_string()))?;

                if result.is_error.unwrap_or(false) {
                    let msg = extract_text_content(&result);
                    return Err(hkask_ports::ToolPortError::InvocationFailed(msg));
                }
                return Ok(parse_call_result(&result));
            }

            // No live connection — is the tool at least registered?
            if !self.tool_exists(tool).await {
                return Err(hkask_ports::ToolPortError::NotFound(
                    hkask_types::NotFound {
                        entity_type: "tool".to_string(),
                        id: format!("Tool '{}' not found in MCP runtime", tool),
                    },
                ));
            }

            tracing::warn!(
                target: "hkask.mcp",
                tool = %tool,
                server = %server,
                "Server registered but not connected — start it with McpRuntime::start_server()"
            );
            Err(hkask_ports::ToolPortError::InvocationFailed(format!(
                "Server '{}' is registered but not connected — call McpRuntime::start_server() first",
                server
            )))
        })
    }

    fn discover_tools(&self) -> hkask_ports::ToolFuture<'_, Vec<String>> {
        Box::pin(async move { McpRuntime::discover_tools(self).await })
    }

    fn get_tool_info(
        &self,
        tool_name: &str,
    ) -> hkask_ports::ToolFuture<'_, Option<hkask_ports::ToolInfo>> {
        Box::pin(async move { McpRuntime::get_tool_info(self, tool_name).await })
    }
}

/// Extract concatenated text from a CallToolResult's content items.
fn extract_text_content(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &**c {
            rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a CallToolResult into a JSON Value.
///
/// For a single text content item, tries to parse as JSON first
/// (structured tool responses often return JSON strings).
/// Falls back to a plain JSON string if parsing fails.
/// For multiple items, wraps them in a JSON array.
fn parse_call_result(result: &rmcp::model::CallToolResult) -> Value {
    use rmcp::model::RawContent;
    if result.content.is_empty() {
        return Value::Null;
    }

    if result.content.len() == 1
        && let RawContent::Text(text_content) = &*result.content[0]
    {
        if let Ok(v) = serde_json::from_str::<Value>(&text_content.text) {
            return v;
        }
        return Value::String(text_content.text.clone());
    }

    let items: Vec<Value> = result
        .content
        .iter()
        .map(|c| match &**c {
            RawContent::Text(t) => serde_json::from_str::<Value>(&t.text)
                .unwrap_or_else(|_| Value::String(t.text.clone())),
            RawContent::Image(i) => serde_json::json!({
                "type": "image",
                "data": i.data,
                "mimeType": i.mime_type,
            }),
            _ => Value::Null,
        })
        .collect();
    Value::Array(items)
}
