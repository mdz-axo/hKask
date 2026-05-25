//! CLI commands implementation
//!
//! This module contains the actual command handlers.

use crate::errors::{AgentError, CuratorError, EnsembleError, RegistryError};
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
        required_capabilities: vec![],
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
        capabilities: vec![],
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

fn registry_db_path() -> String {
    std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string())
}

fn registry_yaml_path() -> PathBuf {
    let p = std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string());
    PathBuf::from(p)
}

async fn init_registry() -> Result<
    (
        Arc<hkask_agents::AcpRuntime>,
        hkask_storage::AgentRegistryStore,
    ),
    RegistryError,
> {
    let secret = std::env::var("HKASK_ACP_SECRET")
        .unwrap_or_else(|_| "hkask-dev-secret-minimum-eight-chars".to_string());
    let acp = Arc::new(hkask_agents::AcpRuntime::new(secret.as_bytes(), None));

    let db_path = registry_db_path();
    let passphrase = std::env::var("HKASK_DB_PASSPHRASE")
        .unwrap_or_else(|_| "hkask-dev-passphrase-minimum-eight".to_string());

    let db = if db_path == ":memory:" {
        hkask_storage::Database::in_memory()
            .map_err(|e| RegistryError::DatabaseError(e.to_string()))?
    } else {
        hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| RegistryError::DatabaseError(e.to_string()))?
    };

    let store = hkask_storage::AgentRegistryStore::new(db.conn_arc());
    store
        .initialize_schema()
        .map_err(|e| RegistryError::SchemaError(e.to_string()))?;

    // R2: Restore agent state from persistent storage
    let registered_agents = store
        .list()
        .map_err(|e| RegistryError::LoadFailed(e.to_string()))?;

    if !registered_agents.is_empty() {
        // Restore AcpRuntime state from storage
        let agents: Vec<hkask_agents::acp::AcpAgent> = registered_agents
            .iter()
            .map(|ra| hkask_agents::acp::AcpAgent {
                webid: hkask_types::WebID::from_string(&ra.definition.name),
                agent_type: ra.definition.agent_kind.as_str().to_string(),
                capabilities: ra.definition.capabilities.clone(),
                registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                active: true,
            })
            .collect();

        // Restore capability tokens (empty for now - R8 will add token persistence)
        let tokens = std::collections::HashMap::new();

        acp.restore_from_storage(agents, tokens)
            .await
            .map_err(|e| RegistryError::LoadFailed(e.to_string()))?;
    }

    Ok((acp, store))
}

pub struct AgentReceipt {
    pub webid: String,
    pub token_hash: String,
    pub registered_at: String,
}

pub async fn bot_list(
    kind_filter: Option<&str>,
) -> Result<Vec<hkask_types::RegisteredAgent>, AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let loader = hkask_agents::BotRegistryLoader::new(registry_yaml_path(), _acp, store);

    let agents = loader
        .boot()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let filtered = if let Some(kind_str) = kind_filter {
        let kind = hkask_types::AgentKind::parse(kind_str)
            .ok_or_else(|| AgentError::InvalidType(kind_str.to_string()))?;
        agents
            .into_iter()
            .filter(|a| a.definition.agent_kind == kind)
            .collect()
    } else {
        agents
    };

    Ok(filtered)
}

pub async fn bot_status(name: &str) -> Result<hkask_types::RegisteredAgent, AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let loader = hkask_agents::BotRegistryLoader::new(registry_yaml_path(), _acp, store);

    let agents = loader
        .boot()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    agents
        .into_iter()
        .find(|a| a.definition.name == name)
        .ok_or_else(|| AgentError::NotFound(name.to_string()))
}

pub async fn agent_register(
    webid_str: &str,
    agent_type: &str,
    capabilities: Vec<String>,
) -> Result<AgentReceipt, AgentError> {
    let (acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    let webid = hkask_types::WebID::from_string(webid_str);

    let token = acp
        .register_agent(webid, agent_type.to_string(), capabilities)
        .await
        .map_err(|e| AgentError::RegistrationFailed(e.to_string()))?;

    let definition = hkask_types::AgentDefinition {
        name: webid_str.to_string(),
        agent_kind: hkask_types::AgentKind::parse(agent_type)
            .unwrap_or(hkask_types::AgentKind::Bot),
        binding_contract: false,
        editor: "cli".to_string(),
        charter: None,
        capabilities: vec![],
        rights: vec![],
        responsibilities: vec![],
        reporting: None,
        standing_session: None,
        persona: None,
        depends_on: vec![],
        readiness_probe: None,
        process_manifest: None,
    };

    let registered = hkask_types::RegisteredAgent {
        definition,
        token_hash: token.signature.clone(),
        registered_at: chrono::Utc::now().to_rfc3339(),
        source_yaml: "cli-register".to_string(),
    };

    store
        .insert(&registered)
        .map_err(|e| AgentError::RegistrationFailed(e.to_string()))?;

    Ok(AgentReceipt {
        webid: webid_str.to_string(),
        token_hash: token.signature,
        registered_at: registered.registered_at,
    })
}

pub async fn agent_unregister(name: &str) -> Result<(), AgentError> {
    let (_acp, store) = init_registry()
        .await
        .map_err(|e| AgentError::CapabilityError(e.to_string()))?;

    store
        .remove(name)
        .map_err(|e| AgentError::UnregistrationFailed(e.to_string()))?;

    Ok(())
}

pub async fn chat_with_agent(input: &str, agent_name: Option<&str>) -> String {
    use hkask_agents::pod::{PodContext, PodManagerBuilder};
    use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
    use hkask_types::LLMParameters;
    use std::sync::Arc;

    let name = agent_name.unwrap_or("Curator");

    // Load agent registry
    let (acp, store) = match init_registry().await {
        Ok(r) => r,
        Err(e) => return format!("Registry init error: {}", e),
    };

    let loader = hkask_agents::BotRegistryLoader::new(registry_yaml_path(), acp.clone(), store);

    let agents = match loader.boot().await {
        Ok(a) => a,
        Err(e) => return format!("Registry load error: {}", e),
    };

    let agent = agents.iter().find(|a| a.definition.name == name);

    // R11: Wire Russell Direct Chat
    // Check if this is a Russell chat request
    if name == "russell" || name == "Russell" {
        // Check if Russell is registered
        if agent.is_none() {
            return "Russell is not registered. Use `kask agent register` to register Russell first.".to_string();
        }

        // Use RussellAcpAdapter for direct chat
        use hkask_agents::acp::A2AMessage;
        use hkask_agents::adapters::RussellAcpAdapter;
        use hkask_agents::ports::AcpPort;
        use hkask_types::WebID;
        use zeroize::Zeroizing;

        // Get Russell binary path from environment or use default
        let russell_binary = std::env::var("HKASK_RUSSELL_BINARY")
            .unwrap_or_else(|_| "russell-acp-server".to_string());

        // Get bridge secret from environment or use default
        let bridge_secret_str = std::env::var("HKASK_ACP_SECRET")
            .unwrap_or_else(|_| "hkask-dev-secret-minimum-eight-chars".to_string());
        let bridge_secret = Arc::new(Zeroizing::new(bridge_secret_str.into_bytes()));

        // Create Russell adapter
        let russell_adapter = RussellAcpAdapter::new(russell_binary, bridge_secret);

        // Create a WebID for this chat session
        let webid = WebID::from_persona_with_namespace(b"russell-chat-session", "russell");

        // Register with Russell (creates a session)
        if let Err(e) = russell_adapter
            .register_agent(webid, "Replicant", vec![])
            .await
        {
            return format!("Failed to create Russell session: {}", e);
        }

        // Send the message to Russell
        let message = A2AMessage::TemplateDispatch {
            from: webid,
            to: Some(webid),
            template_id: "russell:direct-chat".to_string(),
            input: serde_json::json!({
                "message": input,
            }),
            correlation_id: uuid::Uuid::new_v4().to_string(),
        };

        match russell_adapter.send_message(message).await {
            Ok(response) => return response,
            Err(e) => return format!("Russell error: {}", e),
        }
    } else {
        // Standard chat flow for non-Russell agents
        let system_prompt = match agent {
            Some(registered) => registered.definition.compose_system_prompt(),
            None => format!("You are {}, an assistant in the hKask system.\n\n", name),
        };

        // Create inference port
        let config = OkapiConfig::local_dev();
        let inference = match OkapiInference::new("qwen3:8b", config) {
            Ok(i) => Arc::new(i) as Arc<dyn InferencePort>,
            Err(e) => return format!("Okapi init error: {}", e),
        };

        // Create PodManager with inference port (R1: Restore Pod Invariant)
        let pod_manager = PodManagerBuilder::new()
            .acp_runtime(acp)
            .inference_port(inference.clone())
            .with_in_memory_storage()
            .build();

        // Create or find pod for this agent
        let persona_yaml = format!(
            r#"
agent:
  name: {}
  type: {}
  version: "0.1.0"
charter:
  description: "Chat session with {}"
  editor: cli
capabilities:
  - "tool:inference:call"
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#,
            name,
            if name == "Curator" {
                "Replicant"
            } else {
                "Bot"
            },
            name
        );

        let persona = match hkask_agents::pod::AgentPersona::from_yaml(&persona_yaml) {
            Ok(p) => p,
            Err(e) => return format!("Persona parse error: {}", e),
        };

        let pod_id = match pod_manager
            .create_pod("chat-template", &persona, Some(name.to_string()))
            .await
        {
            Ok(id) => id,
            Err(e) => return format!("Pod creation error: {}", e),
        };

        // Activate pod (registers with ACP, grants MCP access)
        if let Err(e) = pod_manager.activate_pod(&pod_id).await {
            return format!("Pod activation error: {}", e);
        }

        // Create PodContext (R1: all access goes through pod)
        let pod_context = match PodContext::from_manager(&pod_manager, &pod_id).await {
            Ok(ctx) => ctx,
            Err(e) => return format!("Pod context error: {}", e),
        };

        // Emit CNS span for observability
        pod_context.emit_span(
            "cns.prompt.chat",
            "chat_interaction",
            serde_json::json!({
                "agent": name,
                "input_length": input.len(),
            }),
        );

        // Build prompt with system context
        let full_prompt = format!("{}\n\nUser: {}", system_prompt, input);

        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
        };

        // Use inference port from PodContext (R1: pod-mediated inference)
        let inference_port = match pod_context.inference_port() {
            Some(port) => port,
            None => return "No inference port available in pod context".to_string(),
        };

        match inference_port.generate(&full_prompt, &params).await {
            Ok(result) => result.text,
            Err(e) => format!("Inference error: {}", e),
        }
    }
}

pub async fn curator_escalations() -> Result<Vec<hkask_agents::EscalationEntry>, CuratorError> {
    use rusqlite::Connection;
    use std::sync::Arc;

    let db_path = registry_db_path();
    let conn = if db_path == ":memory:" {
        Connection::open_in_memory().map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    } else {
        Connection::open(&db_path).map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    };

    let queue = hkask_agents::EscalationQueue::new(Arc::new(conn))
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .list_pending()
        .map_err(|e| CuratorError::EscalationNotFound(e.to_string()))
}

pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    use rusqlite::Connection;
    use std::sync::Arc;

    let db_path = registry_db_path();
    let conn = if db_path == ":memory:" {
        Connection::open_in_memory().map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    } else {
        Connection::open(&db_path).map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    };

    let queue = hkask_agents::EscalationQueue::new(Arc::new(conn))
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .resolve(id, "cli-administrator")
        .map_err(|e| CuratorError::EscalationResolutionFailed(e.to_string()))
}

pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    use rusqlite::Connection;
    use std::sync::Arc;

    let db_path = registry_db_path();
    let conn = if db_path == ":memory:" {
        Connection::open_in_memory().map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    } else {
        Connection::open(&db_path).map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    };

    let queue = hkask_agents::EscalationQueue::new(Arc::new(conn))
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .dismiss(id, "cli-administrator")
        .map_err(|e| CuratorError::EscalationResolutionFailed(e.to_string()))
}

pub async fn curator_metacognition() -> Result<String, CuratorError> {
    use hkask_agents::adapters::CnsRuntimeAdapter;
    use hkask_agents::curator::{MetacognitionConfig, MetacognitionLoop};
    use hkask_cns::CnsRuntime;
    use rusqlite::Connection;
    use std::sync::Arc;

    let db_path = registry_db_path();
    let conn = if db_path == ":memory:" {
        Connection::open_in_memory().map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    } else {
        Connection::open(&db_path).map_err(|e| CuratorError::DatabaseError(e.to_string()))?
    };

    let queue = Arc::new(
        hkask_agents::EscalationQueue::new(Arc::new(conn))
            .map_err(|e| CuratorError::DatabaseError(e.to_string()))?,
    );

    let cns = Arc::new(CnsRuntimeAdapter::new(Arc::new(CnsRuntime::new())));
    let config = MetacognitionConfig::default();
    let loop_instance = MetacognitionLoop::new(cns, queue, config);

    let snapshot = loop_instance
        .run_cycle()
        .await
        .map_err(|e| CuratorError::MetacognitionFailed(e.to_string()))?;

    Ok(loop_instance.generate_summary(&snapshot))
}

pub fn ensemble_standing_start(
    config_path: &std::path::Path,
) -> Result<hkask_ensemble::StandingSessionStatus, EnsembleError> {
    let session = hkask_ensemble::bootstrap_standing_session(config_path)
        .map_err(|e| EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}

pub fn ensemble_standing_status() -> Result<hkask_ensemble::StandingSessionStatus, EnsembleError> {
    let config_path = std::path::Path::new("registry/manifests/standing-ensemble-session.yaml");
    if !config_path.exists() {
        return Err(EnsembleError::SessionNotFound(
            "Standing session not bootstrapped. Run 'kask ensemble standing-start' first."
                .to_string(),
        ));
    }

    let session = hkask_ensemble::bootstrap_standing_session(config_path)
        .map_err(|e| EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}
