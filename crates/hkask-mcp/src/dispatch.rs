//! MCP dispatch — Communication loop concerns
//!
//! Routes tool calls through MCP runtime. Governance (capability
//! verification, energy budget, observability) is delegated to `GovernedTool`
//! which subsumes the former inline checks (McpGovernor authorization,
//! CnsRuntime throttle, ToolSpanGuard).
//!
//! This split enforces the authority DAG: Cybernetics governs
//! Communication. The dispatcher is the dumb transport pipe; the
//! governed tool membrane is the security property.
//!
//! All invocations require a GovernedTool membrane. The legacy inline
//! path (McpGovernor.authorize) has been removed — call sites must wire
//! through GovernedTool.

use crate::governor::McpGovernor;
use crate::runtime::McpRuntime;
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::ports::{ToolPort, ToolPortError};
use hkask_types::{DelegationToken, WebID};
use serde_json::Value;
use std::sync::Arc;

/// MCP dispatcher — Communication-layer tool routing.
///
/// Wraps `McpRuntime` for tool discovery and invocation.
/// All governance concerns (OCAP verification, energy budgets, CNS
/// observability) are routed through the `GovernedTool` membrane.
pub struct McpDispatcher {
    /// MCP runtime for tool discovery and invocation
    runtime: McpRuntime,
    /// Capability governor for token issuance (`issue_capability`).
    /// Not used for invocation governance — that flows through GovernedTool.
    governor: Arc<McpGovernor>,
    /// Governed tool membrane — the singular governance boundary.
    /// When present, all tool invocations route through this membrane
    /// which handles OCAP verification, energy budgets, and CNS observability.
    governed_tool: Option<Arc<dyn ToolPort>>,
}

impl McpDispatcher {
    /// Create a dispatcher with a runtime and a secret for the capability checker.
    ///
    /// The dispatcher will have no GovernedTool membrane — any invocation
    /// attempt will return an error. Use `with_governed_tool()` for a
    /// working dispatcher.
    pub fn new(runtime: McpRuntime, secret: &[u8]) -> Self {
        Self {
            runtime,
            governor: Arc::new(McpGovernor::new(secret)),
            governed_tool: None,
        }
    }

    /// Create a dispatcher with a GovernedTool membrane.
    ///
    /// All tool invocations route through the membrane, which handles
    /// OCAP verification, energy budgets, and CNS observability.
    /// The membrane IS the security property.
    pub fn with_governed_tool(
        runtime: McpRuntime,
        secret: &[u8],
        governed_tool: Arc<dyn ToolPort>,
    ) -> Self {
        Self {
            runtime,
            governor: Arc::new(McpGovernor::new(secret)),
            governed_tool: Some(governed_tool),
        }
    }

    /// Issue capability token to a bot (delegates to governor).
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        self.governor.issue_capability(tool_name, from, to)
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
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
                    ToolPortError::GasBudgetExceeded(msg) => {
                        TemplateError::Mcp(format!("Energy budget exceeded: {}", msg))
                    }
                    ToolPortError::NotFound(msg) => {
                        TemplateError::Mcp(format!("Tool not found: {}", msg))
                    }
                    ToolPortError::InvocationFailed(msg) => TemplateError::Mcp(msg),
                })
        } else {
            Err(TemplateError::Mcp(
                "GovernedTool membrane not configured — all tool invocations require governance"
                    .to_string(),
            ))
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
