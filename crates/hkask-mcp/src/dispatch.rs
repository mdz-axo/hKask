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
use hkask_cns::GovernedTool;
use hkask_ports::{ToolInfo, ToolPort, ToolPortError};
use hkask_templates::{McpPort, Result, TemplateError};
use hkask_types::WebID;
use rmcp::model::RawContent;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

// ── RawMcpToolPort (inlined from raw_tool_port.rs) ──

/// Raw (ungoverned) MCP tool port.
///
/// Wraps an `McpRuntime` and routes tool invocations through live
/// MCP server connections. Governance (OCAP, energy, CNS) is handled
/// by the `GovernedTool` membrane that wraps this port.
///
/// **Never expose this port directly to agents.** Always wrap it
/// with `GovernedTool` before wiring into `McpDispatcher`.
pub struct RawMcpToolPort {
    runtime: McpRuntime,
}

impl RawMcpToolPort {
    /// Create a new raw tool port wrapping the given MCP runtime.
    ///
    /// pre:  runtime is initialized
    /// post: returns RawMcpToolPort
    pub fn new(runtime: McpRuntime) -> Self {
        Self { runtime }
    }
}

impl ToolPort for RawMcpToolPort {
    #[instrument(skip(self, args, token), fields(tool = %tool, server = %server))]
    async fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: Value,
        token: &DelegationToken,
    ) -> std::result::Result<Value, ToolPortError> {
        debug!(
            target: "hkask.mcp.raw_tool_port",
            tool = %tool,
            server = %server,
            "Tool invocation via MCP transport"
        );

        let _ = token;

        // Try the live connection first
        if self.runtime.get_peer(server).await.is_some() {
            let arguments = args.as_object().cloned().unwrap_or_default();
            let result = self
                .runtime
                .call_tool(server, tool, arguments)
                .await
                .map_err(|e| ToolPortError::InvocationFailed(e.to_string()))?;

            // Check for error flag in the result
            if result.is_error.unwrap_or(false) {
                let msg = extract_text_content(&result);
                return Err(ToolPortError::InvocationFailed(msg));
            }

            return Ok(parse_call_result(&result));
        }

        // No live connection — is the tool at least registered?
        if !self.runtime.tool_exists(tool).await {
            return Err(ToolPortError::NotFound(format!(
                "Tool '{}' not found in MCP runtime",
                tool
            )));
        }

        // Tool is registered but server has no live connection
        warn!(
            target: "hkask.mcp.raw_tool_port",
            tool = %tool,
            server = %server,
            "Server registered but not connected — start it with McpRuntime::start_server()"
        );
        Err(ToolPortError::InvocationFailed(format!(
            "Server '{}' is registered but not connected — call McpRuntime::start_server() first",
            server
        )))
    }

    async fn discover_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.runtime.get_tool_info(tool_name).await
    }
}

/// Extract concatenated text from a CallToolResult's content items.
fn extract_text_content(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &**c {
            RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a CallToolResult into a JSON Value.
///
/// For a single text content item, tries to parse as JSON first
/// (structured tool responses like web_extract return JSON strings).
/// Falls back to a plain JSON string if parsing fails.
/// For multiple items, wraps them in a JSON array.
fn parse_call_result(result: &rmcp::model::CallToolResult) -> Value {
    if result.content.is_empty() {
        return Value::Null;
    }

    if result.content.len() == 1
        && let RawContent::Text(text_content) = &*result.content[0]
    {
        // Structured tool responses often return JSON as text
        if let Ok(v) = serde_json::from_str::<Value>(&text_content.text) {
            return v;
        }
        return Value::String(text_content.text.clone());
    }

    // Multiple content items — wrap in array
    let items: Vec<Value> = result
        .content
        .iter()
        .map(|c| match &**c {
            RawContent::Text(t) => serde_json::from_str::<Value>(&t.text)
                .unwrap_or_else(|_| Value::String(t.text.clone())),
            RawContent::Image(i) => serde_json::json!({
                "type": "image",
                "data": i.data,
                "mimeType": i.mime_type,
            }),
            _ => Value::Null,
        })
        .collect();
    Value::Array(items)
}

// ── McpDispatcher ──

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
    ///
    /// pre:  runtime is initialized, secret is non-empty
    /// post: returns McpDispatcher with GovernedTool membrane
    pub fn with_governed_tool(
        runtime: McpRuntime,
        secret: &[u8],
        governed_tool: Arc<GovernedTool<RawMcpToolPort>>,
    ) -> Self {
        Self {
            runtime,
            capability_checker: Arc::new(CapabilityChecker::new()),
            governed_tool: Some(governed_tool),
        }
    }

    /// Issue capability token to a bot.
    ///
    /// pre:  tool_name is non-empty, from and to are valid WebIDs
    /// post: returns DelegationToken granting tool access from → to
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
        let governed = self.governed_tool.clone();
        let runtime = self.runtime.clone();
        let tool_name = tool_name.to_string();
        let token = token.clone();
        Box::pin(async move {
            if let Some(governed) = governed {
                // Route through GovernedTool membrane
                let server_id = runtime
                    .get_tool_info(&tool_name)
                    .await
                    .map(|t| t.server_id)
                    .unwrap_or_else(|| "unknown".to_string());

                governed
                    .invoke(&server_id, &tool_name, input, &token)
                    .await
                    .map_err(|e| match e {
                        ToolPortError::CapabilityDenied(msg) => {
                            TemplateError::CapabilityDenied(msg)
                        }
                        ToolPortError::EnergyBudgetExceeded(msg) => {
                            TemplateError::Mcp(Box::new(ToolPortError::EnergyBudgetExceeded(msg)))
                        }
                        ToolPortError::NotFound(msg) => {
                            TemplateError::Mcp(Box::new(ToolPortError::NotFound(msg)))
                        }
                        ToolPortError::InvocationFailed(msg) => {
                            TemplateError::Mcp(Box::new(ToolPortError::InvocationFailed(msg)))
                        }
                    })
            } else {
                Err(TemplateError::Mcp(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "GovernedTool membrane not configured — all tool invocations require governance",
                ))))
            }
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
