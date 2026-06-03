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
use hkask_cns::CnsRuntime;
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::{DelegationToken, WebID};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

/// MCP dispatcher — Communication-layer tool routing.
///
/// Wraps `McpRuntime` for tool discovery and invocation.
/// All governance checks are delegated to `McpGovernor`.
/// Throttling is delegated to `CnsRuntime` (Loop 6 regulation).
pub struct McpDispatcher {
    /// MCP runtime for tool discovery and invocation
    runtime: McpRuntime,
    /// Cybernetics governor for capability governance
    governor: Arc<McpGovernor>,
    /// CNS runtime for per-agent throttling (Loop 6)
    cns: Arc<CnsRuntime>,
}

impl McpDispatcher {
    /// Create a dispatcher with a runtime, a secret for the capability checker,
    /// and a CNS runtime for throttling.
    pub fn new(runtime: McpRuntime, secret: &[u8], cns: Arc<CnsRuntime>) -> Self {
        Self {
            runtime,
            governor: Arc::new(McpGovernor::new(secret)),
            cns,
        }
    }

    /// Create a dispatcher with a default CNS runtime (convenience).
    pub fn with_default_cns(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self::new(runtime, secret, Arc::new(CnsRuntime::default()))
    }

    /// Issue capability token to a bot (delegates to governor).
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
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
    /// Requires a `DelegationToken` for authorization. Returns an error
    /// if no token is provided (legacy authorization removed in T16).
    #[allow(unused_variables)]
    pub async fn invoke_async(
        &self,
        bot_id: &WebID,
        tool_name: &str,
        input: Value,
        token: Option<&DelegationToken>,
    ) -> Result<Value> {
        // Delegate governance to the governor
        if let Some(token) = token {
            self.governor
                .authorize(token, tool_name)
                .await
                .map_err(TemplateError::CapabilityDenied)?;
        } else {
            return Err(TemplateError::CapabilityDenied(
                "No capability token provided; legacy authorization removed".to_string(),
            ));
        }

        // CNS throttle check (Loop 6 — per-agent rate limiting)
        if !self.cns.check_throttle(bot_id).await {
            warn!(
                target: "hkask.mcp.dispatch",
                bot_id = ?bot_id,
                tool_name = %tool_name,
                "CNS throttle: rate-limited"
            );
            return Err(TemplateError::Mcp(format!(
                "Rate limit exceeded for agent: {}",
                bot_id
            )));
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

        // Transport not yet implemented — see T16
        Err(TemplateError::Mcp(format!(
            "MCP transport not yet implemented for server '{}'",
            tool_info.server_id
        )))
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
        _input: Value,
        token: &DelegationToken,
    ) -> Result<Value> {
        // Delegate governance to the governor
        self.governor
            .authorize(token, tool_name)
            .await
            .map_err(TemplateError::CapabilityDenied)?;

        // Extract the holder WebID from the token for throttle check
        let holder_id = token.holder();
        if !self.cns.check_throttle(&holder_id).await {
            warn!(
                target: "hkask.mcp.dispatch",
                holder_id = ?holder_id,
                tool_name = %tool_name,
                "CNS throttle: rate-limited"
            );
            return Err(TemplateError::Mcp(format!(
                "Rate limit exceeded for agent: {}",
                holder_id
            )));
        }

        let tool_info = self
            .runtime
            .get_tool_info(tool_name)
            .await
            .ok_or_else(|| TemplateError::Mcp(format!("Tool not found: {}", tool_name)))?;

        // Transport not yet implemented — see T16
        Err(TemplateError::Mcp(format!(
            "MCP transport not yet implemented for server '{}'",
            tool_info.server_id
        )))
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
