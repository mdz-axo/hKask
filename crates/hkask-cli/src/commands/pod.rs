//! Pod management command handlers — call PodManager directly.

use std::sync::Arc;

use crate::block_on;
use crate::cli::PodAction;
use hkask_agents::pod::{AgentPersona, AgentPodError, PodID, PodManager, PodStatus};
use uuid::Uuid;

fn parse_pod_id(id: &str) -> Result<PodID, AgentPodError> {
    Uuid::parse_str(id)
        .map(PodID::from_uuid)
        .map_err(|_| AgentPodError::PodNotFound(PodID::from_uuid(Uuid::nil())))
}

fn normalize_pod_error(e: AgentPodError) -> String {
    match &e {
        AgentPodError::PodNotFound(id) => format!("Pod {} not found", id),
        _ => e.to_string(),
    }
}

async fn build_pod_manager() -> Result<Arc<PodManager>, String> {
    let config = hkask_services::ServiceConfig::from_env().map_err(|e| format!("Config: {e}"))?;
    let _db = hkask_storage::Database::open(&config.db_path, &config.db_passphrase)
        .map_err(|e| format!("DB: {e}"))?;
    let acp = Arc::new(hkask_agents::AcpRuntime::new(&config.acp_secret));
    let mcp = Arc::new(hkask_agents::adapters::mcp_runtime::FullMcpAdapter::new(
        Arc::new(hkask_types::CapabilityChecker::new(&config.acp_secret)),
        Arc::new(hkask_mcp::runtime::McpRuntime::new()),
        tokio::runtime::Handle::current(),
    ));
    let git = Arc::new(hkask_mcp::GitCasAdapter::from_path(
        std::path::PathBuf::from(&config.template_cache_path),
    ));

    // Minimal memory for pod creation
    let mem_db = hkask_storage::in_memory_db();
    let mem_conn = mem_db.conn_arc();
    let adapter = Arc::new(
        hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter::new(
            hkask_memory::EpisodicMemory::new(hkask_storage::TripleStore::new(Arc::clone(
                &mem_conn,
            ))),
            hkask_memory::SemanticMemory::new(
                hkask_storage::TripleStore::new(Arc::clone(&mem_conn)),
                hkask_storage::EmbeddingStore::new(Arc::clone(&mem_conn)),
            ),
        ),
    );
    let epi: Arc<dyn hkask_agents::ports::EpisodicStoragePort> = adapter.clone();
    let sem: Arc<dyn hkask_agents::ports::SemanticStoragePort> = adapter;

    Ok(Arc::new(PodManager::new(
        Some(git),
        Some(acp),
        Some(mcp),
        Some(epi),
        Some(sem),
        None,
        None,
        None,
        None,
    )))
}

pub async fn get_pod_status(pod_id: &str) -> Result<PodStatus, String> {
    let pm = build_pod_manager().await?;
    let id = parse_pod_id(pod_id).map_err(normalize_pod_error)?;
    pm.get_pod_status(&id).await.map_err(normalize_pod_error)
}

pub async fn list_pods() -> Result<Vec<PodStatus>, String> {
    let pm = build_pod_manager().await?;
    pm.list_pods().await.map_err(normalize_pod_error)
}

pub async fn create_pod(
    template: &str,
    persona_path: &std::path::PathBuf,
    name: Option<&str>,
) -> Result<String, String> {
    let yaml = std::fs::read_to_string(persona_path).map_err(|e| format!("Read persona: {e}"))?;
    let persona = AgentPersona::from_yaml(&yaml).map_err(|e| format!("Invalid persona: {e}"))?;
    let pm = build_pod_manager().await?;
    pm.create_pod(template, &persona, name.map(String::from))
        .await
        .map(|id| id.to_string())
        .map_err(normalize_pod_error)
}

pub async fn activate_pod(pod_id: &str) -> Result<(), String> {
    let pm = build_pod_manager().await?;
    let id = parse_pod_id(pod_id).map_err(normalize_pod_error)?;
    pm.activate_pod(&id).await.map_err(normalize_pod_error)
}

pub async fn deactivate_pod(pod_id: &str) -> Result<(), String> {
    let pm = build_pod_manager().await?;
    let id = parse_pod_id(pod_id).map_err(normalize_pod_error)?;
    pm.deactivate_pod(&id).await.map_err(normalize_pod_error)
}

pub fn run_pod(rt: &tokio::runtime::Runtime, action: crate::cli::PodAction) {
    use crate::commands;
    match action {
        PodAction::Create {
            template,
            persona,
            name,
        } => {
            let pod_id = block_on!(
                rt,
                commands::create_pod(&template, &persona, name.as_deref()),
                "Failed to create pod"
            );
            println!("Created agent pod: {}", pod_id);
            println!("Template: {}", template);
            println!("Persona file: {}", persona.display());
            if let Some(n) = &name {
                println!("Pod name: {}", n);
            }
        }
        PodAction::Activate { pod_id } => {
            block_on!(
                rt,
                commands::activate_pod(&pod_id),
                "Failed to activate pod"
            );
            println!("Activated agent pod: {}", pod_id);
        }
        PodAction::Deactivate { pod_id } => {
            block_on!(
                rt,
                commands::deactivate_pod(&pod_id),
                "Failed to deactivate pod"
            );
            println!("Deactivated agent pod: {}", pod_id);
        }
        PodAction::Status { pod_id, verbose } => {
            let status = block_on!(
                rt,
                commands::get_pod_status(&pod_id),
                "Failed to get pod status"
            );
            println!("Agent pod status: {}", pod_id);
            println!("  State: {}", status.state);
            println!("  WebID: {}", status.webid);
            if let Some(name) = &status.name {
                println!("  Name: {}", name);
            }
            if verbose {
                println!("  Created at: {}", status.created_at);
            }
        }
        PodAction::List => match rt.block_on(commands::list_pods()) {
            Ok(pods) => {
                if pods.is_empty() {
                    println!("No pods registered.");
                } else {
                    println!("Agent pods ({}):\n", pods.len());
                    for pod in pods {
                        println!("  {} ({})", pod.pod_id, pod.state);
                        println!("    WebID: {}", pod.webid);
                        if let Some(name) = &pod.name {
                            println!("    Name: {}", name);
                        }
                        println!();
                    }
                }
            }
            Err(e) => eprintln!("Pod listing unavailable: {e}"),
        },
    }
}
