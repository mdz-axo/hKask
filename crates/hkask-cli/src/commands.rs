//! CLI commands implementation
//!
//! This module contains the actual command handlers.

use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};
use hkask_templates::{
    MappedTemplate, RegistryEntry, RegistryIndex, RussellMapper, RussellMappingConfig,
    SqliteRegistry, TemplateError,
};
use hkask_types::TemplateType;
use serde_json::Value;
use std::path::{Path, PathBuf};

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

/// Get pod status
pub async fn get_pod_status(pod_id: &str) -> Result<PodStatus, String> {
    use hkask_agents::pod::{PodID, PodManager};
    use uuid::Uuid;

    let uuid = Uuid::parse_str(pod_id).map_err(|e| format!("Invalid pod ID: {}", e))?;
    let manager = PodManager::new_mock();
    let status = manager
        .get_pod_status(&PodID(uuid))
        .await
        .map_err(|e| e.to_string())?;

    Ok(PodStatus {
        pod_id: status.pod_id,
        name: status.name,
        state: status.state,
        webid: status.webid,
        created_at: status.created_at.to_string(),
    })
}

/// List all pods
pub async fn list_pods() -> Vec<PodStatus> {
    use hkask_agents::pod::PodManager;

    let manager = PodManager::new_mock();
    let statuses = manager.list_pods().await.unwrap_or_default();

    statuses
        .into_iter()
        .map(|s| PodStatus {
            pod_id: s.pod_id,
            name: s.name,
            state: s.state,
            webid: s.webid,
            created_at: s.created_at.to_string(),
        })
        .collect()
}

/// Create pod from template crate
pub async fn create_pod(
    template_name: &str,
    persona_path: &PathBuf,
    pod_name: Option<&str>,
) -> Result<String, String> {
    use hkask_agents::pod::{AgentPersona, PodManager};

    let yaml_content = std::fs::read_to_string(persona_path)
        .map_err(|e| format!("Failed to read persona file: {}", e))?;

    let persona = AgentPersona::from_yaml(&yaml_content)
        .map_err(|e| format!("Invalid persona YAML: {}", e))?;

    let manager = PodManager::new_mock();
    let pod_id = manager
        .create_pod(template_name, &persona, pod_name.map(String::from))
        .await
        .map_err(|e| e.to_string())?;

    Ok(pod_id.to_string())
}

/// Activate pod
pub async fn activate_pod(pod_id: &str) -> Result<(), String> {
    use hkask_agents::pod::{PodID, PodManager};
    use uuid::Uuid;

    let uuid = Uuid::parse_str(pod_id).map_err(|e| format!("Invalid pod ID: {}", e))?;
    let manager = PodManager::new_mock();
    manager
        .activate_pod(&PodID(uuid))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Deactivate pod
pub async fn deactivate_pod(pod_id: &str) -> Result<(), String> {
    use hkask_agents::pod::{PodID, PodManager};
    use uuid::Uuid;

    let uuid = Uuid::parse_str(pod_id).map_err(|e| format!("Invalid pod ID: {}", e))?;
    let manager = PodManager::new_mock();
    manager
        .deactivate_pod(&PodID(uuid))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Import Russell assets into hKask registry
pub fn import_russell(
    source_path: &Path,
    config: &RussellMappingConfig,
    verbose: bool,
) -> Result<Vec<MappedTemplate>, String> {
    let mapper = RussellMapper::new();
    let mut assets = Vec::new();

    if source_path.is_file() {
        let extension = source_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match extension {
            "yaml" | "yml" => match mapper.analyze_skill_manifest(source_path) {
                Ok(manifest) => {
                    let mapped = mapper.map_to_hkask(&manifest);
                    if verbose {
                        println!("Mapped Russell manifest: {} -> {}", manifest.id, mapped.id);
                    }
                    assets.push(mapped);
                }
                Err(e) => {
                    eprintln!("Failed to analyze {}: {}", source_path.display(), e);
                    if !config.dry_run {
                        return Err(format!("Migration failed: {}", e));
                    }
                }
            },
            _ => {
                return Err(format!("Unsupported file type: {}", extension));
            }
        }
    } else if source_path.is_dir() {
        for entry in std::fs::read_dir(source_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                match extension {
                    "yaml" | "yml" => match mapper.analyze_skill_manifest(&path) {
                        Ok(manifest) => {
                            let mapped = mapper.map_to_hkask(&manifest);
                            if verbose {
                                println!(
                                    "Mapped Russell manifest: {} -> {}",
                                    manifest.id, mapped.id
                                );
                            }
                            assets.push(mapped);
                        }
                        Err(e) => {
                            eprintln!("Failed to analyze {}: {}", path.display(), e);
                            if !config.dry_run {
                                return Err(format!("Migration failed: {}", e));
                            }
                        }
                    },
                    _ => {}
                }
            } else if path.is_dir() {
                for sub_entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
                    let sub_entry = sub_entry.map_err(|e| e.to_string())?;
                    let sub_path = sub_entry.path();

                    if sub_path.is_file() {
                        let extension = sub_path.extension().and_then(|s| s.to_str()).unwrap_or("");

                        if (extension == "yaml" || extension == "yml")
                            && sub_path.file_name().and_then(|s| s.to_str())
                                == Some("manifest.yaml")
                        {
                            match mapper.analyze_skill_manifest(&sub_path) {
                                Ok(manifest) => {
                                    let mapped = mapper.map_to_hkask(&manifest);
                                    if verbose {
                                        println!(
                                            "Mapped Russell manifest: {} -> {}",
                                            manifest.id, mapped.id
                                        );
                                    }
                                    assets.push(mapped);
                                }
                                Err(e) => {
                                    eprintln!("Failed to analyze {}: {}", sub_path.display(), e);
                                    if !config.dry_run {
                                        return Err(format!("Migration failed: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(format!(
            "Source path does not exist: {}",
            source_path.display()
        ));
    }

    if config.dry_run {
        println!("\nDry run complete - no assets written to registry");
    }

    Ok(assets)
}
