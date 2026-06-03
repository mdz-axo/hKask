//! MCP runtime for hKask
//!
//! Manages MCP server connections, tool discovery, and lifecycle.
//! Integrates with capability security and energy budget enforcement.

use hkask_types::DelegationToken;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Tool information metadata
#[derive(Debug, Clone)]
pub(crate) struct ToolInfo {
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
    /// Connection status
    pub connected: bool,
}

/// MCP runtime manager
pub struct McpRuntime {
    /// Registered MCP servers
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    /// Tool registry (tool_name -> server_id)
    tool_registry: Arc<RwLock<HashMap<String, String>>>,
}

impl McpRuntime {
    /// Create new MCP runtime
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tool_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an MCP server
    pub async fn register_server(&self, mut server: McpServer) {
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

        // Mark as connected
        server.connected = true;

        servers.insert(server.id.clone(), server);
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
    pub(crate) async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
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

    /// Get server by ID
    pub async fn get_server(&self, server_id: &str) -> Option<McpServer> {
        let servers = self.servers.read().await;
        servers.get(server_id).cloned()
    }

    /// List all registered servers
    pub async fn list_servers(&self) -> Vec<McpServer> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Get tool count
    pub async fn tool_count(&self) -> usize {
        let tool_registry = self.tool_registry.read().await;
        tool_registry.len()
    }

    /// Unregister a server
    pub async fn unregister_server(&self, server_id: &str) {
        let mut servers = self.servers.write().await;
        let mut tool_registry = self.tool_registry.write().await;

        if let Some(server) = servers.remove(server_id) {
            // Remove tools from registry
            for tool in &server.tools {
                tool_registry.remove(&tool.name);
            }

            info!(
                target: "hkask.mcp",
                server_id = %server_id,
                "Unregistered MCP server"
            );
        }
    }

    /// Call a tool by name with arguments
    ///
    /// Dispatches the tool call to the appropriate MCP server transport.
    /// The optional `token` travels with the call for audit logging;
    /// verification is the responsibility of the dispatch layer (`McpDispatcher`).
    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        _arguments: serde_json::Value,
        token: Option<&DelegationToken>,
    ) -> Result<serde_json::Value, String> {
        info!(
            target: "hkask.mcp",
            tool = tool_name,
            token_id = token.map(|t| t.id.as_str()).unwrap_or("none"),
            "Tool call dispatched"
        );

        // Check if tool exists
        if !self.tool_exists(tool_name).await {
            return Err(format!("Tool '{}' not found", tool_name));
        }

        // Get server and transport
        let servers = self.servers.read().await;
        let _server = servers
            .get(server_id)
            .ok_or_else(|| format!("Server '{}' not found", server_id))?;

        // No transport available — tool dispatch is deferred
        Err(format!(
            "No transport available for server '{}'; tool dispatch not yet implemented",
            server_id
        ))
    }
}

impl Default for McpRuntime {
    fn default() -> Self {
        Self::new()
    }
}
