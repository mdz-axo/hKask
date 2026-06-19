//! Pod management command handlers — delegates to PodService.

use hkask_services::{PodService, PodStatusResponse, ServiceError};

use crate::cli::PodAction;

pub async fn get_pod_status(pod_id: &str) -> Result<PodStatusResponse, ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::get_pod_status(&ctx, pod_id).await
}

pub async fn list_pods() -> Result<Vec<PodStatusResponse>, ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::list_pods(&ctx).await
}

pub async fn create_pod(
    template: &str,
    persona_path: &std::path::PathBuf,
    name: Option<&str>,
) -> Result<String, ServiceError> {
    let yaml = std::fs::read_to_string(persona_path)
        .map_err(|e| ServiceError::Infra(hkask_types::InfrastructureError::Io(e.to_string())))?;
    let ctx = super::helpers::build_service_context();
    let resp = PodService::create_pod(
        &ctx,
        hkask_services::CreatePodRequest {
            template: template.to_string(),
            persona_yaml: yaml,
            name: name.map(String::from),
        },
    )
    .await?;
    Ok(resp.pod_id)
}

pub async fn activate_pod(pod_id: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::activate_pod(&ctx, pod_id).await
}

pub async fn deactivate_pod(pod_id: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::deactivate_pod(&ctx, pod_id).await
}

pub async fn assign_role(name: &str, role: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::assign_role(&ctx, name, role).await
}

pub async fn set_mode(name: &str, mode: &str, role: Option<&str>) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::set_mode(&ctx, name, mode, role).await
}

pub fn run_pod(rt: &tokio::runtime::Runtime, action: crate::cli::PodAction) {
    use crate::commands;
    match action {
        PodAction::Create {
            template,
            persona,
            name,
        } => {
            let pod_id = crate::block_on!(
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
            crate::block_on!(
                rt,
                commands::activate_pod(&pod_id),
                "Failed to activate pod"
            );
            println!("Activated agent pod: {}", pod_id);
        }
        PodAction::Deactivate { pod_id } => {
            crate::block_on!(
                rt,
                commands::deactivate_pod(&pod_id),
                "Failed to deactivate pod"
            );
            println!("Deactivated agent pod: {}", pod_id);
        }
        PodAction::Status { pod_id, verbose } => {
            let status = crate::block_on!(
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
        PodAction::Assign { name, role } => {
            crate::block_on!(
                rt,
                commands::assign_role(&name, &role),
                "Failed to assign role"
            );
            println!("Assigned MCP role '{}' to replicant '{}'", role, name);
        }
        PodAction::Mode { name, mode, role } => {
            crate::block_on!(
                rt,
                commands::set_mode(&name, &mode, role.as_deref()),
                "Failed to set mode"
            );
            match role {
                Some(r) => println!("Set replicant '{}' to server mode serving '{}'", name, r),
                None => println!("Set replicant '{}' to {} mode", name, mode),
            }
        }
    }
}
