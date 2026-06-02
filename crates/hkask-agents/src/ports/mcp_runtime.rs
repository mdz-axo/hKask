//! MCP Runtime Port — Hexagonal boundary for MCP tool invocation

use hkask_types::CapabilityToken;

/// Port trait for MCP runtime operations
///
/// Implementations:
/// - `McpRuntimeAdapter` — Production adapter via rmcp
/// - Mock implementations for testing
pub(crate) trait MCPRuntimePort: Send + Sync {
    fn grant_tool_access(&self, token: CapabilityToken) -> Result<(), crate::error::McpError>;

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<serde_json::Value, crate::error::McpError>;
}
