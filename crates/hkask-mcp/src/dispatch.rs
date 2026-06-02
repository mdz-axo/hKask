//! MCP dispatch — Communication loop concerns
//!
//! Routes tool calls through MCP runtime. Governance (capability
//! verification, token lifecycle) is delegated to `McpGovernor`.
//!
//! This split enforces the authority DAG: Cybernetics governs
//! Communication. The dispatcher is the dumb transport pipe; the
//! governor is the gatekeeper.

use crate::governor::McpGovernor;
use crate::runtime::{McpRuntime, McpTool};
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::{CapabilityToken, WebID};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// MCP dispatcher — Communication-layer tool routing.
///
/// Wraps `McpRuntime` for tool discovery and invocation.
/// All governance checks are delegated to `McpGovernor`.
pub struct McpDispatcher {
    /// MCP runtime for tool discovery and invocation
    runtime: McpRuntime,
    /// Cybernetics governor for capability governance
    governor: Arc<McpGovernor>,
}

impl McpDispatcher {
    /// Create a dispatcher with a runtime and a secret for the capability checker.
    pub fn new(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self {
            runtime,
            governor: Arc::new(McpGovernor::new(secret)),
        }
    }

    /// Access the governor for governance operations.
    pub fn governor(&self) -> &Arc<McpGovernor> {
        &self.governor
    }

    /// Issue capability token to a bot (delegates to governor).
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        self.governor.issue_capability(tool_name, from, to)
    }

    /// Get tool definition
    pub async fn get_tool(&self, tool_name: &str) -> Option<McpTool> {
        self.runtime.get_tool(tool_name).await
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
    }

    /// Get MCP runtime
    pub fn runtime(&self) -> &McpRuntime {
        &self.runtime
    }

    /// Invoke a tool with capability checking
    ///
    /// When a `CapabilityToken` is provided, governance is delegated
    /// to the `McpGovernor`. When `None`, falls back to legacy
    /// bot-capabilities string match.
    pub async fn invoke_async(
        &self,
        bot_id: &WebID,
        tool_name: &str,
        input: Value,
        token: Option<&CapabilityToken>,
    ) -> Result<Value> {
        // Delegate governance to the governor
        if let Some(token) = token {
            self.governor
                .authorize(token, tool_name)
                .await
                .map_err(TemplateError::CapabilityDenied)?;
        } else {
            self.governor
                .authorize_legacy(bot_id, tool_name)
                .await
                .map_err(TemplateError::CapabilityDenied)?;
        }

        // Check if tool exists
        if !self.runtime.tool_exists(tool_name).await {
            return Err(TemplateError::Mcp(format!("Tool not found: {}", tool_name)));
        }

        info!(
            target: "hkask.mcp",
            bot_id = ?bot_id,
            tool_name = %tool_name,
            token_id = token.map(|t| t.id.as_str()).unwrap_or("none"),
            "Dispatching tool call"
        );

        let tool_info = self
            .runtime
            .get_tool_info(tool_name)
            .await
            .ok_or_else(|| TemplateError::Mcp(format!("Tool info not found: {}", tool_name)))?;

        let result = self
            .runtime
            .call_tool(&tool_info.server_id, tool_name, input, token)
            .await
            .map_err(|e| TemplateError::Mcp(format!("Tool call failed: {}", e)))?;

        Ok(result)
    }
}

#[async_trait::async_trait]
impl McpPort for McpDispatcher {
    async fn discover_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
    }

    async fn invoke(
        &self,
        tool_name: &str,
        input: Value,
        token: &CapabilityToken,
    ) -> Result<Value> {
        // Delegate governance to the governor
        self.governor
            .authorize(token, tool_name)
            .await
            .map_err(TemplateError::CapabilityDenied)?;

        let tool_info = self
            .runtime
            .get_tool_info(tool_name)
            .await
            .ok_or_else(|| TemplateError::Mcp(format!("Tool not found: {}", tool_name)))?;

        self.runtime
            .call_tool(&tool_info.server_id, tool_name, input, Some(token))
            .await
            .map_err(|e| TemplateError::Mcp(format!("Tool call failed: {}", e)))
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<hkask_templates::ports::ToolInfo> {
        self.runtime
            .get_tool_info(tool_name)
            .await
            .map(|t| hkask_templates::ports::ToolInfo {
                name: t.name,
                description: t.description,
                input_schema: t.input_schema,
                server_id: t.server_id,
                required_capability: t.required_capability,
            })
    }
}
