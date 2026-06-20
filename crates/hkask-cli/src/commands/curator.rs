//! Curator commands — delegates to CuratorService.

use hkask_services::{CuratorService, ServiceError};
use hkask_storage::EscalationEntry;

use crate::block_on;
use crate::cli::CuratorAction;
use crate::cloud::fly::FlyClient;

/// Initialize the hKask system.
///
/// Deploys the shared Conduit Matrix homeserver as a Fly App.
/// Generates Conduit signing key and stores it in the Curator's keystore.
/// This is a one-time setup — run once per hKask installation.
pub async fn curator_init(domain: &str) -> Result<(), String> {
    let token = std::env::var("FLY_API_TOKEN").map_err(|_| {
        "FLY_API_TOKEN not set. Run: fly tokens create org -o <org> -n hkask-curator".to_string()
    })?;
    let org_slug = std::env::var("FLY_ORG_SLUG").unwrap_or_else(|_| "personal".to_string());

    let fly = FlyClient::new(token.clone());
    let app_name = "hkask-conduit";

    println!("=== hKask Curator Init ===");
    println!("Domain: {domain}");
    println!("Organization: {org_slug}");
    println!();

    // 1. Create the Conduit Fly App
    println!("Creating Fly App '{app_name}'...");
    let app = fly.create_app(app_name, &org_slug).await?;
    println!("  Created: {}", app.name);

    // 2. Create persistent volume for Conduit's SQLite database
    println!("Creating volume (1GB)...");
    let volume = fly
        .create_volume(app_name, "conduit_data", "iad", 1)
        .await?;
    println!("  Volume: {} ({})", volume.name, volume.id);

    // 3. Generate Ed25519 signing key for Conduit Matrix federation
    println!("Generating Conduit signing key...");
    let signing_key = generate_conduit_signing_key();
    println!("  Signing key generated.");

    // 4. Set Fly Secrets
    let mut secrets = std::collections::HashMap::new();
    secrets.insert("CONDUIT_SERVER_NAME".to_string(), app_name.to_string());
    secrets.insert("CONDUIT_SIGNING_KEY".to_string(), signing_key.clone());
    secrets.insert("CONDUIT_PORT".to_string(), "8008".to_string());
    secrets.insert(
        "CONDUIT_DATABASE_PATH".to_string(),
        "/data/conduit.db".to_string(),
    );
    secrets.insert(
        "CONDUIT_ALLOW_REGISTRATION".to_string(),
        "false".to_string(),
    );

    println!("Setting secrets...");
    fly.set_secrets(app_name, &secrets).await?;
    println!("  Secrets set.");

    // 5. Deploy Conduit Machine using official image
    println!("Deploying Conduit Machine...");
    let machine_config = crate::cloud::fly::MachineConfig {
        name: "conduit".to_string(),
        region: "iad".to_string(),
        config: crate::cloud::fly::MachineSpec {
            image: "registry.gitlab.com/famedly/conduit:latest".to_string(),
            env: Some(secrets),
            mounts: Some(vec![crate::cloud::fly::MachineMount {
                volume: "conduit_data".to_string(),
                path: "/data".to_string(),
            }]),
            services: None,
            guest: crate::cloud::fly::MachineGuest {
                cpu_kind: "shared".to_string(),
                cpus: 1,
                memory_mb: 256,
            },
        },
    };
    let machine = fly.create_machine(app_name, &machine_config).await?;
    println!("  Machine: {} ({})", machine.name, machine.id);

    // 6. Store signing key in Curator's keystore
    // TODO: wire to hkask-keystore when keystore API is available
    println!("  Signing key stored in memory (keystore integration pending).");

    println!();
    println!("=== Conduit deployed ===");
    println!("Matrix URL: http://{app_name}.internal:8008");
    println!("Public URL: https://{app_name}.fly.dev");
    println!();

    // 6. Create the Curator pod (master replicant)
    let curator_app = "hkask-pod-curator";
    println!("Creating Curator pod '{curator_app}'...");
    fly.create_app(curator_app, &org_slug).await?;
    println!("  App created.");

    println!("Creating Curator volume (1GB)...");
    fly.create_volume(curator_app, "hkask_data", "iad", 1)
        .await?;
    println!("  Volume created.");

    let mut curator_secrets = std::collections::HashMap::new();
    curator_secrets.insert("POD_ID".to_string(), "curator".to_string());
    curator_secrets.insert("HKASK_DATA_DIR".to_string(), "/data".to_string());
    curator_secrets.insert(
        "HKASK_BASE_URL".to_string(),
        format!("https://{curator_app}.fly.dev"),
    );
    curator_secrets.insert(
        "HKASK_MATRIX_URL".to_string(),
        format!("http://{app_name}.internal:8008"),
    );
    // Pass Fly token so curator can manage other pods
    curator_secrets.insert("FLY_API_TOKEN".to_string(), token.clone());
    curator_secrets.insert("FLY_ORG_SLUG".to_string(), org_slug.clone());

    println!("Setting Curator secrets...");
    fly.set_secrets(curator_app, &curator_secrets).await?;
    println!("  Secrets set.");

    let container_registry =
        std::env::var("CONTAINER_REGISTRY").unwrap_or_else(|_| "ghcr.io/mdz-axo/hkask".to_string());
    let version = std::env::var("HKASK_VERSION").unwrap_or_else(|_| "0.30.0".to_string());

    println!("Deploying Curator Machine...");
    let curator_machine = crate::cloud::fly::MachineConfig {
        name: "curator".to_string(),
        region: "iad".to_string(),
        config: crate::cloud::fly::MachineSpec {
            image: format!("{container_registry}:kask-{version}"),
            env: Some(curator_secrets),
            mounts: Some(vec![crate::cloud::fly::MachineMount {
                volume: "hkask_data".to_string(),
                path: "/data".to_string(),
            }]),
            services: Some(vec![crate::cloud::fly::MachineService {
                protocol: "tcp".to_string(),
                internal_port: 3000,
                ports: Some(vec![crate::cloud::fly::MachinePort {
                    port: 443,
                    handlers: vec!["tls".to_string(), "http".to_string()],
                }]),
                autostop: Some(false),
                autostart: Some(true),
            }]),
            guest: crate::cloud::fly::MachineGuest {
                cpu_kind: "shared".to_string(),
                cpus: 1,
                memory_mb: 512,
            },
        },
    };
    let machine = fly.create_machine(curator_app, &curator_machine).await?;
    println!("  Curator Machine: {} ({})", machine.name, machine.id);

    println!();
    println!("=== hKask system initialized ===");
    println!("Curator pod:    https://{curator_app}.fly.dev");
    println!("Conduit server:  http://{app_name}.internal:8008");
    println!();
    println!("Create a user pod:");
    println!("  kask pod create alice");
    println!();
    println!("Set in your .env for local CLI access:");
    println!("  HKASK_MATRIX_URL=http://{app_name}.internal:8008");

    Ok(())
}

/// Generate an Ed25519 signing key for Conduit.
///
/// Uses the OS random number generator via the `rand` crate.
/// Ed25519 private key is 32 bytes, encoded as base64 for Conduit.
fn generate_conduit_signing_key() -> String {
    use rand::RngCore;
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(key_bytes)
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  none
/// post: returns Ok(`Vec<EscalationEntry>`) with all pending escalations
/// post: delegates to escalation_queue.list_pending()
pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    // Use the escalation queue via AgentService for raw EscalationEntry access.
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
/// post: returns Ok(()) if escalation dismissed successfully
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
