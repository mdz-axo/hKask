//! MCP dispatch with capability-based security
//!
//! Dispatches tool calls through MCP with OCAP capability verification
//! and rate limiting integration.

use hkask_cns::{CnsEmit, RateLimiter};
use hkask_templates::{CnsPort, McpPort, Result, TemplateError};
use hkask_types::{BotCapabilities, CapabilityChecker, CapabilityToken, WebID};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::security::SecurityGateway;

use crate::runtime::{McpRuntime, McpTool};

/// Retry configuration — alias for the canonical RetryConfig
pub type McpMcpRetryConfig = hkask_types::cns::RetryConfig;

/// MCP dispatcher with security and rate limiting
pub struct McpDispatcher {
    /// MCP runtime for tool discovery
    runtime: McpRuntime,
    /// Capability checker for OCP
    capability_checker: Arc<CapabilityChecker>,
    /// Rate limiter for DoS prevention
    rate_limiter: RateLimiter,
    /// Bot capabilities registry
    bot_capabilities: Arc<RwLock<std::collections::HashMap<WebID, BotCapabilities>>>,
    /// Retry configuration (future: use in invoke_async)
    _retry_config: McpMcpRetryConfig,
    /// Optional CNS emitter for structured span emission
    cns_emitter: Option<Arc<dyn CnsEmit + Send + Sync>>,
    /// Optional security gateway for input validation, tool allow/deny, rate limiting
    security_gateway: Option<SecurityGateway>,
}

impl McpDispatcher {
    pub fn new(runtime: McpRuntime, secret: &[u8], retry_config: McpMcpRetryConfig) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            rate_limiter: RateLimiter::default(),
            bot_capabilities: Arc::new(RwLock::new(std::collections::HashMap::new())),
            _retry_config: retry_config,
            cns_emitter: None,
            security_gateway: None,
        }
    }

    /// Set the CNS emitter for structured span emission
    pub fn with_cns_emitter(mut self, emitter: Arc<dyn CnsEmit + Send + Sync>) -> Self {
        self.cns_emitter = Some(emitter);
        self
    }

    /// Wire in the security gateway for input validation, tool allow/deny, and rate limiting
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

    /// Check rate limit for bot
    pub fn check_rate_limit(&self, bot_id: &WebID) -> bool {
        self.rate_limiter.check(bot_id)
    }

    /// Get remaining rate limit tokens for bot
    pub fn remaining_rate_limit(&self, bot_id: &WebID) -> u32 {
        self.rate_limiter.remaining(bot_id)
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

        // Security gateway: rate limiting
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
    /// Invoke a tool with capability and rate limit checking
    pub async fn invoke_async(
        &self,
        bot_id: &WebID,
        tool_name: &str,
        input: Value,
        cns: &impl CnsPort,
    ) -> Result<Value> {
        // Check rate limit first
        if !self.check_rate_limit(bot_id) {
            if let Some(ref emitter) = self.cns_emitter {
                emitter.emit_event(
                    "cns.tool.rate_limit_exceeded",
                    "observe",
                    &serde_json::json!({"bot_id": bot_id.to_string(), "tool": tool_name}),
                    0.0,
                );
            }
            cns.emit(
                "cns.tool.rate_limit_exceeded",
                Value::String(format!("Rate limit exceeded for tool: {}", tool_name)),
                1.0,
            );

            return Err(TemplateError::RateLimitExceeded(format!(
                "Rate limit exceeded for bot: {:?}",
                bot_id
            )));
        }

        // Check capability
        if !self.check_capability(bot_id, tool_name).await {
            if let Some(ref emitter) = self.cns_emitter {
                emitter.emit_event(
                    &format!("cns.tool.{}.unauthorized", tool_name.replace(':', ".")),
                    "observe",
                    &serde_json::json!({"bot_id": bot_id.to_string(), "tool": tool_name}),
                    0.0,
                );
            }
            cns.emit(
                "cns.tool.access_denied",
                Value::String(format!("Capability denied for tool: {}", tool_name)),
                1.0,
            );

            return Err(TemplateError::CapabilityDenied(format!(
                "Bot {:?} lacks capability for tool: {}",
                bot_id, tool_name
            )));
        }

        // Check if tool exists
        if !self.runtime.tool_exists(tool_name).await {
            if let Some(ref emitter) = self.cns_emitter {
                emitter.emit(
                    &format!("cns.tool.{}.not_found", tool_name.replace(':', ".")),
                    serde_json::json!({"tool": tool_name}),
                    0.0,
                );
            }
            return Err(TemplateError::Mcp(format!("Tool not found: {}", tool_name)));
        }

        // Emit CNS event for tool invocation (Observe phase)
        if let Some(ref emitter) = self.cns_emitter {
            emitter.emit_event(
                &format!("cns.tool.{}.invoked", tool_name.replace(':', ".")),
                "observe",
                &serde_json::json!({"bot_id": bot_id.to_string(), "tool": tool_name, "input": input}),
                1.0,
            );
        }
        cns.emit(
            &format!("cns.tool.{}", tool_name.replace(':', ".")),
            input.clone(),
            1.0,
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
            .map_err(|e| {
                if let Some(ref emitter) = self.cns_emitter {
                    emitter.emit_event(
                        &format!("cns.tool.{}.failed", tool_name.replace(':', ".")),
                        "outcome",
                        &serde_json::json!({"tool": tool_name, "error": e.to_string()}),
                        0.0,
                    );
                }
                TemplateError::Mcp(format!("Tool call failed: {}", e))
            })?;

        // Emit CNS event for tool completion (Outcome phase)
        if let Some(ref emitter) = self.cns_emitter {
            emitter.emit_event(
                &format!("cns.tool.{}.completed", tool_name.replace(':', ".")),
                "outcome",
                &serde_json::json!({"tool": tool_name}),
                1.0,
            );
        }
        cns.emit(
            &format!("cns.tool.{}.result", tool_name.replace(':', ".")),
            result.clone(),
            1.0,
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
