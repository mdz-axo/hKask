//! Pod management command handlers
//
//! Delegates pod lifecycle operations to `PodService` in `hkask-services`.
//! All context is derived from `ServiceContext` via `PodContext::from(&*ctx)` —
//! no mock PodManager construction or direct database access.

use crate::block_on;
use crate::cli::PodAction;
use hkask_agents::pod::AgentPersona;
use hkask_services::{PodContext, PodService};
use std::path::PathBuf;

/// Build a ServiceContext for pod subcommands.
async fn build_service_context() -> Result<hkask_services::ServiceContext, String> {
    let config = hkask_services::ServiceConfig::from_env()
        .map_err(|e| format!("Failed to resolve service config: {}", e))?;
    hkask_services::ServiceContext::build(config)
        .await
        .map_err(|e| format!("Failed to build service context: {}", e))
}

/// Get pod status
pub async fn get_pod_status(pod_id: &str) -> Result<hkask_agents::pod::PodStatus, String> {
    let ctx = build_service_context().await?;
    let pod_ctx = PodContext::from(&ctx);
    PodService::get_pod_status(&pod_ctx, pod_id)
        .await
        .map_err(|e| e.to_string())
}

/// List all pods
pub async fn list_pods() -> Result<Vec<hkask_agents::pod::PodStatus>, String> {
    let ctx = build_service_context().await?;
    let pod_ctx = PodContext::from(&ctx);
    PodService::list_pods(&pod_ctx)
        .await
        .map_err(|e| e.to_string())
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

    let ctx = build_service_context().await?;
    let pod_ctx = PodContext::from(&ctx);
    PodService::create_pod(
        &pod_ctx,
        template_name,
        &persona,
        pod_name.map(String::from),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Activate pod
pub async fn activate_pod(pod_id: &str) -> Result<(), String> {
    let ctx = build_service_context().await?;
    let pod_ctx = PodContext::from(&ctx);
    PodService::activate_pod(&pod_ctx, pod_id)
        .await
        .map_err(|e| e.to_string())
}

/// Deactivate pod
///
/// Note: Previous CLI code swallowed deactivation errors with `let _ = ...`.
/// The service layer now propagates errors consistently — both CLI and API
/// receive proper `PodNotFound` or `Pod(AgentPodError)` on failure.
pub async fn deactivate_pod(pod_id: &str) -> Result<(), String> {
    let ctx = build_service_context().await?;
    let pod_ctx = PodContext::from(&ctx);
    PodService::deactivate_pod(&pod_ctx, pod_id)
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
