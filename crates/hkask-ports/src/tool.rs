use hkask_capability::DelegationToken;
use hkask_types::NotFound;

/// Governance membrane error types.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolPortError {
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Gas budget exceeded: {0}")]
    EnergyBudgetExceeded(String),
    #[error("Tool not found: {0}")]
    NotFound(NotFound),
    #[error("Tool invocation failed: {0}")]
    InvocationFailed(String),
}

impl From<NotFound> for ToolPortError {
    fn from(nf: NotFound) -> Self {
        ToolPortError::NotFound(nf)
    }
}

/// Governance membrane for MCP tool invocation.
///
/// GovernedTool checks: OCAP authority → budget → emit span → delegate → account cost → emit outcome.
/// Impl: `McpDispatcher` (hkask-mcp)
///
/// # Authentication asymmetry
///
/// `invoke()` requires a [`DelegationToken`] — every tool execution is OCAP-gated (P4).
/// `discover_tools()` and `get_tool_info()` are **intentionally unauthenticated** — tool
/// schemas are public metadata (the agent must know what tools exist before it can request
/// a token to use them). This follows the MCP protocol's own design: `tools/list` is an
/// unauthenticated handshake. OCAP enforcement applies at the actuator boundary
/// (`invoke`), not the sensor boundary (`discover`).
pub trait ToolPort: Send + Sync {
    /// Invoke a tool. Requires a [`DelegationToken`] proving OCAP authorization.
    ///
    /// pre:  token must be valid and not expired
    /// post: returns tool output or `ToolPortError::CapabilityDenied` if token is insufficient
    fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
        token: &DelegationToken,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, ToolPortError>> + Send;

    /// Discover available tools. Public metadata — no token required.
    ///
    /// Tool schemas are public per the MCP protocol design:
    /// `tools/list` is an unauthenticated handshake. OCAP enforcement
    /// applies at the actuator boundary (`invoke`), not here.
    fn discover_tools(&self) -> impl std::future::Future<Output = Vec<String>> + Send;

    /// Get metadata for a specific tool. Public metadata — no token required.
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
