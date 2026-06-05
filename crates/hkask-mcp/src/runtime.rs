//! MCP runtime for hKask
//!
//! Manages MCP server connections, tool discovery, and lifecycle.
//! Supports both static metadata registration and dynamic server
//! startup with live rmcp client transport.

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

/// Tool information metadata
#[derive(Debug, Clone)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
    /// Server that provides this tool
    pub server_id: String,
    /// Required capability (if any)
    pub required_capability: Option<String>,
}

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
pub enum ServerStartError {
    #[error("Failed to spawn MCP server process: {0}")]
    SpawnFailed(String),
    #[error("Failed to connect to MCP server (handshake): {0}")]
    ConnectFailed(String),
    #[error("Failed to discover tools from server: {0}")]
    DiscoveryFailed(String),
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
    /// Create new MCP runtime
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
    /// If a server with the same ID is already connected, returns `Ok(())`.
    pub async fn start_server(
        &self,
        server_id: &str,
        command: &str,
    ) -> Result<(), ServerStartError> {
        // Already connected — idempotent
        if self.connections.read().await.contains_key(server_id) {
            info!(
                target: "hkask.mcp",
                server_id = %server_id,
                "Server already connected"
            );
            return Ok(());
        }

        let transport = TokioChildProcess::new(Command::new(command))
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

        // Store the connection and cancellation token
        self.connections
            .write()
            .await
            .insert(server_id.to_string(), peer);
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
    pub async fn get_peer(&self, server_id: &str) -> Option<Peer<RoleClient>> {
        self.connections.read().await.get(server_id).cloned()
    }

    /// Call a tool on a connected server directly via the Peer.
    ///
    /// Lower-level than `RawMcpToolPort::invoke` — no governance membrane.
    /// Used internally by `RawMcpToolPort`.
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

    /// Invoke a tool by name, looking up the server automatically.
    ///
    /// This is the highest-level convenience method: finds the server that
    /// owns the tool, calls it through the live connection, and parses
    /// the result into a `Value`. Returns `None` if the tool is not found
    /// or the server is not connected.
    pub async fn invoke_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Map<String, Value>,
    ) -> Option<Result<Value, String>> {
        let server_id = self.get_tool_info(tool_name).await?.server_id;

        let result = match self.call_tool(&server_id, tool_name, arguments).await {
            Ok(r) => r,
            Err(e) => return Some(Err(e.to_string())),
        };

        if result.is_error.unwrap_or(false) {
            let msg = result
                .content
                .iter()
                .filter_map(|c| match &**c {
                    rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some(Err(msg));
        }

        Some(Ok(crate::raw_tool_port::parse_call_result(&result)))
    }

    /// Shut down a specific managed server process.
    pub async fn shutdown_server(&self, server_id: &str) {
        if let Some(cancel) = self.cancellation_tokens.write().await.remove(server_id) {
            cancel.cancel();
        }
        self.connections.write().await.remove(server_id);
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
    pub async fn discover_tools(&self) -> Vec<String> {
        let tool_registry = self.tool_registry.read().await;
        tool_registry.keys().cloned().collect()
    }

    /// Get tool definition
    pub async fn get_tool(&self, tool_name: &str) -> Option<McpTool> {
        let tool_registry = self.tool_registry.read().await;
        let server_id = tool_registry.get(tool_name)?;

        let servers = self.servers.read().await;
        let server = servers.get(server_id)?;

        server.tools.iter().find(|t| t.name == tool_name).cloned()
    }

    /// Get tool information with metadata
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
                required_capability: None, // Future: load from config
            })
    }

    /// Check if a tool exists
    pub async fn tool_exists(&self, tool_name: &str) -> bool {
        let tool_registry = self.tool_registry.read().await;
        tool_registry.contains_key(tool_name)
    }

    /// List all registered servers
    pub async fn list_servers(&self) -> Vec<McpServer> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }
}

impl Default for McpRuntime {
    fn default() -> Self {
        Self::new()
    }
}
