use hkask_capability::DelegationToken;

/// Governance membrane error types.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolPortError {
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Gas budget exceeded: {0}")]
    EnergyBudgetExceeded(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Tool invocation failed: {0}")]
    InvocationFailed(String),
}

/// Governance membrane for MCP tool invocation.
/// GovernedTool checks: OCAP authority → budget → emit span → delegate → account cost → emit outcome.
/// Impl: `McpDispatcher` (hkask-mcp)
pub trait ToolPort: Send + Sync {
    /// Token proves agent authorization for this invocation.
    fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
        token: &DelegationToken,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, ToolPortError>> + Send;

    fn discover_tools(&self) -> impl std::future::Future<Output = Vec<String>> + Send;

    fn get_tool_info(
        &self,
        tool_name: &str,
    ) -> impl std::future::Future<Output = Option<ToolInfo>> + Send;
}

/// Canonical tool metadata for OCAP capability matching.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_id: String,
    /// The capability required to invoke this tool, derived from the server ID.
    /// Maps `hkask-mcp-<domain>` → `tool:<domain>:execute`.
    /// `None` for servers that don't follow the `hkask-mcp-` naming convention.
    pub required_capability: Option<String>,
}
