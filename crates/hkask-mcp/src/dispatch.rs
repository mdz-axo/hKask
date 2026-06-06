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
use crate::raw_tool_port::RawMcpToolPort;
use crate::runtime::McpRuntime;
use hkask_cns::GovernedTool;
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use hkask_types::{DelegationToken, WebID};
use serde_json::Value;
use std::sync::Arc;

/// Concrete governed tool type used by the MCP dispatcher.
pub type DispatchGovernedTool = GovernedTool<RawMcpToolPort>;

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
    governed_tool: Option<Arc<DispatchGovernedTool>>,
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
        governed_tool: Arc<DispatchGovernedTool>,
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

    /// Shut down all managed MCP server processes.
    ///
    /// Call this when the dispatcher is no longer needed to clean up
    /// child processes spawned via `McpRuntime::start_server()`.
    pub async fn shutdown_all(&self) {
        self.runtime.shutdown_all().await;
    }
}

impl McpPort for McpDispatcher {
    fn discover_tools(&self) -> impl std::future::Future<Output = Vec<String>> + Send {
        async move { self.runtime.discover_tools().await }
    }

    fn invoke(
        &self,
        tool_name: &str,
        input: Value,
        token: &DelegationToken,
    ) -> impl std::future::Future<Output = Result<Value>> + Send {
        let governed = self.governed_tool.clone();
        let runtime = self.runtime.clone();
        async move {
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
                        ToolPortError::CapabilityDenied(msg) => {
                            TemplateError::CapabilityDenied(msg)
                        }
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
    }

    fn get_tool_info(
        &self,
        tool_name: &str,
    ) -> impl std::future::Future<Output = Option<ToolInfo>> + Send {
        async move { self.runtime.get_tool_info(tool_name).await }
    }
}
