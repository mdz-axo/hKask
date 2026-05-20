//! MCP Server Implementations
//!
//! Built-in MCP server implementations for hKask:
//! - `std-sqlite`: SQLite storage operations
//! - `git-registry`: Git-based registry access

use crate::runtime::{McpRuntime, McpServer, McpTool};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Register built-in MCP servers with runtime
pub async fn register_builtin_servers(runtime: &McpRuntime) {
    register_sqlite_server(runtime).await;
    register_git_registry_server(runtime).await;
}

/// Register SQLite MCP server
pub async fn register_sqlite_server(runtime: &McpRuntime) {
    let server = McpServer {
        id: "hkask-mcp-storage".to_string(),
        name: "hKask Storage Server".to_string(),
        tools: vec![
            McpTool {
                name: "storage:read".to_string(),
                description: "Read data from storage".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string", "description": "Storage key"}
                    },
                    "required": ["key"]
                }),
                server_id: "hkask-mcp-storage".to_string(),
            },
            McpTool {
                name: "storage:write".to_string(),
                description: "Write data to storage".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string", "description": "Storage key"},
                        "value": {"type": "object", "description": "Value to store"}
                    },
                    "required": ["key", "value"]
                }),
                server_id: "hkask-mcp-storage".to_string(),
            },
            McpTool {
                name: "storage:delete".to_string(),
                description: "Delete data from storage".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string", "description": "Storage key"}
                    },
                    "required": ["key"]
                }),
                server_id: "hkask-mcp-storage".to_string(),
            },
            McpTool {
                name: "storage:list".to_string(),
                description: "List all storage keys".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prefix": {"type": "string", "description": "Optional key prefix filter"}
                    }
                }),
                server_id: "hkask-mcp-storage".to_string(),
            },
        ],
        connected: true,
    };

    runtime.register_server(server).await;
    info!(target: "hkask.mcp", "Registered SQLite MCP server");
}

/// Register Git Registry MCP server
pub async fn register_git_registry_server(runtime: &McpRuntime) {
    let server = McpServer {
        id: "hkask-mcp-registry".to_string(),
        name: "hKask Git Registry Server".to_string(),
        tools: vec![
            McpTool {
                name: "registry:register".to_string(),
                description: "Register a template or manifest in the git registry".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "Artifact ID"},
                        "type": {"type": "string", "description": "Artifact type (template/manifest)"},
                        "content": {"type": "string", "description": "Artifact content"}
                    },
                    "required": ["id", "type", "content"]
                }),
                server_id: "hkask-mcp-registry".to_string(),
            },
            McpTool {
                name: "registry:get".to_string(),
                description: "Get an artifact from the git registry".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "Artifact ID"}
                    },
                    "required": ["id"]
                }),
                server_id: "hkask-mcp-registry".to_string(),
            },
            McpTool {
                name: "registry:list".to_string(),
                description: "List all artifacts in the git registry".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "type": {"type": "string", "description": "Filter by artifact type"}
                    }
                }),
                server_id: "hkask-mcp-registry".to_string(),
            },
            McpTool {
                name: "registry:search".to_string(),
                description: "Search artifacts by lexicon term".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "term": {"type": "string", "description": "Lexicon term to search"}
                    },
                    "required": ["term"]
                }),
                server_id: "hkask-mcp-registry".to_string(),
            },
        ],
        connected: true,
    };

    runtime.register_server(server).await;
    info!(target: "hkask.mcp", "Registered Git Registry MCP server");
}

/// In-memory storage for SQLite MCP server
pub struct SqliteStorage {
    data: Arc<RwLock<std::collections::HashMap<String, Value>>>,
}

impl SqliteStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn read(&self, key: &str) -> Option<Value> {
        let data = self.data.read().await;
        data.get(key).cloned()
    }

    pub async fn write(&self, key: String, value: Value) {
        let mut data = self.data.write().await;
        data.insert(key, value);
    }

    pub async fn delete(&self, key: &str) {
        let mut data = self.data.write().await;
        data.remove(key);
    }

    pub async fn list(&self, prefix: Option<&str>) -> Vec<String> {
        let data = self.data.read().await;
        match prefix {
            Some(p) => data.keys().filter(|k| k.starts_with(p)).cloned().collect(),
            None => data.keys().cloned().collect(),
        }
    }
}

impl Default for SqliteStorage {
    fn default() -> Self {
        Self::new()
    }
}

