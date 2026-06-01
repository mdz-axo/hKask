//! MCP Transport Layer
//!
//! Defines transport abstractions for MCP server communication.
//! Supports stdio and HTTP transports.

/// MCP transport enum for tool invocation
///
/// Replaces the former `McpTransport` trait with a concrete enum,
/// eliminating dynamic dispatch and simplifying the type system.
#[derive(Debug, Clone)]
pub enum McpTransport {
    /// Stdio transport for child process servers (not yet implemented)
    Stdio,
    /// HTTP transport for remote servers (not yet implemented)
    Http,
}

impl McpTransport {
    /// Call a tool on the MCP server
    pub async fn call(
        &self,
        _server_id: &str,
        tool_name: &str,
        _arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self {
            McpTransport::Stdio => Err(format!(
                "StdioMcpTransport not yet implemented for tool '{}'",
                tool_name
            )),
            McpTransport::Http => Err(format!(
                "HttpMcpTransport not yet implemented for tool '{}'",
                tool_name
            )),
        }
    }

    /// Check if transport is connected
    pub fn is_connected(&self) -> bool {
        match self {
            McpTransport::Stdio => false,
            McpTransport::Http => false,
        }
    }
}
