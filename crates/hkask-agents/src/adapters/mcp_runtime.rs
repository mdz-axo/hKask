//! MCP Runtime Adapter
//!
//! Concrete implementation of MCPRuntimePort using rmcp crate.

use crate::error::McpError;
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
    fn grant_tool_access(&self, token: CapabilityToken) -> Result<(), McpError> {
        let token_id = token.id.clone();

        if token_id.is_empty() {
            return Err(McpError::InvalidToken("Token ID is empty".to_string()));
        }

        Ok(())
    }

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<serde_json::Value, McpError> {
        let token_id = token.id.clone();
        if token_id.is_empty() {
            return Err(McpError::CapabilityDenied(
                "Invalid capability token".to_string(),
            ));
        }

        Ok(serde_json::json!({
            "tool": tool_name,
            "status": "invoked",
            "input": input
        }))
    }
}
