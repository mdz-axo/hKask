//! Pod management command handlers — delegates to PodService.

use hkask_services::{PodService, PodStatusResponse, ServiceError};

use crate::cli::PodAction;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  pod_id is a valid pod identifier
/// post: returns Ok(PodStatusResponse) with pod status
/// post: delegates to PodService::get_pod_status
pub async fn get_pod_status(pod_id: &str) -> Result<PodStatusResponse, ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::get_pod_status(&ctx, pod_id).await
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  none
/// post: returns Ok(`Vec<PodStatusResponse>`) with all pod statuses
/// post: delegates to PodService::list_pods
pub async fn list_pods() -> Result<Vec<PodStatusResponse>, ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::list_pods(&ctx).await
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  template is a valid template ID
/// pre:  persona_path points to a readable YAML file
/// post: returns Ok(String) with the created pod ID
/// post: if persona file unreadable → Err(ServiceError::Infra)
/// post: delegates to PodService::create_pod
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

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  pod_id is a valid pod identifier
/// post: returns Ok(()) on successful activation
/// post: delegates to PodService::activate_pod
pub async fn activate_pod(pod_id: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::activate_pod(&ctx, pod_id).await
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  pod_id is a valid pod identifier
/// post: returns Ok(()) on successful deactivation
/// post: delegates to PodService::deactivate_pod
pub async fn deactivate_pod(pod_id: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::deactivate_pod(&ctx, pod_id).await
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  name is a valid pod name
/// pre:  role is a valid role identifier
/// post: returns Ok(()) on successful role assignment
/// post: delegates to PodService::assign_role
pub async fn assign_role(name: &str, role: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::assign_role(&ctx, name, role).await
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  name is a valid pod name
/// pre:  mode is a valid mode identifier
/// post: returns Ok(()) on successful mode change
/// post: delegates to PodService::set_mode
pub async fn set_mode(name: &str, mode: &str, role: Option<&str>) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::set_mode(&ctx, name, mode, role).await
}

/// Export a pod as a container image build context.
/// Produces Containerfile + pod files in output_dir. After export:
///   docker build -t hkask-pod-{pod_id} {output_dir}
pub async fn export_container(
    pod_id: &str,
    output_dir: &std::path::Path,
) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    let pm = ctx.pod_manager();
    let pid = hkask_agents::pod::PodID::from_name(pod_id);
    pm.export_container(pid, output_dir)
        .map_err(|e| ServiceError::Pod {
            message: e.to_string(),
        })
}

/// Export a pod as a Fly.io deployment context (fly.toml + secrets script).
/// Writes to output_dir:
///   fly.toml        — Fly.io app configuration
///   fly-secrets.sh  — secrets to set via `fly secrets set`
pub async fn export_fly(
    pod_id: &str,
    region: &str,
    volume_size_gb: u32,
    output_dir: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    let app_name = format!("hkask-pod-{pod_id}");
    let container_registry =
        std::env::var("CONTAINER_REGISTRY").unwrap_or_else(|_| "ghcr.io/mdz-axo/hkask".to_string());
    let version = std::env::var("HKASK_VERSION").unwrap_or_else(|_| "0.30.0".to_string());
    let base_url =
        std::env::var("HKASK_BASE_URL").unwrap_or_else(|_| format!("https://{app_name}.fly.dev"));

    // --- fly.toml ---
    let fly_toml = format!(
        r#"app = "{app_name}"
primary_region = "{region}"

[build]
  image = "{container_registry}:kask-{version}"

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 768

[mounts]
  source = "hkask_data"
  destination = "/data"
  initial_size = "{volume_size_gb}gb"
  auto_extend_size_increment = "1gb"
  auto_extend_size_limit = "10gb"

[[services]]
  protocol = "tcp"
  internal_port = 3000

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.ports]]
    port = 80
    handlers = ["http"]
    force_https = true

  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[[services]]
  protocol = "tcp"
  internal_port = 8448

  [[services.ports]]
    port = 8448
    handlers = ["tls"]

  auto_stop_machines = false

[experimental]
  auto_rollback = true

[deploy]
  release_command = "kask migrate --data-dir /data"

[env]
  HKASK_DATA_DIR = "/data"
  POD_ID = "{pod_id}"
  HKASK_BASE_URL = "{base_url}"
"#
    );

    std::fs::write(output_dir.join("fly.toml"), &fly_toml)
        .map_err(|e| format!("Failed to write fly.toml: {e}"))?;

    // --- fly-secrets.sh ---
    let litestream_bucket = std::env::var("LITESTREAM_BUCKET").unwrap_or_default();
    let litestream_endpoint = std::env::var("LITESTREAM_ENDPOINT").unwrap_or_default();
    let litestream_region =
        std::env::var("LITESTREAM_REGION").unwrap_or_else(|_| "auto".to_string());
    let litestream_access_key = std::env::var("LITESTREAM_ACCESS_KEY_ID").unwrap_or_default();
    let litestream_secret_key = std::env::var("LITESTREAM_SECRET_ACCESS_KEY").unwrap_or_default();
    let litestream_force_path =
        std::env::var("LITESTREAM_FORCE_PATH_STYLE").unwrap_or_else(|_| "false".to_string());
    let keystore_passphrase = std::env::var("HKASK_KEYSTORE_PASSPHRASE").unwrap_or_default();

    let secrets_script = format!(
        r#"#!/bin/bash
# Generated by: kask pod export fly {pod_id}
# Run once before first deploy: source fly-secrets.sh
# WARNING: Contains secrets. Never commit to version control.

fly secrets set \
  LITESTREAM_BUCKET="{litestream_bucket}" \
  LITESTREAM_ENDPOINT="{litestream_endpoint}" \
  LITESTREAM_REGION="{litestream_region}" \
  LITESTREAM_ACCESS_KEY_ID="{litestream_access_key}" \
  LITESTREAM_SECRET_ACCESS_KEY="{litestream_secret_key}" \
  LITESTREAM_FORCE_PATH_STYLE="{litestream_force_path}" \
  POD_ID="{pod_id}" \
  HKASK_KEYSTORE_PASSPHRASE="{keystore_passphrase}"
"#
    );

    let secrets_path = output_dir.join("fly-secrets.sh");
    std::fs::write(&secrets_path, &secrets_script)
        .map_err(|e| format!("Failed to write fly-secrets.sh: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&secrets_path)
            .map_err(|e| format!("Failed to read secrets file metadata: {e}"))?
            .permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(&secrets_path, perms)
            .map_err(|e| format!("Failed to set secrets file permissions: {e}"))?;
    }

    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio runtime
/// pre:  action is a valid PodAction variant
/// post: dispatches to the appropriate pod command handler
/// post: prints result or error to stdout
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
        PodAction::ExportContainer { pod_id, output } => {
            crate::block_on!(
                rt,
                commands::export_container(&pod_id, &output),
                "Failed to export pod container"
            );
            println!("Pod container exported: {}", pod_id);
            println!("Build context: {}", output.display());
            println!(
                "Run: docker build -t hkask-pod-{} {}",
                pod_id,
                output.display()
            );
        }
        PodAction::ExportFly {
            pod_id,
            region,
            volume_size_gb,
            output,
        } => match rt.block_on(export_fly(&pod_id, &region, volume_size_gb, &output)) {
            Ok(()) => {
                println!("Fly.io deployment exported: {}", pod_id);
                println!("  fly.toml:        {}/fly.toml", output.display());
                println!("  fly-secrets.sh:  {}/fly-secrets.sh", output.display());
                println!();
                println!("Next steps:");
                println!("  1. source {}/fly-secrets.sh", output.display());
                println!("  2. fly deploy --config {}/fly.toml", output.display());
            }
            Err(e) => eprintln!("Fly export failed: {e}"),
        },
    }
}
