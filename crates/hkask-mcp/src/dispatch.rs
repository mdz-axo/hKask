//! MCP dispatch with capability-based security
//!
//! Dispatches tool calls through MCP with OCAP capability verification
//! and rate limiting integration.

use hkask_agents::{BotCapabilities, CapabilityChecker, CapabilityToken};
use hkask_cns::{CnsPort, RateLimiter};
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::WebID;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::runtime::{McpRuntime, McpTool};

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
}

impl McpDispatcher {
    /// Create new MCP dispatcher
    pub fn new(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            rate_limiter: RateLimiter::default(),
            bot_capabilities: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
    pub fn issue_capability(
        &self,
        tool_name: String,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
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

    fn invoke(&self, tool_name: &str, input: Value) -> Result<Value> {
        // Synchronous invoke - for async, use invoke_async
        warn!(
            target: "hkask.mcp",
            tool_name = %tool_name,
            "Synchronous invoke not supported — use invoke_async"
        );

        Err(TemplateError::Mcp(
            "Use invoke_async for MCP tool invocation".to_string()
        ))
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

        // TODO: Actual MCP tool invocation via rmcp
        // For now, return placeholder
        Ok(Value::String(format!("Tool {} invoked (placeholder)", tool_name)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_cns::spans::SpanEmitter;

    struct MockCns;
    impl CnsPort for MockCns {
        fn emit(&self, _span: &str, _outcome: Value, _confidence: f64) {
            // Mock implementation
        }
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_new() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        assert_eq!(dispatcher.list_tools().await.len(), 0);
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_capability_check() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let bot_id = WebID::new();
        let caps = BotCapabilities::new(bot_id)
            .with_capabilities(vec!["inference:call", "storage:read"]);

        dispatcher.register_bot_capabilities(caps).await;

        assert!(dispatcher.check_capability(&bot_id, "inference:call").await);
        assert!(dispatcher.check_capability(&bot_id, "storage:read").await);
        assert!(!dispatcher.check_capability(&bot_id, "memory:write").await);
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_invoke_capability_denied() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let bot_id = WebID::new();
        let caps = BotCapabilities::new(bot_id)
            .with_capabilities(vec!["inference:call"]);

        dispatcher.register_bot_capabilities(caps).await;

        let result = dispatcher.invoke_async(&bot_id, "memory:write", Value::Null, &MockCns).await;
        
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("CapabilityDenied"));
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_invoke_tool_not_found() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let bot_id = WebID::new();
        let caps = BotCapabilities::new(bot_id)
            .with_capabilities(vec!["nonexistent:tool"]);

        dispatcher.register_bot_capabilities(caps).await;

        let result = dispatcher.invoke_async(&bot_id, "nonexistent:tool", Value::Null, &MockCns).await;
        
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("not found"));
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_rate_limit() {
        use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};
        
        let runtime = McpRuntime::new();
        let mut dispatcher = McpDispatcher::new(runtime, b"test-secret");
        
        // Set very low rate limit for testing
        let bot_id = WebID::new();
        dispatcher.rate_limiter.configure_bot(&bot_id, RateLimitConfig {
            max_tokens: 2,
            refill_interval: std::time::Duration::from_secs(60),
        });

        let caps = BotCapabilities::new(bot_id)
            .with_capabilities(vec!["test:tool"]);
        dispatcher.register_bot_capabilities(caps).await;

        // Register a tool
        use crate::runtime::McpServer;
        dispatcher.runtime().register_server(McpServer {
            id: "test".to_string(),
            name: "Test".to_string(),
            tools: vec![crate::runtime::McpTool {
                name: "test:tool".to_string(),
                description: "Test".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
                server_id: "test".to_string(),
            }],
            connected: true,
        }).await;

        // First two calls should succeed
        assert!(dispatcher.check_rate_limit(&bot_id));
        dispatcher.check_rate_limit(&bot_id); // consume 1
        assert!(dispatcher.check_rate_limit(&bot_id)); // consume 1
        
        // Third call should fail (rate limited)
        assert!(!dispatcher.check_rate_limit(&bot_id));
    }

    #[tokio::test]
    async fn test_mcp_dispatcher_issue_capability() {
        let runtime = McpRuntime::new();
        let dispatcher = McpDispatcher::new(runtime, b"test-secret");

        let from = WebID::new();
        let to = WebID::new();

        let token = dispatcher.issue_capability("inference:call".to_string(), from, to);

        assert_eq!(token.tool_name, "inference:call");
        assert!(dispatcher.capability_checker.verify(&token));
    }
}