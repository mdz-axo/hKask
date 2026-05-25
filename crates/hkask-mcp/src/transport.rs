//! MCP Transport Layer
//!
//! Defines transport abstractions for MCP server communication.
//! Supports in-process, stdio, and HTTP transports.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP transport trait for tool invocation
#[async_trait]
pub trait McpTransport: Send + Sync + fmt::Debug {
    /// Call a tool on the MCP server
    async fn call(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String>;

    /// Check if transport is connected
    fn is_connected(&self) -> bool;
}

/// In-process MCP transport for co-located servers
///
/// Allows MCP servers to be registered as in-process handlers,
/// avoiding network overhead for local tools.
pub struct InProcessMcpTransport {
    handlers:
        Arc<RwLock<HashMap<String, Box<dyn Fn(Value) -> Result<Value, String> + Send + Sync>>>>,
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
}

impl Default for InProcessMcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpTransport for InProcessMcpTransport {
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

    fn is_connected(&self) -> bool {
        true
    }
}

/// Stdio MCP transport for child process servers
///
/// Communicates with MCP servers spawned as child processes
/// using JSON-RPC over stdin/stdout.
pub struct StdioMcpTransport {
    // TODO: Implement stdio transport with child process management
    // This requires:
    // - Spawning child process
    // - JSON-RPC framing over stdin/stdout
    // - Process lifecycle management
    // - Error handling and reconnection
    _placeholder: (),
}

impl fmt::Debug for StdioMcpTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdioMcpTransport")
            .field("status", &"not_implemented")
            .finish()
    }
}

impl StdioMcpTransport {
    /// Create new stdio transport (not yet implemented)
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Default for StdioMcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpTransport for StdioMcpTransport {
    async fn call(
        &self,
        _server_id: &str,
        tool_name: &str,
        _arguments: Value,
    ) -> Result<Value, String> {
        Err(format!(
            "StdioMcpTransport not yet implemented for tool '{}'",
            tool_name
        ))
    }

    fn is_connected(&self) -> bool {
        false
    }
}

/// HTTP MCP transport for remote servers
///
/// Communicates with MCP servers over HTTP using JSON-RPC.
pub struct HttpMcpTransport {
    // TODO: Implement HTTP transport
    // This requires:
    // - HTTP client (reqwest)
    // - JSON-RPC request/response handling
    // - Authentication (capability tokens)
    // - Connection pooling and timeouts
    _placeholder: (),
}

impl fmt::Debug for HttpMcpTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpMcpTransport")
            .field("status", &"not_implemented")
            .finish()
    }
}

impl HttpMcpTransport {
    /// Create new HTTP transport (not yet implemented)
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Default for HttpMcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpTransport for HttpMcpTransport {
    async fn call(
        &self,
        _server_id: &str,
        tool_name: &str,
        _arguments: Value,
    ) -> Result<Value, String> {
        Err(format!(
            "HttpMcpTransport not yet implemented for tool '{}'",
            tool_name
        ))
    }

    fn is_connected(&self) -> bool {
        false
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

        // Call the tool
        let result = transport
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

        let result = transport
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

        let result = transport
            .call("test_server", "test_tool", serde_json::json!({}))
            .await;

        assert!(result.is_err());
    }
}
