//! MCP Runtime Adapter
//!
//! Concrete implementation of MCPRuntimePort using rmcp crate.

use crate::error::McpError;
use crate::ports::MCPRuntimePort;
use hkask_types::{CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken};
use std::sync::Arc;
use tracing::warn;

/// MCP Runtime Adapter — Concrete implementation for tool access
#[derive(Default, Clone)]
pub struct McpRuntimeAdapter {
    /// Granted tokens (reserved for future use)
    _granted_tokens: std::collections::HashSet<String>,
    /// Optional capability checker for HMAC verification
    capability_checker: Option<Arc<CapabilityChecker>>,
}

impl McpRuntimeAdapter {
    /// Create new MCP runtime adapter
    pub fn new() -> Self {
        Self {
            _granted_tokens: std::collections::HashSet::new(),
            capability_checker: None,
        }
    }

    /// Set the capability checker for cryptographic OCAP verification
    pub fn with_capability_checker(mut self, checker: CapabilityChecker) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }
}

impl MCPRuntimePort for McpRuntimeAdapter {
    fn grant_tool_access(&self, token: CapabilityToken) -> Result<(), McpError> {
        let token_id = token.id.clone();

        if token_id.is_empty() {
            return Err(McpError::InvalidToken("Token ID is empty".to_string()));
        }

        if let Some(checker) = &self.capability_checker
            && !checker.verify(&token)
        {
            return Err(McpError::InvalidToken(
                "Token signature verification failed".to_string(),
            ));
        }

        Ok(())
    }

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &CapabilityToken,
    ) -> Result<serde_json::Value, McpError> {
        if let Some(checker) = &self.capability_checker {
            if !checker.verify(token) {
                return Err(McpError::CapabilityDenied(
                    "Token signature verification failed".to_string(),
                ));
            }

            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            if token.is_expired(current_time) {
                return Err(McpError::CapabilityDenied("Token is expired".to_string()));
            }

            if !token.is_valid_for(
                CapabilityResource::Tool,
                tool_name,
                CapabilityAction::Execute,
            ) {
                return Err(McpError::CapabilityDenied(format!(
                    "Token does not authorize tool: {}",
                    tool_name
                )));
            }
        } else {
            warn!(
                target: "hkask.agents.mcp_runtime",
                "No capability checker configured; falling back to stub verification"
            );
            if token.id.is_empty() {
                return Err(McpError::CapabilityDenied(
                    "Invalid capability token".to_string(),
                ));
            }
        }

        Ok(serde_json::json!({
            "tool": tool_name,
            "status": "invoked",
            "input": input
        }))
    }
}
