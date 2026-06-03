//! MCP Runtime Port — Hexagonal boundary for MCP tool invocation

use hkask_types::DelegationToken;

/// Port trait for MCP runtime operations
///
/// Implementations:
/// - `McpRuntimeAdapter` — Production adapter via rmcp
/// - Mock implementations for testing
pub trait MCPRuntimePort: Send + Sync {
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), crate::error::McpError>;

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, crate::error::McpError>;
}
