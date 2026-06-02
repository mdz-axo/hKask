//! Pod management command handlers

use hkask_agents::pod::{AgentPersona, PodID, PodManager, PodManagerBuilder, PodStatus};
use std::path::PathBuf;
use uuid::Uuid;

/// Get pod status
pub async fn get_pod_status(pod_id: &str) -> Result<PodStatus, String> {
    let uuid = Uuid::parse_str(pod_id).map_err(|e| format!("Invalid pod ID: {}", e))?;
    let manager = PodManager::new_mock();
    manager
        .get_pod_status(&PodID(uuid))
        .await
        .map_err(|e| e.to_string())
}

/// List all pods
pub async fn list_pods() -> Result<Vec<PodStatus>, String> {
    let (acp, _store) = crate::commands::config::init_registry()
        .await
        .map_err(|e| {
            format!(
                "Registry not initialized: {}. Run `kask chat` to complete onboarding.",
                e
            )
        })?;

    let manager = PodManagerBuilder::new()
        .acp_runtime(acp)
        .with_in_memory_storage()
        .build();

    manager.list_pods().await.map_err(|e| e.to_string())
}

/// Create pod from template crate
pub async fn create_pod(
    template_name: &str,
    persona_path: &PathBuf,
    pod_name: Option<&str>,
) -> Result<String, String> {
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
    let uuid = Uuid::parse_str(pod_id).map_err(|e| format!("Invalid pod ID: {}", e))?;
    let manager = PodManager::new_mock();
    manager
        .deactivate_pod(&PodID(uuid))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
