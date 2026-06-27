//! Pod management command handlers — direct calls to pod manager.
//! Formerly delegated to PodService (removed v0.31.0 per P5).

use hkask_agents::pod::PodStatusInfo;

use crate::cli::PodAction;

/// Run a pod command.
pub fn run_pod(rt: &tokio::runtime::Runtime, action: PodAction) {
    rt.block_on(run_pod_inner(action));
}

async fn run_pod_inner(action: PodAction) {
    match action {
        PodAction::Create {
            template,
            persona,
            name,
        } => match create_pod(&template, &persona, name.as_deref()).await {
            Ok(pod_id) => println!("Created pod: {}", pod_id),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        PodAction::List => match list_pods().await {
            Ok(pods) => {
                if pods.is_empty() {
                    println!("No pods registered.");
                } else {
                    for p in &pods {
                        println!(
                            "  {} [{}] {} ({})",
                            p.pod_id,
                            p.state,
                            p.name.as_deref().unwrap_or("unnamed"),
                            p.agent_type
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        PodAction::Activate { pod_id } => match activate_pod(&pod_id).await {
            Ok(()) => println!("Pod {} activated", pod_id),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        PodAction::Deactivate { pod_id } => match deactivate_pod(&pod_id).await {
            Ok(()) => println!("Pod {} deactivated", pod_id),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        PodAction::Status { pod_id, verbose: _ } => match get_pod_status(&pod_id).await {
            Ok(status) => {
                println!("Pod {}", status.pod_id);
                println!(
                    "  name:       {}",
                    status.name.as_deref().unwrap_or("unnamed")
                );
                println!("  state:      {}", status.state);
                println!("  webid:      {}", status.webid);
                println!("  agent_type: {}", status.agent_type);
                println!("  template:   {}", status.template);
                println!("  created_at: {}", status.created_at);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        PodAction::Assign { name, role } => match assign_role(&name, &role).await {
            Ok(()) => println!("Role '{}' assigned to '{}'", role, name),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        PodAction::Mode { name, mode, role } => {
            match set_mode(&name, &mode, role.as_deref()).await {
                Ok(()) => println!("Mode '{}' set for '{}'", mode, name),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        PodAction::ExportContainer { pod_id, output } => {
            match export_container(&pod_id, &output).await {
                Ok(()) => println!(
                    "Pod '{}' exported as container build context to {}",
                    pod_id,
                    output.display()
                ),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        PodAction::ExportK8s {
            pod_id,
            volume_size_gb: _,
            max_replicas: _,
            output,
        } => {
            // Validate pod exists before copying manifests
            match get_pod_status(&pod_id).await {
                Ok(_) => match export_k8s(&output) {
                    Ok(count) => {
                        println!(
                            "K8s manifests for '{}' exported to {} ({} files)",
                            pod_id,
                            output.display(),
                            count
                        );
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn build_ctx() -> hkask_services::AgentService {
    super::helpers::build_service_context()
}

fn parse_pod_id(id: &str) -> Result<hkask_agents::pod::PodID, String> {
    uuid::Uuid::parse_str(id)
        .map(hkask_agents::pod::PodID::from_uuid)
        .map_err(|_| format!("Invalid pod ID '{}'", id))
}

pub async fn get_pod_status(pod_id: &str) -> Result<PodStatusInfo, String> {
    let ctx = build_ctx();
    let pid = parse_pod_id(pod_id)?;
    ctx.pod_manager()
        .get_pod_status(&pid)
        .await
        .map_err(|e| format!("Failed to get pod status: {e}"))
}

pub async fn list_pods() -> Result<Vec<PodStatusInfo>, String> {
    let ctx = build_ctx();
    ctx.pod_manager()
        .list_pods()
        .await
        .map_err(|e| format!("Failed to list pods: {e}"))
}

pub async fn create_pod(
    template: &str,
    persona_path: &std::path::PathBuf,
    name: Option<&str>,
) -> Result<String, String> {
    let yaml = std::fs::read_to_string(persona_path)
        .map_err(|e| format!("Cannot read persona file: {e}"))?;
    let persona = hkask_agents::pod::AgentPersona::from_yaml(&yaml)
        .map_err(|e| format!("Invalid persona YAML: {e}"))?;
    let ctx = build_ctx();
    let pm = ctx.pod_manager();
    let pod_id = pm
        .create_pod(
            template,
            &persona,
            name.map(String::from),
            hkask_agents::pod::PodKind::Replicant,
        )
        .await
        .map_err(|e| format!("Failed to create pod: {e}"))?;
    Ok(pod_id.to_string())
}

pub async fn activate_pod(pod_id: &str) -> Result<(), String> {
    let ctx = build_ctx();
    let pid = parse_pod_id(pod_id)?;
    ctx.pod_manager()
        .activate_pod(&pid)
        .await
        .map_err(|e| format!("Failed to activate pod: {e}"))
}

pub async fn deactivate_pod(pod_id: &str) -> Result<(), String> {
    let ctx = build_ctx();
    let pid = parse_pod_id(pod_id)?;
    ctx.pod_manager()
        .deactivate_pod(&pid)
        .await
        .map_err(|e| format!("Failed to deactivate pod: {e}"))
}

pub async fn assign_role(name: &str, role: &str) -> Result<(), String> {
    let ctx = build_ctx();
    ctx.pod_manager()
        .assign_role(name, role)
        .await
        .map_err(|e| format!("Failed to assign role: {e}"))
}

pub async fn set_mode(name: &str, mode: &str, role: Option<&str>) -> Result<(), String> {
    let ctx = build_ctx();
    ctx.pod_manager()
        .set_mode(name, mode, role)
        .await
        .map_err(|e| format!("Failed to set mode: {e}"))
}

/// Export a pod as a container build context (delegates to PodFactory).
pub async fn export_container(pod_id: &str, output_dir: &std::path::Path) -> Result<(), String> {
    let ctx = build_ctx();
    let pm = ctx.pod_manager();
    let pid = hkask_agents::pod::PodID::from_name(pod_id);
    pm.export_container(pid, output_dir)
        .map_err(|e| format!("Failed to export container: {e}"))
}

/// Export K8s manifests from the canonical `deploy/k8s/` directory.
///
/// Copies all files from the repo's `deploy/k8s/` into `output_dir`.
/// The canonical manifests include namespace, deployment (kask + Litestream
/// sidecar), service, PVC, configmap, secret, ingress, and the conduit/
/// subdirectory for the standalone Conduit Matrix homeserver.
/// No inline generation — single source of truth in `deploy/k8s/`.
///
/// Source directory resolution via `helpers::resolve_deploy_dir()`.
///
/// Returns the count of copied files.
pub fn export_k8s(output_dir: &std::path::Path) -> Result<usize, String> {
    let source_dir = crate::commands::helpers::resolve_deploy_dir()?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    let mut copied = 0usize;
    for entry in std::fs::read_dir(&source_dir)
        .map_err(|e| format!("Cannot read deploy/k8s/ ({source_dir:?}): {e}"))?
    {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().ok_or("missing filename")?;
            std::fs::copy(&path, output_dir.join(name))
                .map_err(|e| format!("Failed to copy {name:?}: {e}"))?;
            copied += 1;
        }
    }

    if copied == 0 {
        return Err(format!("No files found in {source_dir:?}"));
    }

    Ok(copied)
}
