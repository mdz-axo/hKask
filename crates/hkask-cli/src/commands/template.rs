//! Template and MCP command handlers

use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};
use hkask_templates::{RegistryEntry, RegistryIndex, SqliteRegistry, TemplateError};
use hkask_types::TemplateType;
use serde_json::Value;

/// Template list command
pub fn list_templates(
    registry: &dyn RegistryIndex,
    template_type: Option<TemplateType>,
) -> Vec<RegistryEntry> {
    registry.list(template_type)
}

/// Template list command (local in-memory, for REPL use)
pub fn list_templates_local() -> Vec<RegistryEntry> {
    let registry = SqliteRegistry::new(None).unwrap_or_else(|_| {
        SqliteRegistry::new(None).expect("SqliteRegistry::new(None) must succeed")
    });
    registry.list(None)
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
        required_capabilities: vec![],
    };

    registry.register(entry, None)
}

/// Get template command
pub fn get_template(
    registry: &dyn RegistryIndex,
    id: &str,
) -> Result<RegistryEntry, hkask_types::ports::RegistryError> {
    registry.get(id)
}

/// Search templates by lexicon
pub fn search_templates(
    registry: &SqliteRegistry,
    term: &str,
) -> Result<Vec<RegistryEntry>, TemplateError> {
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
