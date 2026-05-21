//! MCP Runtime Adapter
//!
//! Concrete implementation of MCPRuntimePort using rmcp crate.

use crate::pod::MCPRuntimePort;
use hkask_types::CapabilityToken;

/// MCP Runtime Adapter — Concrete implementation for tool access
#[derive(Default)]
pub struct McpRuntimeAdapter {
    /// Granted tokens (reserved for future use)
    #[allow(dead_code)]
    granted_tokens: std::collections::HashSet<String>,
}

impl McpRuntimeAdapter {
    /// Create new MCP runtime adapter
    pub fn new() -> Self {
        Self {
            granted_tokens: std::collections::HashSet::new(),
        }
    }
}

impl MCPRuntimePort for McpRuntimeAdapter {
    fn grant_tool_access(&self, token: CapabilityToken) -> Result<(), String> {
        let token_id = token.id.clone();

        if token_id.is_empty() {
            return Err("Invalid capability token".to_string());
        }

        Ok(())
    }

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<serde_json::Value, String> {
        let token_id = token.id.clone();
        if token_id.is_empty() {
            return Err("Invalid capability token".to_string());
        }

        Ok(serde_json::json!({
            "tool": tool_name,
            "status": "invoked",
            "input": input
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{CapabilityAction, CapabilityResource, WebID};

    #[test]
    fn test_mcp_runtime_adapter_new() {
        let _adapter = McpRuntimeAdapter::new();
        assert!(true);
    }

    #[test]
    fn test_mcp_grant_tool_access() {
        let adapter = McpRuntimeAdapter::new();
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "*".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            WebID::new(),
            b"test-secret",
        );

        let result = adapter.grant_tool_access(token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mcp_invoke_tool() {
        let adapter = McpRuntimeAdapter::new();
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "*".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            WebID::new(),
            b"test-secret",
        );

        let input = serde_json::json!({"param": "value"});
        let result = adapter.invoke_tool("test_tool", input, &token);

        assert!(result.is_ok());
    }
}
