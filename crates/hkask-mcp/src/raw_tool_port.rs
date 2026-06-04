//! Raw MCP tool port — ungoverned executor for tool invocation.
//!
//! Implements `ToolPort` by delegating to `McpRuntime` for discovery
//! and providing a passthrough invocation path. This is the "raw"
//! (ungoverned) tool port that `GovernedTool` wraps — all governance
//! (OCAP verification, energy budgets, CNS observability) is handled
//! by the `GovernedTool` membrane, not here.
//!
//! **Never expose this port directly to agents.** Always wrap it
//! with `GovernedTool` before wiring into `McpDispatcher`.

use crate::runtime::McpRuntime;
use hkask_types::DelegationToken;
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use serde_json::Value;
use tracing::{debug, instrument};

/// Raw (ungoverned) MCP tool port.
///
/// Wraps an `McpRuntime` and delegates tool discovery and invocation
/// without any governance checks. This is the inner tool port that
/// `GovernedTool` wraps — governance is the membrane's responsibility.
///
/// # Security
///
/// This port performs NO capability verification, NO energy budget
/// checks, and NO CNS observability. It exists solely as the
/// "bare metal" executor inside the `GovernedTool` membrane.
pub struct RawMcpToolPort {
    runtime: McpRuntime,
}

impl RawMcpToolPort {
    /// Create a new raw tool port wrapping the given MCP runtime.
    pub fn new(runtime: McpRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait::async_trait]
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
            "Raw tool invocation (no governance)"
        );

        // Verify tool exists in the runtime registry
        if !self.runtime.tool_exists(tool).await {
            return Err(ToolPortError::NotFound(format!(
                "Tool '{}' not found in MCP runtime",
                tool
            )));
        }

        // The raw port does not perform OCAP verification or energy checks.
        // GovernedTool handles all governance before delegating here.
        // For now, the token is accepted as-is (already verified by the membrane).

        // Invoke the tool through the MCP runtime.
        // McpRuntime currently manages metadata but the actual invocation
        // path goes through registered MCP servers via transport.
        // Return tool info + args as the invocation record for now;
        // real MCP transport invocation will be wired when client-side
        // rmcp is integrated.
        let _ = (server, token);

        Ok(serde_json::json!({
            "tool": tool,
            "args": args,
            "status": "invoked",
        }))
    }

    async fn discover_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.runtime
            .get_tool_info(tool_name)
            .await
            .map(|t| ToolInfo {
                name: t.name,
                description: t.description,
                input_schema: t.input_schema,
                server_id: t.server_id,
                required_capability: t.required_capability,
            })
    }
}
