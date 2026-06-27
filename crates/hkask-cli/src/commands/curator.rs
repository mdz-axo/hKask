//! Curator commands — delegates to CuratorService.

use hkask_services::{CuratorService, ServiceError};
use hkask_storage::EscalationEntry;

use crate::block_on;
use crate::cli::CuratorAction;

/// Initialize the hKask system on a Hetzner K3s cluster.
///
/// This is a one-time setup command. It:
///   1. Validates all required env vars (HCLOUD_TOKEN, CONTAINER_REGISTRY, LITESTREAM_*, etc.)
///   2. Confirms kubectl access to the K3s cluster
///   3. Validates the Hetzner API token and object storage credentials
///   4. Generates a Conduit Matrix signing key (Ed25519)
///   5. Deploys the shared Conduit Matrix homeserver to the cluster
///   6. Creates the Curator pod and deploys it
///   7. Prints the Matrix URL and Curator URL
pub async fn curator_init(domain: &str) -> Result<(), String> {
    // ── 1. Validate environment ──────────────────────────────────────────
    let hcloud_token = std::env::var("HCLOUD_TOKEN")
        .map_err(|_| "HCLOUD_TOKEN not set. Set your Hetzner Cloud API token.".to_string())?;
    if hcloud_token.is_empty() {
        return Err("HCLOUD_TOKEN is empty.".to_string());
    }

    let _container_registry = std::env::var("CONTAINER_REGISTRY")
        .map_err(|_| "CONTAINER_REGISTRY not set (e.g., ghcr.io/your-org/hkask).".to_string())?;
    let _version = std::env::var("HKASK_VERSION").unwrap_or_else(|_| "0.30.0".to_string());

    let litestream_bucket =
        std::env::var("LITESTREAM_BUCKET").map_err(|_| "LITESTREAM_BUCKET not set.".to_string())?;
    let litestream_endpoint = std::env::var("LITESTREAM_ENDPOINT")
        .map_err(|_| "LITESTREAM_ENDPOINT not set.".to_string())?;
    let litestream_access_key = std::env::var("LITESTREAM_ACCESS_KEY_ID")
        .map_err(|_| "LITESTREAM_ACCESS_KEY_ID not set.".to_string())?;
    let litestream_secret_key = std::env::var("LITESTREAM_SECRET_ACCESS_KEY")
        .map_err(|_| "LITESTREAM_SECRET_ACCESS_KEY not set.".to_string())?;

    let _base_url = std::env::var("HKASK_BASE_URL")
        .map_err(|_| "HKASK_BASE_URL not set (e.g., https://hkask.example.com).".to_string())?;

    let _keystore_passphrase = std::env::var("HKASK_KEYSTORE_PASSPHRASE").unwrap_or_default();

    // ── 2. Check kubectl availability ────────────────────────────────────
    let kubectl_ok = std::process::Command::new("kubectl")
        .arg("version")
        .arg("--client")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !kubectl_ok {
        return Err(
            "kubectl not found. Install it and ensure KUBECONFIG points to your K3s cluster."
                .to_string(),
        );
    }

    // ── 3. Validate cloud provider access ─────────────────────────────────
    println!("=== hKask Curator Init ===");
    println!("Domain: {domain}");
    println!();

    let hetzner = hkask_services::cloud::hetzner::HetznerClient::new(hcloud_token.clone());
    println!("Validating Hetzner API token...");
    hetzner.validate_token().await.map_err(|e| {
        format!("Hetzner API token validation failed: {e}\nCheck HCLOUD_TOKEN in your .env file.")
    })?;
    println!("  Hetzner API: OK");

    println!("Validating object storage...");
    hkask_services::cloud::hetzner::validate_object_storage(
        &litestream_endpoint,
        &litestream_bucket,
        &litestream_access_key,
        &litestream_secret_key,
    )
    .await
    .map_err(|e| {
        format!("Object storage validation failed: {e}\nCheck LITESTREAM_* vars in your .env file.")
    })?;
    println!("  Object storage: OK");
    println!();

    // ── 4. Generate Conduit signing key ──────────────────────────────────
    println!("Generating Conduit signing key...");
    use rand::RngCore;
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    let signing_key = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(key_bytes)
    };
    println!("  Signing key generated.");

    // ── 5. Deploy shared Conduit ─────────────────────────────────────────
    let conduit_ns = "hkask-conduit";
    println!("Deploying shared Conduit (namespace: {conduit_ns})...");

    // Create namespace
    let ns_yaml = format!(
        "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {conduit_ns}\n  labels:\n    app: hkask\n    component: conduit\n"
    );
    kubectl_apply_stdin(&ns_yaml)
        .map_err(|e| format!("Failed to create Conduit namespace: {e}"))?;

    // Create PVC for conduit.db
    let pvc_yaml = format!(
        "apiVersion: v1\nkind: PersistentVolumeClaim\nmetadata:\n  name: conduit-data\n  namespace: {conduit_ns}\nspec:\n  storageClassName: hcloud-volumes\n  accessModes: [ReadWriteOnce]\n  resources:\n    requests:\n      storage: 10Gi\n"
    );
    kubectl_apply_stdin(&pvc_yaml).map_err(|e| format!("Failed to create Conduit PVC: {e}"))?;

    // Create Secret for signing key
    let conduit_server_name = domain.to_string();
    let secret_yaml = format!(
        "apiVersion: v1\nkind: Secret\nmetadata:\n  name: conduit-secrets\n  namespace: {conduit_ns}\nstringData:\n  CONDUIT_SERVER_NAME: \"{conduit_server_name}\"\n  CONDUIT_SIGNING_KEY: \"{signing_key}\"\n  CONDUIT_PORT: \"8008\"\n  CONDUIT_DATABASE_PATH: \"/data/conduit.db\"\n  CONDUIT_ALLOW_REGISTRATION: \"false\"\n"
    );
    kubectl_apply_stdin(&secret_yaml)
        .map_err(|e| format!("Failed to create Conduit secret: {e}"))?;

    // Create Service
    let svc_yaml = format!(
        "apiVersion: v1\nkind: Service\nmetadata:\n  name: conduit\n  namespace: {conduit_ns}\nspec:\n  selector:\n    app: conduit\n  ports:\n    - name: matrix\n      port: 8008\n      targetPort: 8008\n      protocol: TCP\n"
    );
    kubectl_apply_stdin(&svc_yaml).map_err(|e| format!("Failed to create Conduit service: {e}"))?;

    // Create Deployment
    let deployment_yaml = format!(
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: conduit
  namespace: {conduit_ns}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: conduit
  template:
    metadata:
      labels:
        app: conduit
    spec:
      containers:
        - name: conduit
          image: registry.gitlab.com/famedly/conduit:latest
          ports:
            - containerPort: 8008
              protocol: TCP
          envFrom:
            - secretRef:
                name: conduit-secrets
          volumeMounts:
            - name: data
              mountPath: /data
          resources:
            requests:
              cpu: 50m
              memory: 64Mi
            limits:
              cpu: 200m
              memory: 256Mi
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: conduit-data
"#
    );
    kubectl_apply_stdin(&deployment_yaml)
        .map_err(|e| format!("Failed to create Conduit deployment: {e}"))?;

    let matrix_url = format!("http://conduit.{conduit_ns}.svc.cluster.local:8008");
    println!("  Conduit deployed. Matrix URL: {matrix_url}");
    println!();

    // ── 6. Store signing key in Curator keystore ─────────────────────────
    // TODO: wire to hkask-keystore when keystore API is available
    println!("  Signing key stored in memory (keystore integration pending).");

    // ── 7. Deploy Curator pod ────────────────────────────────────────────
    println!("Deploying Curator pod...");

    // Generate Curator K8s manifests using export_k8s
    let curator_dir = std::env::temp_dir().join("hkask-curator-init");
    let _ = std::fs::create_dir_all(&curator_dir);

    crate::commands::pod::export_k8s("curator", 10, 3, &curator_dir)
        .map_err(|e| format!("Failed to generate Curator manifests: {e}"))?;

    // Run kubectl apply on the generated manifests
    let apply_output = std::process::Command::new("kubectl")
        .arg("apply")
        .arg("-f")
        .arg(&curator_dir)
        .output()
        .map_err(|e| format!("Failed to run kubectl apply: {e}"))?;

    if !apply_output.status.success() {
        let stderr = String::from_utf8_lossy(&apply_output.stderr);
        return Err(format!("kubectl apply failed:\n{stderr}"));
    }

    // Clean up temp manifests
    let _ = std::fs::remove_dir_all(&curator_dir);

    let curator_url = format!("https://{domain}");

    println!();
    println!("=== hKask system initialized ===");
    println!("Curator pod:    {curator_url}");
    println!("Conduit server:  {matrix_url}");
    println!();
    println!("Create a user pod:");
    println!("  kask pod create alice");
    println!();

    Ok(())
}

/// Apply a YAML string via `kubectl apply -f -` (stdin).
fn kubectl_apply_stdin(yaml: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = std::process::Command::new("kubectl")
        .arg("apply")
        .arg("-f")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn kubectl: {e}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(yaml.as_bytes())
            .map_err(|e| format!("Failed to write to kubectl stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait on kubectl: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("kubectl apply failed:\n{stderr}"));
    }

    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  none
/// post: returns Ok(`Vec<EscalationEntry>`) with all pending escalations
/// post: delegates to escalation_queue.list_pending()
pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    let queue = ctx.escalation_queue();
    queue.list_pending().map_err(|e| ServiceError::Escalation {
        message: e.to_string(),
    })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  id is a valid escalation identifier
/// post: returns Ok(()) if escalation resolved successfully
/// post: delegates to CuratorService::resolve
pub async fn curator_resolve(id: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    CuratorService::resolve(&ctx, id, "cli-administrator")
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  id is a valid escalation identifier
/// post: returns Ok(()) if dismissal dismissed successfully
/// post: delegates to CuratorService::dismiss
pub async fn curator_dismiss(id: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    CuratorService::dismiss(&ctx, id, "cli-administrator")
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  none
/// post: returns Ok(String) with metacognition report
/// post: delegates to CuratorService::metacognition
pub async fn curator_metacognition() -> Result<String, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    CuratorService::metacognition(&ctx).await
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio runtime
/// pre:  registry, runtime, handle are valid
/// pre:  action is a valid CuratorAction variant
/// post: dispatches to chat/escalations/resolve/dismiss/metacognition
/// post: prints result or error to stdout
pub fn run_curator(
    rt: &tokio::runtime::Runtime,
    registry: &mut hkask_templates::SqliteRegistry,
    runtime: &hkask_mcp::runtime::McpRuntime,
    handle: &tokio::runtime::Handle,
    action: crate::cli::CuratorAction,
) {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "curator", action = ?action, "CNS");
    use crate::commands;

    match action {
        CuratorAction::Chat => {
            crate::repl::run(registry, runtime, None, "Curator", None, handle.clone());
        }
        CuratorAction::Escalations => {
            let escalations = block_on!(
                rt,
                commands::curator_escalations(),
                "Failed to list escalations"
            );
            if escalations.is_empty() {
                println!("No pending escalations.");
            } else {
                println!("{:<20} {:<15} {:<10} CONTEXT", "ID", "BOT", "CONFIDENCE");
                println!("{}", "-".repeat(80));
                for esc in &escalations {
                    println!(
                        "{:<20} {:<15} {:<10.2} {}",
                        &esc.id.to_string()[..std::cmp::min(20, esc.id.to_string().len())],
                        esc.bot_id
                            .as_uuid()
                            .to_string()
                            .split('-')
                            .next()
                            .unwrap_or("unknown"),
                        esc.confidence,
                        &esc.error_context[..std::cmp::min(40, esc.error_context.len())],
                    );
                }
                println!("\nTotal: {} pending escalations", escalations.len());
            }
        }
        CuratorAction::Resolve { id } => {
            block_on!(
                rt,
                commands::curator_resolve(&id),
                "Failed to resolve escalation"
            );
            println!("Escalation {} resolved.", id);
        }
        CuratorAction::Dismiss { id } => {
            block_on!(
                rt,
                commands::curator_dismiss(&id),
                "Failed to dismiss escalation"
            );
            println!("Escalation {} dismissed.", id);
        }
        CuratorAction::Metacognition => {
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::curator_metacognition(),
                    "Metacognition cycle failed"
                )
            );
        }
        CuratorAction::Init { domain } => match rt.block_on(curator_init(&domain)) {
            Ok(()) => {}
            Err(e) => eprintln!("Curator init failed: {e}"),
        },
    }
}
