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
            volume_size_gb,
            max_replicas,
            output,
        } => match export_k8s(&pod_id, volume_size_gb, max_replicas, &output).await {
            Ok(manifests) => {
                println!(
                    "K8s manifests for '{}' exported to {} ({} files)",
                    pod_id,
                    output.display(),
                    manifests
                );
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
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

/// Export a pod as a container build context (preserved for curator.rs compatibility).
pub async fn export_container(pod_id: &str, output_dir: &std::path::Path) -> Result<(), String> {
    let ctx = build_ctx();
    let pm = ctx.pod_manager();
    let pid = hkask_agents::pod::PodID::from_name(pod_id);
    pm.export_container(pid, output_dir)
        .map_err(|e| format!("Failed to export container: {e}"))
}

/// Export K8s manifests for Hetzner K3s deployment.
///
/// Generates standard Kubernetes resources in `output_dir`:
/// - `namespace.yaml` — pod-scoped namespace
/// - `deployment.yaml` — Deployment with `max_replicas`, resource limits, and pod anti-affinity
/// - `service.yaml` — ClusterIP Service
/// - `pvc.yaml` — PersistentVolumeClaim with requested volume size
///
/// Returns the count of generated manifest files.
pub async fn export_k8s(
    pod_id: &str,
    volume_size_gb: u32,
    max_replicas: u32,
    output_dir: &std::path::Path,
) -> Result<usize, String> {
    // Validate pod exists before generating manifests
    let ctx = build_ctx();
    let pid = parse_pod_id(pod_id)?;
    ctx.pod_manager()
        .get_pod_status(&pid)
        .await
        .map_err(|e| format!("Pod '{}' not found: {}", pod_id, e))?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let ns_name = format!("hkask-pod-{}", pod_id);
    let app_label = format!("hkask-pod-{}", pod_id);

    // ── namespace.yaml ────────────────────────────────────────────────
    let namespace_yaml = format!(
        "# K8s Namespace — hKask pod '{}'\n\
         # Generated by hKask v0.31.0\n\
         apiVersion: v1\n\
         kind: Namespace\n\
         metadata:\n\
           name: {}\n\
           labels:\n\
             app.kubernetes.io/name: hkask\n\
             app.kubernetes.io/instance: pod-{}\n",
        pod_id, ns_name, pod_id
    );
    std::fs::write(output_dir.join("namespace.yaml"), &namespace_yaml)
        .map_err(|e| format!("Failed to write namespace.yaml: {}", e))?;

    // ── deployment.yaml ───────────────────────────────────────────────
    let deployment_yaml = format!(
        "# K8s Deployment — hKask pod '{}'\n\
         # Generated by hKask v0.31.0\n\
         apiVersion: apps/v1\n\
         kind: Deployment\n\
         metadata:\n\
           name: {}\n\
           namespace: {}\n\
           labels:\n\
             app: {}\n\
         spec:\n\
           replicas: {}\n\
           selector:\n\
             matchLabels:\n\
               app: {}\n\
           template:\n\
             metadata:\n\
               labels:\n\
                 app: {}\n\
             spec:\n\
               affinity:\n\
                 podAntiAffinity:\n\
                   preferredDuringSchedulingIgnoredDuringExecution:\n\
                     - weight: 100\n\
                       podAffinityTerm:\n\
                         labelSelector:\n\
                           matchExpressions:\n\
                             - key: app\n\
                               operator: In\n\
                               values:\n\
                                 - {}\n\
                         topologyKey: kubernetes.io/hostname\n\
               containers:\n\
                 - name: hkask\n\
                   image: hkask-runtime:0.31.0\n\
                   imagePullPolicy: IfNotPresent\n\
                   args:\n\
                     - pod\n\
                     - serve\n\
                     - --pod-id\n\
                     - {}\n\
                   env:\n\
                     - name: HKASK_POD_ID\n\
                       value: \"{}\"\n\
                     - name: HKASK_POD_MODE\n\
                       value: \"server\"\n\
                     - name: RUST_LOG\n\
                       value: \"info,hkask=debug\"\n\
                   ports:\n\
                     - containerPort: 3000\n\
                       name: http\n\
                   resources:\n\
                     requests:\n\
                       memory: \"256Mi\"\n\
                       cpu: \"250m\"\n\
                     limits:\n\
                       memory: \"1Gi\"\n\
                       cpu: \"1000m\"\n\
                   volumeMounts:\n\
                     - name: data\n\
                       mountPath: /data\n\
                   livenessProbe:\n\
                     httpGet:\n\
                       path: /health\n\
                       port: 3000\n\
                     initialDelaySeconds: 30\n\
                     periodSeconds: 30\n\
                   readinessProbe:\n\
                     httpGet:\n\
                       path: /health\n\
                       port: 3000\n\
                     initialDelaySeconds: 5\n\
                     periodSeconds: 10\n\
               volumes:\n\
                 - name: data\n\
                   persistentVolumeClaim:\n\
                     claimName: {}-data\n",
        pod_id,
        app_label,
        ns_name,
        app_label,
        max_replicas,
        app_label,
        app_label,
        app_label,
        pod_id,
        pod_id,
        app_label
    );
    std::fs::write(output_dir.join("deployment.yaml"), &deployment_yaml)
        .map_err(|e| format!("Failed to write deployment.yaml: {}", e))?;

    // ── service.yaml ──────────────────────────────────────────────────
    let service_yaml = format!(
        "# K8s Service — hKask pod '{}'\n\
         # Generated by hKask v0.31.0\n\
         apiVersion: v1\n\
         kind: Service\n\
         metadata:\n\
           name: {}\n\
           namespace: {}\n\
           labels:\n\
             app: {}\n\
         spec:\n\
           type: ClusterIP\n\
           selector:\n\
             app: {}\n\
           ports:\n\
             - name: http\n\
               port: 3000\n\
               targetPort: 3000\n\
               protocol: TCP\n",
        pod_id, app_label, ns_name, app_label, app_label
    );
    std::fs::write(output_dir.join("service.yaml"), &service_yaml)
        .map_err(|e| format!("Failed to write service.yaml: {}", e))?;

    // ── pvc.yaml ──────────────────────────────────────────────────────
    let pvc_yaml = format!(
        "# K8s PersistentVolumeClaim — hKask pod '{}'\n\
         # Generated by hKask v0.31.0\n\
         apiVersion: v1\n\
         kind: PersistentVolumeClaim\n\
         metadata:\n\
           name: {}-data\n\
           namespace: {}\n\
           labels:\n\
             app: {}\n\
         spec:\n\
           accessModes:\n\
             - ReadWriteOnce\n\
           resources:\n\
             requests:\n\
               storage: {}Gi\n",
        pod_id, app_label, ns_name, app_label, volume_size_gb
    );
    std::fs::write(output_dir.join("pvc.yaml"), &pvc_yaml)
        .map_err(|e| format!("Failed to write pvc.yaml: {}", e))?;

    Ok(4)
}
