//! MCP dispatch — Communication loop concerns
//!
//! Routes tool calls through MCP runtime. Governance (capability
//! verification, energy budget, observability) is delegated to `GovernedTool`
//! when available, which subsumes the former inline checks (McpGovernor
//! authorization, CnsRuntime throttle, ToolSpanGuard).
//!
//! This split enforces the authority DAG: Cybernetics governs
//! Communication. The dispatcher is the dumb transport pipe; the
//! governed tool membrane is the security property.

use crate::governor::McpGovernor;
use crate::runtime::{McpRuntime, McpTool};
use hkask_cns::CnsRuntime;
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use hkask_types::{DelegationToken, WebID};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

/// MCP dispatcher — Communication-layer tool routing.
///
/// Wraps `McpRuntime` for tool discovery and invocation.
/// When a `GovernedTool` membrane is injected, all governance concerns
/// (OCAP verification, energy budgets, CNS observability) are routed
/// through it. Otherwise, falls back to inline `McpGovernor` + `CnsRuntime`
/// checks (legacy path).
pub struct McpDispatcher {
    /// MCP runtime for tool discovery and invocation
    runtime: McpRuntime,
    /// Cybernetics governor for capability governance (legacy, used when no GovernedTool)
    governor: Arc<McpGovernor>,
    /// CNS runtime for per-agent throttling (Loop 6, legacy)
    cns: Arc<CnsRuntime>,
    /// Governed tool membrane — the singular governance boundary.
    /// When present, all tool invocations route through this membrane
    /// which handles OCAP verification, energy budgets, and CNS observability.
    governed_tool: Option<Arc<dyn ToolPort>>,
}

impl McpDispatcher {
    /// Create a dispatcher with a runtime, a secret for the capability checker,
    /// and a CNS runtime for throttling.
    pub fn new(runtime: McpRuntime, secret: &[u8], cns: Arc<CnsRuntime>) -> Self {
        Self {
            runtime,
            governor: Arc::new(McpGovernor::new(secret)),
            cns,
            governed_tool: None,
        }
    }

    /// Create a dispatcher with a default CNS runtime (convenience).
    pub fn with_default_cns(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self::new(runtime, secret, Arc::new(CnsRuntime::default()))
    }

    /// Create a dispatcher with a GovernedTool membrane.
    ///
    /// When a GovernedTool is present, all tool invocations route through it
    /// instead of the inline governor + throttle checks. The membrane IS
    /// the security property.
    pub fn with_governed_tool(
        runtime: McpRuntime,
        secret: &[u8],
        cns: Arc<CnsRuntime>,
        governed_tool: Arc<dyn ToolPort>,
    ) -> Self {
        Self {
            runtime,
            governor: Arc::new(McpGovernor::new(secret)),
            cns,
            governed_tool: Some(governed_tool),
        }
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

    /// Invoke a tool, routing through GovernedTool if available.
    ///
    /// When GovernedTool is present: OCAP verification, energy budgets,
    /// and CNS observability are handled by the membrane.
    /// When absent: falls back to inline governor + throttle checks.
    #[allow(unused_variables)]
    pub async fn invoke_async(
        &self,
        bot_id: &WebID,
        tool_name: &str,
        input: Value,
        token: Option<&DelegationToken>,
    ) -> Result<Value> {
        let token = token.ok_or_else(|| {
            TemplateError::CapabilityDenied(
                "No capability token provided; legacy authorization removed".to_string(),
            )
        })?;

        if let Some(governed) = &self.governed_tool {
            // Route through GovernedTool membrane — the membrane IS the security property
            let server_id = self
                .runtime
                .get_tool_info(tool_name)
                .await
                .map(|t| t.server_id)
                .unwrap_or_else(|| "unknown".to_string());

            governed
                .invoke(&server_id, tool_name, input, token)
                .await
                .map_err(|e| match e {
                    ToolPortError::CapabilityDenied(msg) => TemplateError::CapabilityDenied(msg),
                    ToolPortError::EnergyBudgetExceeded(msg) => {
                        TemplateError::Mcp(format!("Energy budget exceeded: {}", msg))
                    }
                    ToolPortError::NotFound(msg) => {
                        TemplateError::Mcp(format!("Tool not found: {}", msg))
                    }
                    ToolPortError::InvocationFailed(msg) => TemplateError::Mcp(msg),
                })
        } else {
            // LEGACY: Remove when all call sites route through GovernedTool.
            // The GovernedTool membrane subsumes OCAP authorization, throttle
            // checks, and NuEvent observability. This inline path exists only
            // for call sites that have not yet been wired through GovernedTool.
            self.governor
                .authorize(token, tool_name)
                .await
                .map_err(TemplateError::CapabilityDenied)?;

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

            if !self.runtime.tool_exists(tool_name).await {
                return Err(TemplateError::Mcp(format!("Tool not found: {}", tool_name)));
            }

            info!(
                target: "hkask.mcp",
                bot_id = ?bot_id,
                tool_name = %tool_name,
                token_id = token.id.as_str(),
                "Dispatching tool call"
            );

            let tool_info =
                self.runtime.get_tool_info(tool_name).await.ok_or_else(|| {
                    TemplateError::Mcp(format!("Tool info not found: {}", tool_name))
                })?;

            Err(TemplateError::Mcp(format!(
                "MCP transport not yet implemented for server '{}'",
                tool_info.server_id
            )))
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
        _input: Value,
        token: &DelegationToken,
    ) -> Result<Value> {
        if let Some(governed) = &self.governed_tool {
            // Route through GovernedTool membrane
            let server_id = self
                .runtime
                .get_tool_info(tool_name)
                .await
                .map(|t| t.server_id)
                .unwrap_or_else(|| "unknown".to_string());

            governed
                .invoke(&server_id, tool_name, serde_json::json!({}), token)
                .await
                .map_err(|e| match e {
                    ToolPortError::CapabilityDenied(msg) => TemplateError::CapabilityDenied(msg),
                    ToolPortError::EnergyBudgetExceeded(msg) => {
                        TemplateError::Mcp(format!("Energy budget exceeded: {}", msg))
                    }
                    ToolPortError::NotFound(msg) => {
                        TemplateError::Mcp(format!("Tool not found: {}", msg))
                    }
                    ToolPortError::InvocationFailed(msg) => TemplateError::Mcp(msg),
                })
        } else {
            // LEGACY: Remove when all call sites route through GovernedTool.
            self.governor
                .authorize(token, tool_name)
                .await
                .map_err(TemplateError::CapabilityDenied)?;

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

            Err(TemplateError::Mcp(format!(
                "MCP transport not yet implemented for server '{}'",
                tool_info.server_id
            )))
        }
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
