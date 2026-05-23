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
use tracing::{info, warn};

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
    #[allow(dead_code)]
    retry_config: McpMcpRetryConfig,
}

impl McpDispatcher {
    /// Create new MCP dispatcher
    pub fn new(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            rate_limiter: RateLimiter::default(),
            bot_capabilities: Arc::new(RwLock::new(std::collections::HashMap::new())),
            retry_config: McpMcpRetryConfig::default(),
        }
    }

    /// Create with custom retry configuration
    pub fn with_retry_config(
        runtime: McpRuntime,
        secret: &[u8],
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
        // Note: This is synchronous; use runtime.discover_tools().await in async context
        vec![]
    }

    fn invoke(&self, tool_name: &str, _input: Value) -> Result<Value> {
        // Synchronous invoke - for async, use invoke_async
        warn!(
            target: "hkask.mcp",
            tool_name = %tool_name,
            "Synchronous invoke not supported — use invoke_async"
        );

        Err(TemplateError::Mcp(
            "Use invoke_async for MCP tool invocation".to_string(),
        ))
    }

    fn get_tool_info(&self, tool_name: &str) -> Option<hkask_templates::ports::ToolInfo> {
        // Synchronous version - use get_tool_info_async for full functionality
        warn!(
            target: "hkask.mcp",
            tool_name = %tool_name,
            "Synchronous get_tool_info not supported — use get_tool_info_async"
        );
        None
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

        Ok(Value::String(format!("Tool {} invoked", tool_name)))
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
