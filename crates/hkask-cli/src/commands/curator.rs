//! Curator commands — delegates to CuratorService.
//! curator_init: stub pending K8s deployment pipeline rebuild.

use hkask_services::{CuratorService, ServiceError};
use hkask_storage::EscalationEntry;

use crate::block_on;
use crate::cli::CuratorAction;

/// Initialize the hKask system.
///
/// K8s deployment pipeline (Hetzner K3s) — under construction.
/// This stub will be replaced with a full K8s-based init that:
///   1. Validates HCLOUD_TOKEN, CONTAINER_REGISTRY, LITESTREAM_* env vars
///   2. Deploys Conduit Matrix homeserver as a K8s StatefulSet
///   3. Generates Conduit signing key, stores in Curator keystore
///   4. Deploys Curator pod as a K8s StatefulSet
///   5. Provisions Litestream sidecar for continuous SQLite backup
pub async fn curator_init(domain: &str) -> Result<(), String> {
    let hcloud_token = std::env::var("HCLOUD_TOKEN")
        .map_err(|_| "HCLOUD_TOKEN not set. Set your Hetzner Cloud API token.".to_string())?;
    if hcloud_token.is_empty() {
        return Err("HCLOUD_TOKEN is empty.".to_string());
    }

    println!("=== hKask Curator Init (K8s) ===");
    println!("Domain: {domain}");
    println!();
    println!("K8s deployment pipeline is under construction.");
    println!("For now, export K8s manifests:");
    println!("  kask pod export-k8s curator");
    println!();
    println!("Then apply manually:");
    println!("  kubectl apply -f ./k8s-deploy/");

    Ok(())
}

/// Generate an Ed25519 signing key for Conduit.
///
/// Uses the OS random number generator via the `rand` crate.
/// Ed25519 private key is 32 bytes, encoded as base64 for Conduit.
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
