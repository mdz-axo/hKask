//! CLI commands implementation
//!
//! This module contains the actual command handlers.

use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};
use hkask_templates::{RegistryEntry, RegistryIndex, SqliteRegistry, TemplateError};
use hkask_types::TemplateType;
use serde_json::Value;
use std::path::PathBuf;

/// Template list command
pub fn list_templates(
    registry: &dyn RegistryIndex,
    template_type: Option<TemplateType>,
) -> Vec<RegistryEntry> {
    registry.list(template_type)
}

/// Register template command
pub fn register_template(
    registry: &mut SqliteRegistry,
    id: String,
    template_type: TemplateType,
    source_path: String,
    lexicon_terms: Vec<String>,
    description: String,
) -> Result<(), TemplateError> {
    let entry = RegistryEntry {
        id,
        template_type,
        lexicon_terms,
        description,
        source_path,
    };

    registry.register(entry, None)
}

/// Get template command
pub fn get_template(
    registry: &dyn RegistryIndex,
    id: &str,
) -> Result<RegistryEntry, TemplateError> {
    registry.get(id)
}

/// Search templates by lexicon
pub fn search_templates(registry: &SqliteRegistry, term: &str) -> Vec<RegistryEntry> {
    registry.search_by_lexicon(term)
}

/// List MCP servers
pub async fn list_mcp_servers(runtime: &McpRuntime) -> Vec<McpServer> {
    runtime.list_servers().await
}

/// List MCP tools
pub async fn list_mcp_tools(runtime: &McpRuntime) -> Vec<String> {
    runtime.discover_tools().await
}

/// Get MCP tool definition
pub async fn get_mcp_tool(runtime: &McpRuntime, name: &str) -> Option<Value> {
    runtime.get_tool(name).await.map(|tool| {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
            "server_id": tool.server_id,
        })
    })
}

/// Register MCP server
pub async fn register_mcp_server(
    runtime: &McpRuntime,
    id: String,
    name: String,
    tools: Vec<McpTool>,
) {
    let server = McpServer {
        id,
        name,
        tools,
        connected: true,
    };

    runtime.register_server(server).await;
}

/// Pod status information
pub struct PodStatus {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub created_at: String,
}

/// Get pod status (placeholder - requires pod manager implementation)
pub fn get_pod_status(_pod_id: &str) -> Result<PodStatus, String> {
    Err("Pod manager not yet implemented. This is a placeholder for Phase 3.".to_string())
}

/// List all pods (placeholder - requires pod manager implementation)
pub fn list_pods() -> Vec<PodStatus> {
    vec![]
}

/// Create pod from template crate (placeholder - requires pod manager implementation)
pub fn create_pod(
    _template_name: &str,
    _persona_path: &PathBuf,
    _pod_name: Option<&str>,
) -> Result<String, String> {
    Err("Pod manager not yet implemented. This is a placeholder for Phase 3.".to_string())
}

/// Activate pod (placeholder - requires pod manager implementation)
pub fn activate_pod(_pod_id: &str) -> Result<(), String> {
    Err("Pod manager not yet implemented. This is a placeholder for Phase 3.".to_string())
}

/// Deactivate pod (placeholder - requires pod manager implementation)
pub fn deactivate_pod(_pod_id: &str) -> Result<(), String> {
    Err("Pod manager not yet implemented. This is a placeholder for Phase 3.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        // Test would require a mock registry
    }

    #[tokio::test]
    async fn test_list_mcp_servers() {
        let runtime = McpRuntime::new();
        let servers = list_mcp_servers(&runtime).await;
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_list_mcp_tools() {
        let runtime = McpRuntime::new();
        let tools = list_mcp_tools(&runtime).await;
        assert!(tools.is_empty());
    }
}
