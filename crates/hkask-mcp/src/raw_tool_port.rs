//! Raw MCP tool port — ungoverned executor for tool invocation.
//!
//! Implements `ToolPort` by delegating to `McpRuntime` for discovery
//! and routing tool calls through live rmcp client connections.
//! When a server has a live connection (started via `McpRuntime::start_server`),
//! calls go through the rmcp transport. When no connection exists, returns
//! an error indicating the server needs to be started.
//!
//! **Never expose this port directly to agents.** Always wrap it
//! with `GovernedTool` before wiring into `McpDispatcher`.

use crate::runtime::McpRuntime;
use hkask_types::DelegationToken;
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use rmcp::model::RawContent;
use serde_json::Value;
use tracing::{debug, instrument, warn};

/// Raw (ungoverned) MCP tool port.
///
/// Wraps an `McpRuntime` and routes tool invocations through live
/// MCP server connections. Governance (OCAP, energy, CNS) is handled
/// by the `GovernedTool` membrane that wraps this port.
pub struct RawMcpToolPort {
    runtime: McpRuntime,
}

impl RawMcpToolPort {
    /// Create a new raw tool port wrapping the given MCP runtime.
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
    ) -> Result<Value, ToolPortError> {
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
pub fn parse_call_result(result: &rmcp::model::CallToolResult) -> Value {
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
