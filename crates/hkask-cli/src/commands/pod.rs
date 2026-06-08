//! Pod management command handlers
//!
//! Delegates pod lifecycle operations to `PodService` in `hkask-services`.
//! Surface concerns (file I/O for persona YAML, PodManager construction)
//! stay here. Business logic (UUID parsing, error normalization) moves to
//! the service layer.

use crate::block_on;
use crate::cli::PodAction;
use hkask_agents::pod::{AgentPersona, PodManager, PodManagerBuilder};
use hkask_services::{PodContext, PodService};
use hkask_types::CapabilityChecker;
use std::path::PathBuf;
use std::sync::Arc;

/// Get pod status
pub async fn get_pod_status(pod_id: &str) -> Result<hkask_agents::pod::PodStatus, String> {
    let ctx = PodContext::from_parts(Arc::new(PodManager::new_mock()));
    PodService::get_pod_status(&ctx, pod_id)
        .await
        .map_err(|e| e.to_string())
}

/// List all pods
pub async fn list_pods() -> Result<Vec<hkask_agents::pod::PodStatus>, String> {
    let (acp, _store) = crate::commands::config::init_registry()
        .await
        .map_err(|e| {
            format!(
                "Registry not initialized: {}. Run `kask chat` to complete onboarding.",
                e
            )
        })?;

    let acp_secret = crate::commands::config::resolve_acp_secret()
        .map_err(|e| format!("ACP secret resolution error: {}", e))?;

    let manager = PodManagerBuilder::new()
        .acp_runtime(acp)
        .capability_checker(CapabilityChecker::new(acp_secret.as_bytes()))
        .with_in_memory_storage()
        .build();

    let ctx = PodContext::from_parts(Arc::new(manager));
    PodService::list_pods(&ctx).await.map_err(|e| e.to_string())
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

    let ctx = PodContext::from_parts(Arc::new(PodManager::new_mock()));
    PodService::create_pod(&ctx, template_name, &persona, pod_name.map(String::from))
        .await
        .map_err(|e| e.to_string())
}

/// Activate pod
pub async fn activate_pod(pod_id: &str) -> Result<(), String> {
    let ctx = PodContext::from_parts(Arc::new(PodManager::new_mock()));
    PodService::activate_pod(&ctx, pod_id)
        .await
        .map_err(|e| e.to_string())
}

/// Deactivate pod
///
/// Note: Previous CLI code swallowed deactivation errors with `let _ = ...`.
/// The service layer now propagates errors consistently — both CLI and API
/// receive proper `PodNotFound` or `Pod(AgentPodError)` on failure.
pub async fn deactivate_pod(pod_id: &str) -> Result<(), String> {
    let ctx = PodContext::from_parts(Arc::new(PodManager::new_mock()));
    PodService::deactivate_pod(&ctx, pod_id)
        .await
        .map_err(|e| e.to_string())
}

/// CLI handler for `kask pod` subcommand
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
            Err(e) => eprintln!("Pod listing unavailable: {}", e),
        },
    }
}
