//! MCP dispatch with capability-based security
//!
//! Dispatches tool calls through MCP with OCAP capability verification.

use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::{BotCapabilities, CapabilityChecker, CapabilityToken, WebID};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::security::SecurityGateway;

use crate::runtime::{McpRuntime, McpTool};

/// MCP dispatcher with security
pub struct McpDispatcher {
    /// MCP runtime for tool discovery
    runtime: McpRuntime,
    /// Capability checker for OCP
    capability_checker: Arc<CapabilityChecker>,
    /// Bot capabilities registry
    bot_capabilities: Arc<RwLock<std::collections::HashMap<WebID, BotCapabilities>>>,
    /// Optional security gateway for input validation, tool allow/deny
    security_gateway: Option<SecurityGateway>,
}

impl McpDispatcher {
    pub fn new(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            bot_capabilities: Arc::new(RwLock::new(std::collections::HashMap::new())),
            security_gateway: None,
        }
    }

    /// Wire in the security gateway for input validation, tool allow/deny
    pub fn with_security_gateway(mut self, sg: SecurityGateway) -> Self {
        self.security_gateway = Some(sg);
        self
    }

    /// Register bot capabilities
    pub async fn register_bot_capabilities(&self, caps: BotCapabilities) {
        let mut capabilities = self.bot_capabilities.write().await;
        capabilities.insert(caps.bot_id, caps);
    }

    /// Get bot capabilities
    pub async fn get_bot_capabilities(&self, bot_id: &WebID) -> Option<BotCapabilities> {
        let capabilities = self.bot_capabilities.read().await;
        capabilities.get(bot_id).cloned()
    }

    /// Issue capability token to a bot
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        self.capability_checker.grant_tool(tool_name, from, to)
    }

    /// Check if bot has capability for tool
    pub async fn check_capability(&self, bot_id: &WebID, tool_name: &str) -> bool {
        let capabilities = self.bot_capabilities.read().await;

        if let Some(caps) = capabilities.get(bot_id) {
            caps.has_capability(tool_name)
        } else {
            // No capabilities registered = no access
            false
        }
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
        // Validate the capability token before dispatching
        if !self.capability_checker.verify(token) {
            return Err(TemplateError::CapabilityDenied(format!(
                "Invalid capability token for tool: {}",
                tool_name
            )));
        }

        // Check token expiry
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        if token.is_expired(current_time) {
            return Err(TemplateError::CapabilityDenied(format!(
                "Expired capability token for tool: {}",
                tool_name
            )));
        }

        // Verify the token authorizes this tool execution
        if !token.is_valid_for(
            hkask_types::CapabilityResource::Tool,
            tool_name,
            hkask_types::CapabilityAction::Execute,
        ) {
            return Err(TemplateError::CapabilityDenied(format!(
                "Capability token does not authorize tool: {}",
                tool_name
            )));
        }

        // Security gateway: input size validation
        self.security_gateway
            .as_ref()
            .map_or(Ok(()), |sg| sg.validate_input_size(&input))?;

        // Security gateway: tool allow/deny check
        if !self
            .security_gateway
            .as_ref()
            .map_or(true, |sg| sg.is_tool_allowed(tool_name))
        {
            return Err(TemplateError::CapabilityDenied(format!(
                "Tool denied by security policy: {}",
                tool_name
            )));
        }

        // Security gateway: rate limiting (energy budget enforcement)
        if !self
            .security_gateway
            .as_ref()
            .map_or(true, |sg| sg.check_rate_limit(&token.holder()))
        {
            return Err(TemplateError::RateLimitExceeded(format!(
                "Rate limit exceeded for tool: {}",
                tool_name
            )));
        }

        let tool_info = self
            .runtime
            .get_tool_info(tool_name)
            .await
            .ok_or_else(|| TemplateError::Mcp(format!("Tool not found: {}", tool_name)))?;

        self.runtime
            .call_tool(&tool_info.server_id, tool_name, input)
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
                rate_limit_hint: t.rate_limit_hint,
            })
    }
}

impl McpDispatcher {
    /// Invoke a tool with capability checking
    pub async fn invoke_async(
        &self,
        bot_id: &WebID,
        tool_name: &str,
        input: Value,
    ) -> Result<Value> {
        // Check capability
        if !self.check_capability(bot_id, tool_name).await {
            tracing::debug!(
                target: "cns.tool.access_denied",
                bot_id = %bot_id,
                tool_name,
                "Capability denied"
            );
            return Err(TemplateError::CapabilityDenied(format!(
                "Bot {:?} lacks capability for tool: {}",
                bot_id, tool_name
            )));
        }

        // Check if tool exists
        if !self.runtime.tool_exists(tool_name).await {
            tracing::debug!(
                target: "cns.tool.not_found",
                tool_name,
                "Tool not found"
            );
            return Err(TemplateError::Mcp(format!("Tool not found: {}", tool_name)));
        }

        tracing::debug!(
            target: "cns.tool.invoked",
            bot_id = %bot_id,
            tool_name,
            "Dispatching tool call"
        );

        info!(
            target: "hkask.mcp",
            bot_id = ?bot_id,
            tool_name = %tool_name,
            "Dispatching tool call"
        );

        let tool_info = self
            .runtime
            .get_tool_info(tool_name)
            .await
            .ok_or_else(|| TemplateError::Mcp(format!("Tool info not found: {}", tool_name)))?;

        let result = self
            .runtime
            .call_tool(&tool_info.server_id, tool_name, input)
            .await
            .map_err(|e| TemplateError::Mcp(format!("Tool call failed: {}", e)))?;

        tracing::debug!(
            target: "cns.tool.completed",
            tool_name,
            "Tool call completed"
        );

        Ok(result)
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
}
