//! MCP Runtime Port — Hexagonal boundary for MCP tool invocation

use hkask_capability::DelegationToken;

/// Port trait for MCP runtime operations
///
/// Implementations:
/// - `FullMcpAdapter` — Full MCP adapter with capability checking + tool dispatch
/// - `CapabilityOnlyAdapter` — Capability verification only (no tool invocation)
/// - Mock implementations for testing
pub trait MCPRuntimePort: Send + Sync {
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), crate::error::AgentMcpError>;

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, crate::error::AgentMcpError>;

    /// Resolve the server ID for a tool.
    ///
    /// Returns the server ID (e.g., "inference", "cns") that owns the tool,
    /// or `None` if the tool is not found or the runtime doesn't support resolution.
    /// Used by `PodContext::invoke_tool` to route through `GovernedTool` with
    /// accurate energy estimation and CNS observability.
    fn resolve_tool_server(&self, _tool_name: &str) -> Option<String> {
        None
    }
}
