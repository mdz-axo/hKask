//! CLI commands implementation
//!
//! This module contains the actual command handlers.

use crate::russell_mapper::{MappedTemplate, RussellMapper, RussellMappingConfig};
use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};
use hkask_mcp::transport::McpTransport;
use hkask_templates::{RegistryEntry, RegistryIndex, SqliteRegistry, TemplateError};
use hkask_types::TemplateType;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    transport: Arc<dyn McpTransport>,
) {
    let server = McpServer {
        id,
        name,
        tools,
        connected: true,
        transport: None, // Will be set by register_server
    };

    runtime.register_server(server, transport).await;
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

/// Import Russell assets into hKask registry using an existing mapper
pub fn import_russell_with_mapper(
    mapper: &RussellMapper,
    source_path: &Path,
    verbose: bool,
) -> Result<Vec<MappedTemplate>, String> {
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
                    return Err(format!("Migration failed: {}", e));
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

    Ok(assets)
}

// Git archival commands (Phase 9)
pub use super::git_archival::{
    archive_registry_to_git, create_registry_snapshot, list_registry_archives,
    restore_registry_from_git,
};

// Ensemble multi-agent commands (Phase 7)
use hkask_ensemble::{
    ChatMessage, ChatParticipant, DeliberationCoordinator, EnsembleChatManager, ParticipantRole,
};
use hkask_types::WebID;
use tokio::sync::RwLock;

/// Ensemble chat manager (singleton for CLI)
static CHAT_MANAGER: std::sync::OnceLock<Arc<RwLock<EnsembleChatManager>>> =
    std::sync::OnceLock::new();
static DELIBERATION_COORDINATOR: std::sync::OnceLock<Arc<RwLock<DeliberationCoordinator>>> =
    std::sync::OnceLock::new();

fn get_chat_manager() -> Arc<RwLock<EnsembleChatManager>> {
    CHAT_MANAGER
        .get_or_init(|| Arc::new(RwLock::new(EnsembleChatManager::new(WebID::new()))))
        .clone()
}

fn get_deliberation_coordinator() -> Arc<RwLock<DeliberationCoordinator>> {
    DELIBERATION_COORDINATOR
        .get_or_init(|| Arc::new(RwLock::new(DeliberationCoordinator::new(WebID::new()))))
        .clone()
}

/// Create chat session
pub async fn ensemble_chat_create(session: String) -> Result<String, String> {
    let manager = get_chat_manager();
    manager.write().await.create_chat(&session).await;
    Ok(format!("Chat session '{}' created", session))
}

/// Register bot in chat
pub async fn ensemble_chat_register(
    session: String,
    bot: String,
    role: String,
) -> Result<String, String> {
    let manager = get_chat_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(&session).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session))?;

    let participant_role = match role.as_str() {
        "memory_bot" => ParticipantRole::MemoryBot,
        "spandrel_bot" => ParticipantRole::SpandrelBot,
        "okapi_bot" => ParticipantRole::OkapiBot,
        "scholar_bot" => ParticipantRole::ScholarBot,
        _ => ParticipantRole::Custom(role.clone()),
    };

    let mut chat_write = chat.write().await;
    chat_write.register_participant(ChatParticipant {
        webid: WebID::new(),
        role: participant_role,
        pod_id: None,
    });

    Ok(format!(
        "Bot '{}' registered as {} in session '{}'",
        bot, role, session
    ))
}

/// Send message to chat
pub async fn ensemble_chat_send(session: String, message: String) -> Result<String, String> {
    let manager = get_chat_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(&session).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session))?;

    let mut chat_write = chat.write().await;
    let msg = ChatMessage::new(WebID::new(), message);
    chat_write.add_message(msg);

    Ok("Message sent".to_string())
}

/// List chat sessions
pub async fn ensemble_chat_list() -> Result<Vec<String>, String> {
    let manager = get_chat_manager();
    let sessions = {
        let manager_read = manager.read().await;
        manager_read.list_sessions().await
    };
    Ok(sessions)
}

/// Create deliberation session
pub async fn ensemble_deliberation_create(session: String) -> Result<String, String> {
    let coordinator = get_deliberation_coordinator();
    coordinator.write().await.create_session(&session);
    Ok(format!("Deliberation session '{}' created", session))
}

/// Start deliberation
pub async fn ensemble_deliberation_start(session: String) -> Result<String, String> {
    let coordinator = get_deliberation_coordinator();
    let mut coord_write = coordinator.write().await;
    let session_ref = coord_write
        .get_session_mut(&session)
        .ok_or_else(|| format!("Deliberation session '{}' not found", session))?;
    session_ref.start();
    Ok("Deliberation started".to_string())
}

/// Record response in deliberation
pub async fn ensemble_deliberation_record(
    session: String,
    _agent: String,
    content: String,
    confidence: f64,
) -> Result<String, String> {
    let coordinator = get_deliberation_coordinator();
    let mut coord_write = coordinator.write().await;
    let session_ref = coord_write
        .get_session_mut(&session)
        .ok_or_else(|| format!("Deliberation session '{}' not found", session))?;

    let agent_webid = WebID::new();
    let response = hkask_ensemble::AgentResponse::new(agent_webid, content, confidence);
    session_ref.record_response(response);

    Ok("Response recorded".to_string())
}

/// Synthesize deliberation
pub async fn ensemble_deliberation_synthesize(session: String) -> Result<String, String> {
    let coordinator = get_deliberation_coordinator();
    let result = {
        let coord_read = coordinator.read().await;
        let session_ref = coord_read
            .get_session(&session)
            .ok_or_else(|| format!("Deliberation session '{}' not found", session))?;
        session_ref.synthesize()
    };
    Ok(result.synthesized_response)
}

/// List deliberation sessions
pub async fn ensemble_deliberation_list() -> Result<Vec<String>, String> {
    let coordinator = get_deliberation_coordinator();
    let sessions = {
        let coord_read = coordinator.read().await;
        coord_read
            .list_sessions()
            .into_iter()
            .map(String::from)
            .collect()
    };
    Ok(sessions)
}
