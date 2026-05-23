//! MCP dispatch with capability-based security
//!
//! Dispatches tool calls through MCP with OCAP capability verification
//! and rate limiting integration.

use hkask_agents::{BotCapabilities, CapabilityChecker, CapabilityToken};
use hkask_cns::RateLimiter;
use hkask_templates::{CnsPort, McpPort, Result, TemplateError};
use hkask_types::WebID;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

use crate::runtime::{McpRuntime, McpTool};

/// Retry configuration for transient errors
#[derive(Debug, Clone)]
pub struct McpMcpRetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub backoff_base: Duration,
    /// Retryable error codes (HTTP-style)
    pub retryable_status: Vec<u16>,
}

impl Default for McpMcpRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_base: Duration::from_millis(500),
            retryable_status: vec![503, 408, 429], // Service unavailable, timeout, rate limit
        }
    }
}

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
    retry_config: McpMcpRetryConfig,
    ) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            rate_limiter: RateLimiter::default(),
            bot_capabilities: Arc::new(RwLock::new(std::collections::HashMap::new())),
            retry_config,
        }
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
        self.capability_checker.grant(tool_name, from, to)
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

impl McpPort for McpDispatcher {
    fn discover_tools(&self) -> Vec<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.runtime.discover_tools())
        })
    }

    fn invoke(&self, tool_name: &str, input: Value) -> Result<Value> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let tool_info =
                    self.runtime.get_tool_info(tool_name).await.ok_or_else(|| {
                        TemplateError::Mcp(format!("Tool not found: {}", tool_name))
                    })?;

                self.runtime
                    .call_tool(&tool_info.server_id, tool_name, input)
                    .await
                    .map_err(|e| TemplateError::Mcp(format!("Tool call failed: {}", e)))
            })
        })
    }

    fn get_tool_info(&self, tool_name: &str) -> Option<hkask_templates::ports::ToolInfo> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.runtime.get_tool_info(tool_name).await.map(|t| {
                    hkask_templates::ports::ToolInfo {
                        name: t.name,
                        description: t.description,
                        input_schema: t.input_schema,
                        server_id: t.server_id,
                        required_capability: t.required_capability,
                        rate_limit_hint: t.rate_limit_hint,
                    }
                })
            })
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
            return Err(TemplateError::Mcp(format!("Tool not found: {}", tool_name)));
        }

        // Emit CNS event for tool invocation
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
            .map_err(|e| TemplateError::Mcp(format!("Tool call failed: {}", e)))?;

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
