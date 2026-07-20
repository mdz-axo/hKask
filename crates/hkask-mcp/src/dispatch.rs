//! MCP dispatch — Communication loop concerns
//!
//! Routes tool calls through MCP runtime. Governance (capability
//! verification, energy budget, observability) is delegated to `GovernedTool`
//! which handles authorization, throttling, and span guarding.
//!
//! This split enforces the authority DAG: Cybernetics governs
//! Communication. The dispatcher is the transport pipe; the
//! governed tool membrane is the security property.
//!
//! All invocations require a GovernedTool membrane.

use crate::runtime::McpRuntime;
use hkask_capability::{CapabilityChecker, DelegationToken};
use hkask_ports::{ToolInfo, ToolPort, ToolPortError};
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::WebID;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::warn;

// ── McpDispatcher ──

/// MCP dispatcher — Communication-layer tool routing.
///
/// Wraps `McpRuntime` for tool discovery and invocation.
/// The runtime itself handles OCAP/gas/CNS — no separate membrane needed.
pub struct McpDispatcher {
    runtime: McpRuntime,
    capability_checker: Arc<CapabilityChecker>,
}

impl McpDispatcher {
    /// Create a dispatcher with a governed McpRuntime.
    #[must_use]
    pub fn with_governed_tool(runtime: McpRuntime, _governed_tool: Arc<McpRuntime>) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new()),
        }
    }

    /// Create a dispatcher with a governed McpRuntime and capability checker.
    #[must_use]
    pub fn with_governed_tool_and_checker(
        runtime: McpRuntime,
        _governed_tool: Arc<McpRuntime>,
        capability_checker: Arc<CapabilityChecker>,
    ) -> Self {
        Self {
            runtime,
            capability_checker,
        }
    }

    /// Issue capability token to a bot.
    ///
    /// pre:  tool_name is non-empty, from and to are valid WebIDs
    /// post: returns DelegationToken granting tool access from → to
    #[must_use]
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
    fn discover_tools(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send>> {
        let runtime = self.runtime.clone();
        Box::pin(async move { runtime.discover_tools().await })
    }

    fn invoke(
        &self,
        tool_name: &str,
        input: Value,
        token: &DelegationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> {
        let runtime = self.runtime.clone();
        let tool_name = tool_name.to_string();
        let token = token.clone();
        Box::pin(async move {
            let server_id = runtime
                .get_tool_info(&tool_name)
                .await
                .map(|t| t.server_id)
                .unwrap_or_else(|| "unknown".to_string());
            runtime
                .invoke(&server_id, &tool_name, input, &token)
                .await
                .map_err(|e| match e {
                    ToolPortError::CapabilityDenied(msg) => TemplateError::CapabilityDenied(msg),
                    other => TemplateError::Mcp(Box::new(other)),
                })
        })
    }

    fn get_tool_info(
        &self,
        tool_name: &str,
    ) -> Pin<Box<dyn Future<Output = Option<ToolInfo>> + Send>> {
        let runtime = self.runtime.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move { runtime.get_tool_info(&tool_name).await })
    }
}
