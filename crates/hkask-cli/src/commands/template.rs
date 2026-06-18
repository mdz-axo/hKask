//! Template and MCP command handlers

use crate::cli::TemplateAction;
use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};
use hkask_services::ServiceError;
use hkask_templates::SqliteRegistry;
use hkask_types::ports::{RegistryEntry, RegistryIndex};
use hkask_types::template_type::TemplateType;
use serde_json::Value;

/// Template list command
///
/// REQ: CLI-019
/// pre:  registry is a valid RegistryIndex
/// post: returns Vec<RegistryEntry> filtered by optional template_type
/// post: delegates to registry.list()
pub fn list_templates(
    registry: &dyn RegistryIndex,
    template_type: Option<TemplateType>,
) -> Vec<RegistryEntry> {
    registry.list(template_type)
}

/// Template list command (local in-memory, for REPL use)
///
/// REQ: CLI-020
/// pre:  none
/// post: returns Vec<RegistryEntry> from in-memory SqliteRegistry
/// post: if registry creation fails twice → returns empty Vec (graceful degradation)
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
///
/// REQ: CLI-021
/// pre:  registry is a mutable SqliteRegistry
/// pre:  id, template_type, source_path, description are valid
/// post: returns Ok(()) if template registered successfully
/// post: returns Err(ServiceError) if registration fails
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

    registry
        .register(entry)
        .map_err(|e| ServiceError::Template {
            message: e.to_string(),
        })
}

/// Get template command
///
/// REQ: CLI-022
/// pre:  registry is a valid RegistryIndex
/// pre:  id is a valid template identifier
/// post: returns Ok(RegistryEntry) if template found
/// post: returns Err(ServiceError) if not found
pub fn get_template(registry: &dyn RegistryIndex, id: &str) -> Result<RegistryEntry, ServiceError> {
    registry.get(id).map_err(|e| ServiceError::Registry {
        message: e.to_string(),
    })
}

/// Search templates by lexicon
///
/// REQ: CLI-023
/// pre:  registry is a valid SqliteRegistry
/// pre:  term is a non-empty search string
/// post: returns Ok(Vec<RegistryEntry>) with matching templates
/// post: returns Err(ServiceError) if search fails
pub fn search_templates(
    registry: &SqliteRegistry,
    term: &str,
) -> Result<Vec<RegistryEntry>, ServiceError> {
    registry
        .search_by_lexicon(term)
        .map_err(|e| ServiceError::Template {
            message: e.to_string(),
        })
}

/// List MCP servers
///
/// REQ: CLI-024
/// pre:  runtime is a valid McpRuntime
/// post: returns Vec<McpServer> with all registered servers
/// post: delegates to runtime.list_servers()
pub async fn list_mcp_servers(runtime: &McpRuntime) -> Vec<McpServer> {
    runtime.list_servers().await
}

/// List MCP tools
///
/// REQ: CLI-025
/// pre:  runtime is a valid McpRuntime
/// post: returns Vec<String> with all discovered tool names
/// post: delegates to runtime.discover_tools()
pub async fn list_mcp_tools(runtime: &McpRuntime) -> Vec<String> {
    runtime.discover_tools().await
}

/// Get MCP tool definition
///
/// REQ: CLI-026
/// pre:  runtime is a valid McpRuntime
/// pre:  name is a valid tool name
/// post: returns Some(Value) with tool metadata if found
/// post: returns None if tool not found
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
///
/// REQ: CLI-027
/// pre:  runtime is a valid McpRuntime
/// pre:  id, name, tools are valid
/// post: server is registered with the runtime
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
///
/// REQ: CLI-028
/// pre:  registry is a mutable SqliteRegistry
/// pre:  action is a valid TemplateAction variant
/// post: dispatches to the appropriate template command handler
/// post: prints result or error to stdout
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
