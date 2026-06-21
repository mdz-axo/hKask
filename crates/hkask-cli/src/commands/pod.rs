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
/// Create a new replicant pod. Cloud deployment is handled via the K8s export/deploy pipeline.
///
/// Creates a Fly App, volume, secrets, and machine for the replicant.
/// The replicant name becomes the pod identifier (e.g., "alice" → hkask-pod-alice).
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
    PodService::activate_pod(&ctx, pod_id).await?;
    // Best-effort K8s activation
    // cloud activation handled by K8s pipeline
    cloud_activate_k8s(pod_id);
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  pod_id is a valid pod identifier
/// post: returns Ok(()) on successful deactivation
/// post: delegates to PodService::deactivate_pod
pub async fn deactivate_pod(pod_id: &str) -> Result<(), ServiceError> {
    let ctx = super::helpers::build_service_context();
    PodService::deactivate_pod(&ctx, pod_id).await?;
    // Best-effort K8s deactivation
    // cloud deactivation handled by K8s pipeline
    cloud_deactivate_k8s(pod_id);
    Ok(())
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

/// Export a pod as K8s manifests for Hetzner K3s deployment.
/// Writes to output_dir: namespace.yaml, networkpolicy.yaml, statefulset.yaml,
/// configmap.yaml, secrets.yaml, hpa.yaml
pub fn export_k8s(
    pod_id: &str,
    volume_size_gb: u32,
    max_replicas: u32,
    output_dir: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    let namespace = format!("hkask-pod-{pod_id}");
    let container_registry =
        std::env::var("CONTAINER_REGISTRY").unwrap_or_else(|_| "ghcr.io/mdz-axo/hkask".to_string());
    let version = std::env::var("HKASK_VERSION").unwrap_or_else(|_| "0.30.0".to_string());

    // --- namespace.yaml ---
    let namespace_yaml = format!(
        "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {namespace}\n  labels:\n    app: hkask\n    pod-id: \"{pod_id}\"\n"
    );
    std::fs::write(output_dir.join("namespace.yaml"), &namespace_yaml)
        .map_err(|e| format!("Failed to write namespace.yaml: {e}"))?;

    // --- networkpolicy.yaml ---
    let netpol_yaml = format!(
        "apiVersion: networking.k8s.io/v1\nkind: NetworkPolicy\nmetadata:\n  name: pod-isolation\n  namespace: {namespace}\nspec:\n  podSelector: {{}}\n  policyTypes: [Ingress, Egress]\n  ingress:\n    - from:\n        - namespaceSelector:\n            matchLabels:\n              name: hkask-ingress\n      ports:\n        - port: 3000\n          protocol: TCP\n  egress:\n    - to:\n        - namespaceSelector: {{}}\n    - to:\n        - ipBlock:\n            cidr: 0.0.0.0/0\n            except: [10.0.0.0/8]\n      ports:\n        - port: 443\n          protocol: TCP\n        - port: 80\n          protocol: TCP\n"
    );
    std::fs::write(output_dir.join("networkpolicy.yaml"), &netpol_yaml)
        .map_err(|e| format!("Failed to write networkpolicy.yaml: {e}"))?;

    // --- statefulset.yaml ---
    let sts_yaml = format!(
        r#"apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: kask
  namespace: {namespace}
spec:
  serviceName: kask
  replicas: 1
  selector:
    matchLabels:
      app: kask
  template:
    metadata:
      labels:
        app: kask
        pod-id: "{pod_id}"
    spec:
      initContainers:
        - name: litestream-restore
          image: litestream/litestream:0.5.0
          args:
            - restore
            - -if-db-not-exists
            - -if-replica-exists
            - /data/kask.db
          envFrom:
            - secretRef:
                name: litestream-replica
          volumeMounts:
            - name: data
              mountPath: /data
            - name: litestream-config
              mountPath: /etc/litestream.yml
              subPath: litestream.yml
        - name: kask-migrate
          image: {container_registry}:kask-{version}
          command: ["kask", "migrate", "--data-dir", "/data"]
          volumeMounts:
            - name: data
              mountPath: /data
      containers:
        - name: kask
          image: {container_registry}:kask-{version}
          args: ["serve", "--data-dir", "/data", "--pod-id", "{pod_id}"]
          ports:
            - containerPort: 3000
              protocol: TCP
          envFrom:
            - secretRef:
                name: kask-secrets
          volumeMounts:
            - name: data
              mountPath: /data
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
        - name: litestream
          image: litestream/litestream:0.5.0
          args: ["replicate"]
          envFrom:
            - secretRef:
                name: litestream-replica
          volumeMounts:
            - name: data
              mountPath: /data
            - name: litestream-config
              mountPath: /etc/litestream.yml
              subPath: litestream.yml
        - name: conduit
          image: {container_registry}:kask-{version}
          command: ["/usr/local/bin/conduit"]
          env:
            - name: CONDUIT_CONFIG
              value: /etc/conduit/conduit.toml
          volumeMounts:
            - name: data
              mountPath: /data
            - name: conduit-config
              mountPath: /etc/conduit
      volumes:
        - name: litestream-config
          configMap:
            name: litestream-config
        - name: conduit-config
          configMap:
            name: conduit-config
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        storageClassName: local-path
        accessModes: [ReadWriteOnce]
        resources:
          requests:
            storage: {volume_size_gb}Gi
"#
    );
    std::fs::write(output_dir.join("statefulset.yaml"), &sts_yaml)
        .map_err(|e| format!("Failed to write statefulset.yaml: {e}"))?;

    // --- hpa.yaml ---
    let hpa_yaml = format!(
        "apiVersion: autoscaling/v2\nkind: HorizontalPodAutoscaler\nmetadata:\n  name: kask-hpa\n  namespace: {namespace}\nspec:\n  scaleTargetRef:\n    apiVersion: apps/v1\n    kind: StatefulSet\n    name: kask\n  minReplicas: 1\n  maxReplicas: {max_replicas}\n  metrics:\n    - type: Resource\n      resource:\n        name: cpu\n        target:\n          type: Utilization\n          averageUtilization: 70\n  behavior:\n    scaleDown:\n      stabilizationWindowSeconds: 300\n      policies:\n        - type: Percent\n          value: 50\n          periodSeconds: 60\n    scaleUp:\n      stabilizationWindowSeconds: 60\n      policies:\n        - type: Percent\n          value: 100\n          periodSeconds: 30\n"
    );
    std::fs::write(output_dir.join("hpa.yaml"), &hpa_yaml)
        .map_err(|e| format!("Failed to write hpa.yaml: {e}"))?;

    let litestream_endpoint = std::env::var("LITESTREAM_ENDPOINT").unwrap_or_default();
    let litestream_bucket = std::env::var("LITESTREAM_BUCKET").unwrap_or_default();
    let litestream_access_key = std::env::var("LITESTREAM_ACCESS_KEY_ID").unwrap_or_default();
    let litestream_secret_key = std::env::var("LITESTREAM_SECRET_ACCESS_KEY").unwrap_or_default();

    // --- configmap.yaml (litestream + conduit) ---
    let cm_yaml = format!(
        r#"apiVersion: v1
kind: ConfigMap
metadata:
  name: litestream-config
  namespace: {namespace}
data:
  litestream.yml: |
    addr: ":9090"
    sync-interval: 1s
    snapshot-interval: 6h
    dbs:
      - path: /data/kask.db
        replicas:
          - type: s3
            bucket: {litestream_bucket}
            path: pods/{pod_id}/kask.db
            endpoint: {litestream_endpoint}
            region: auto
            access-key-id: {litestream_access_key}
            secret-access-key: {litestream_secret_key}
            force-path-style: true
      - path: /data/conduit.db
        replicas:
          - type: s3
            bucket: {litestream_bucket}
            path: pods/{pod_id}/conduit.db
            endpoint: {litestream_endpoint}
            region: auto
            access-key-id: {litestream_access_key}
            secret-access-key: {litestream_secret_key}
            force-path-style: true
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: conduit-config
  namespace: {namespace}
data:
  conduit.toml: |
    [global]
    server_name = "{pod_id}.hkask.local"
    address = "0.0.0.0"
    port = 8008
    [global.federation]
    enabled = true
    address = "0.0.0.0"
    port = 8448
    [global.database]
    backend = "sqlite"
    path = "/data/conduit.db"
    [global.registration]
    enabled = false
    [global.allow_federation]
    servers = ["*.hkask.local"]
"#
    );
    std::fs::write(output_dir.join("configmap.yaml"), &cm_yaml)
        .map_err(|e| format!("Failed to write configmap.yaml: {e}"))?;

    // --- secrets.yaml ---
    let litestream_access_key = std::env::var("LITESTREAM_ACCESS_KEY_ID").unwrap_or_default();
    let litestream_secret_key = std::env::var("LITESTREAM_SECRET_ACCESS_KEY").unwrap_or_default();
    let keystore_passphrase = std::env::var("HKASK_KEYSTORE_PASSPHRASE").unwrap_or_default();
    let base_url = std::env::var("HKASK_BASE_URL").unwrap_or_default();

    let secrets_yaml = format!(
        r#"apiVersion: v1
kind: Secret
metadata:
  name: litestream-replica
  namespace: {namespace}
stringData:
  LITESTREAM_BUCKET: "{litestream_bucket}"
  LITESTREAM_ENDPOINT: "{litestream_endpoint}"
  LITESTREAM_REGION: "auto"
  LITESTREAM_ACCESS_KEY_ID: "{litestream_access_key}"
  LITESTREAM_SECRET_ACCESS_KEY: "{litestream_secret_key}"
  LITESTREAM_FORCE_PATH_STYLE: "true"
---
apiVersion: v1
kind: Secret
metadata:
  name: kask-secrets
  namespace: {namespace}
stringData:
  POD_ID: "{pod_id}"
  HKASK_DATA_DIR: "/data"
  HKASK_BASE_URL: "{base_url}"
  HKASK_KEYSTORE_PASSPHRASE: "{keystore_passphrase}"
"#
    );
    std::fs::write(output_dir.join("secrets.yaml"), &secrets_yaml)
        .map_err(|e| format!("Failed to write secrets.yaml: {e}"))?;

    Ok(())
}

/// Try to activate a pod on Hetzner K3s by applying its manifests.
/// No-op if kubectl is not available or KUBECONFIG is not set.
fn cloud_activate_k8s(pod_id: &str) {
    // Check kubectl availability
    if std::process::Command::new("kubectl")
        .arg("version")
        .arg("--client")
        .output()
        .is_err()
    {
        return;
    }

    let output_dir = std::env::temp_dir().join(format!("hkask-k8s-{pod_id}"));

    // Generate manifests
    if let Err(e) = export_k8s(pod_id, 10, 3, &output_dir) {
        tracing::warn!(target: "cns.cloud", pod_id = %pod_id, error = %e, "Failed to generate K8s manifests");
        return;
    }

    // Apply
    match std::process::Command::new("kubectl")
        .arg("apply")
        .arg("-f")
        .arg(&output_dir)
        .output()
    {
        Ok(out) if out.status.success() => {
            tracing::info!(target: "cns.cloud", pod_id = %pod_id, "K8s manifests applied");
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            tracing::warn!(target: "cns.cloud", pod_id = %pod_id, error = %stderr, "kubectl apply failed");
        }
        Err(e) => {
            tracing::warn!(target: "cns.cloud", pod_id = %pod_id, error = %e, "Failed to run kubectl");
        }
    }

    // Clean up temp manifests
    let _ = std::fs::remove_dir_all(&output_dir);
}

/// Try to deactivate a pod on Hetzner K3s by scaling its StatefulSet to zero.
fn cloud_deactivate_k8s(pod_id: &str) {
    if std::process::Command::new("kubectl")
        .arg("version")
        .arg("--client")
        .output()
        .is_err()
    {
        return;
    }

    let namespace = format!("hkask-pod-{pod_id}");

    match std::process::Command::new("kubectl")
        .arg("scale")
        .arg("statefulset")
        .arg("kask")
        .arg("--replicas=0")
        .arg("-n")
        .arg(&namespace)
        .output()
    {
        Ok(out) if out.status.success() => {
            tracing::info!(target: "cns.cloud", pod_id = %pod_id, "K8s StatefulSet scaled to zero");
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            tracing::warn!(target: "cns.cloud", pod_id = %pod_id, error = %stderr, "kubectl scale failed");
        }
        Err(e) => {
            tracing::warn!(target: "cns.cloud", pod_id = %pod_id, error = %e, "Failed to run kubectl");
        }
    }
}

/// Try to activate a pod on Hetzner K3s by applying its manifests.
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
            println!("Pod deployed: {}", pod_id);
            println!("URL: https://hkask-pod-{}.example.com", pod_id);
            println!("Template: {}", template);
            println!("Persona file: {}", persona.display());
            if let Some(n) = &name {
                println!("Replicant: {}", n);
            }
        }
        PodAction::Activate { pod_id } => {
            crate::block_on!(
                rt,
                commands::activate_pod(&pod_id),
                "Failed to activate pod"
            );
            println!("Pod activated: {}", pod_id);
        }
        PodAction::Deactivate { pod_id } => {
            crate::block_on!(
                rt,
                commands::deactivate_pod(&pod_id),
                "Failed to deactivate pod"
            );
            println!("Pod deactivated: {}", pod_id);
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
        PodAction::ExportK8s {
            pod_id,
            volume_size_gb,
            max_replicas,
            output,
        } => match export_k8s(&pod_id, volume_size_gb, max_replicas, &output) {
            Ok(()) => {
                println!("K8s manifests exported: {}", pod_id);
                for f in &[
                    "namespace.yaml",
                    "networkpolicy.yaml",
                    "statefulset.yaml",
                    "hpa.yaml",
                    "configmap.yaml",
                    "secrets.yaml",
                ] {
                    println!("  {}/{f}", output.display());
                }
                println!();
                println!("Next steps:");
                println!("  1. kubectl apply -f {}/", output.display());
                println!("  2. kubectl get pods -n hkask-pod-{pod_id}",);
            }
            Err(e) => eprintln!("K8s export failed: {e}"),
        },
    }
}
