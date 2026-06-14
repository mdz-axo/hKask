//! Template and MCP command handlers

use crate::cli::TemplateAction;
use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};
use hkask_services::ServiceError;
use hkask_templates::SqliteRegistry;
use hkask_types::TemplateType;
use hkask_types::ports::{RegistryEntry, RegistryIndex};
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
    let registry = match SqliteRegistry::new(None) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "hkask.cli", error = %e, "SqliteRegistry in-memory failed, retrying");
            match SqliteRegistry::new(None) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(target: "hkask.cli", error = %e, "SqliteRegistry in-memory failed twice, returning empty");
                    return Vec::new();
                }
            }
        }
    };
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
) -> Result<(), ServiceError> {
    let entry = RegistryEntry {
        id: id.clone(),
        template_type,
        name: id,
        lexicon_terms,
        description,
        source_path,
        required_capabilities: vec![],
        cascade_level: 0,
        matroshka_limit: hkask_types::SYSTEM_MAX_RECURSION as u32,
    };

    registry.register(entry).map_err(ServiceError::from)
}

/// Get template command
pub fn get_template(registry: &dyn RegistryIndex, id: &str) -> Result<RegistryEntry, ServiceError> {
    registry.get(id).map_err(ServiceError::from)
}

/// Search templates by lexicon
pub fn search_templates(
    registry: &SqliteRegistry,
    term: &str,
) -> Result<Vec<RegistryEntry>, ServiceError> {
    registry.search_by_lexicon(term).map_err(ServiceError::from)
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
    let server = McpServer { id, name, tools };

    runtime.register_server(server).await;
}

/// CLI handler for `kask template` subcommand
pub fn run_template(registry: &mut SqliteRegistry, action: crate::cli::TemplateAction) {
    use crate::cli;

    match action {
        TemplateAction::List { r#type } => {
            let template_type = r#type.as_deref().and_then(cli::parse_template_type);
            let entries = list_templates(registry, template_type);
            if entries.is_empty() {
                println!("No templates registered.");
            } else {
                println!("Registered templates ({}):\n", entries.len());
                for entry in entries {
                    println!(
                        "  {} ({}) — {}",
                        entry.id,
                        entry.template_type.as_str(),
                        entry.name
                    );
                    println!("    Description: {}", entry.description);
                    println!("    Path: {}", entry.source_path);
                    if !entry.lexicon_terms.is_empty() {
                        println!("    Lexicon: {}", entry.lexicon_terms.join(", "));
                    }
                    println!();
                }
            }
        }
        TemplateAction::Register {
            id,
            path,
            r#type,
            lexicon,
            description,
        } => {
            let template_type = match cli::parse_template_type(&r#type) {
                Some(t) => t,
                None => {
                    eprintln!(
                        "Invalid template type: {}. Valid types: wordact, knowact, flowdef",
                        r#type
                    );
                    std::process::exit(1);
                }
            };
            let lexicon_terms: Vec<String> = lexicon
                .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            let desc = description.unwrap_or_else(|| format!("Template {}", id));
            super::helpers::or_exit(
                register_template(
                    registry,
                    id.clone(),
                    template_type,
                    path.to_string_lossy().to_string(),
                    lexicon_terms,
                    desc,
                ),
                "Failed to register template",
            );
            println!("Registered template: {}", id);
        }
        TemplateAction::Get { id } => {
            let entry = super::helpers::or_exit(get_template(registry, &id), "Template not found");
            println!("Template: {}", entry.id);
            println!("  Name: {}", entry.name);
            println!("  Type: {}", entry.template_type.as_str());
            println!("  Description: {}", entry.description);
            println!("  Path: {}", entry.source_path);
            println!("  Lexicon: {}", entry.lexicon_terms.join(", "));
        }
        TemplateAction::Search { term } => {
            let results =
                super::helpers::or_exit(search_templates(registry, &term), "Search failed");
            if results.is_empty() {
                println!("No templates found with lexicon term: {}", term);
            } else {
                println!("Templates matching '{}':\n", term);
                for entry in results {
                    println!(
                        "  {} ({}) — {}",
                        entry.id,
                        entry.template_type.as_str(),
                        entry.name
                    );
                }
            }
        }
    }
}
