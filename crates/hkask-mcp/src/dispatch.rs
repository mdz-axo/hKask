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

use crate::raw_tool_port::RawMcpToolPort;
use crate::runtime::McpRuntime;
use hkask_cns::GovernedTool;
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use hkask_types::{CapabilityChecker, DelegationToken, WebID};
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
    /// Capability checker for token issuance.
    /// Not used for invocation governance — that flows through GovernedTool.
    capability_checker: Arc<CapabilityChecker>,
    /// Governed tool membrane — the singular governance boundary.
    /// When present, all tool invocations route through this membrane
    /// which handles OCAP verification, energy budgets, and CNS observability.
    governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>,
}

impl McpDispatcher {
    /// Create a dispatcher with a GovernedTool membrane.
    ///
    /// All tool invocations route through the membrane, which handles
    /// OCAP verification, energy budgets, and CNS observability.
    /// The membrane IS the security property.
    pub fn with_governed_tool(
        runtime: McpRuntime,
        secret: &[u8],
        governed_tool: Arc<GovernedTool<RawMcpToolPort>>,
    ) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            governed_tool: Some(governed_tool),
        }
    }

    /// Issue capability token to a bot.
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        self.capability_checker.grant_tool(tool_name, from, to)
    }

    /// Shut down all managed MCP server processes.
    ///
    /// Call this when the dispatcher is no longer needed to clean up
    /// child processes spawned via `McpRuntime::start_server()`.
    pub async fn shutdown_all(&self) {
        self.runtime.shutdown_all().await;
    }
}

impl McpPort for McpDispatcher {
    async fn discover_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
    }

    async fn invoke(
        &self,
        tool_name: &str,
        input: Value,
        token: &DelegationToken,
    ) -> Result<Value> {
        let governed = self.governed_tool.clone();
        let runtime = self.runtime.clone();
        if let Some(governed) = governed {
            // Route through GovernedTool membrane
            let server_id = runtime
                .get_tool_info(tool_name)
                .await
                .map(|t| t.server_id)
                .unwrap_or_else(|| "unknown".to_string());

            governed
                .invoke(&server_id, tool_name, input, token)
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

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.runtime.get_tool_info(tool_name).await
    }
}
