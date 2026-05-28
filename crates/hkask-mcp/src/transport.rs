//! MCP Transport Layer
//!
//! Defines transport abstractions for MCP server communication.
//! Supports in-process, stdio, and HTTP transports.

use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP transport enum for tool invocation
///
/// Replaces the former `McpTransport` trait with a concrete enum,
/// eliminating dynamic dispatch and simplifying the type system.
#[derive(Debug, Clone)]
pub enum McpTransport {
    /// In-process transport for co-located servers
    InProcess(InProcessMcpTransport),
    /// Stdio transport for child process servers (not yet implemented)
    Stdio,
    /// HTTP transport for remote servers (not yet implemented)
    Http,
}

impl McpTransport {
    /// Call a tool on the MCP server
    pub async fn call(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        match self {
            McpTransport::InProcess(inner) => inner.call(server_id, tool_name, arguments).await,
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
            McpTransport::InProcess(_) => true,
            McpTransport::Stdio => false,
            McpTransport::Http => false,
        }
    }
}

impl From<InProcessMcpTransport> for McpTransport {
    fn from(inner: InProcessMcpTransport) -> Self {
        McpTransport::InProcess(inner)
    }
}

/// In-process MCP transport for co-located servers
///
/// Allows MCP servers to be registered as in-process handlers,
/// avoiding network overhead for local tools.
type HandlerFn = Box<dyn Fn(Value) -> Result<Value, String> + Send + Sync>;

pub struct InProcessMcpTransport {
    handlers: Arc<RwLock<HashMap<String, HandlerFn>>>,
}

impl fmt::Debug for InProcessMcpTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InProcessMcpTransport")
            .field(
                "handlers_count",
                &self.handlers.try_read().map(|h| h.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl Clone for InProcessMcpTransport {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
        }
    }
}

impl InProcessMcpTransport {
    /// Create new in-process transport
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool handler
    pub async fn register_handler<F>(&self, tool_name: String, handler: F)
    where
        F: Fn(Value) -> Result<Value, String> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(tool_name, Box::new(handler));
    }

    /// Unregister a tool handler
    pub async fn unregister_handler(&self, tool_name: &str) {
        let mut handlers = self.handlers.write().await;
        handlers.remove(tool_name);
    }

    /// Call a tool handler directly
    async fn call(
        &self,
        _server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let handlers = self.handlers.read().await;
        let handler = handlers
            .get(tool_name)
            .ok_or_else(|| format!("No handler registered for tool '{}'", tool_name))?;
        handler(arguments)
    }
}

impl Default for InProcessMcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_process_transport() {
        let transport = InProcessMcpTransport::new();

        // Register a simple handler
        transport
            .register_handler("test_tool".to_string(), |args| {
                Ok(serde_json::json!({
                    "received": args,
                    "status": "ok"
                }))
            })
            .await;

        // Call the tool via the enum
        let enum_transport = McpTransport::InProcess(transport);
        let result = enum_transport
            .call(
                "test_server",
                "test_tool",
                serde_json::json!({"key": "value"}),
            )
            .await
            .unwrap();

        assert_eq!(result["status"], "ok");
        assert_eq!(result["received"]["key"], "value");
    }

    #[tokio::test]
    async fn test_in_process_transport_missing_tool() {
        let transport = InProcessMcpTransport::new();

        let enum_transport = McpTransport::InProcess(transport);
        let result = enum_transport
            .call("test_server", "nonexistent_tool", serde_json::json!({}))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No handler registered"));
    }

    #[tokio::test]
    async fn test_in_process_transport_unregister() {
        let transport = InProcessMcpTransport::new();

        transport
            .register_handler("test_tool".to_string(), |_| Ok(serde_json::json!({})))
            .await;

        transport.unregister_handler("test_tool").await;

        let enum_transport = McpTransport::InProcess(transport);
        let result = enum_transport
            .call("test_server", "test_tool", serde_json::json!({}))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stdio_transport_not_implemented() {
        let transport = McpTransport::Stdio;
        assert!(!transport.is_connected());
        let result = transport
            .call("server", "tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_http_transport_not_implemented() {
        let transport = McpTransport::Http;
        assert!(!transport.is_connected());
        let result = transport
            .call("server", "tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_from_in_process_conversion() {
        let inner = InProcessMcpTransport::new();
        let _enum_transport: McpTransport = inner.into();
    }
}
