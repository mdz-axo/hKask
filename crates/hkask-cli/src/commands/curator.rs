//! Curator governance command handlers — escalations, metacognition
//
//! Routed through `CuratorService` for business logic consistency
//! across CLI and API surfaces. All context is derived from `ServiceContext`
//! via `CuratorContext::from(&*ctx)` — no direct database access.

use hkask_services::{CuratorContext, CuratorService};

use crate::block_on;
use crate::cli::CuratorAction;
use crate::errors::CuratorError;

/// Build a ServiceContext for curator subcommands.
///
/// Uses `ServiceConfig::from_env()` to resolve configuration, then builds
/// the full service context. This replaces the old pattern of opening
/// databases directly via `open_registry_db()`.
async fn build_service_context() -> Result<hkask_services::ServiceContext, CuratorError> {
    let config = hkask_services::ServiceConfig::from_env().map_err(CuratorError::from)?;
    hkask_services::ServiceContext::build(config)
        .await
        .map_err(CuratorError::from)
}

/// List all pending escalations via CuratorService.
pub async fn curator_escalations() -> Result<Vec<hkask_agents::EscalationEntry>, CuratorError> {
    let ctx = build_service_context().await?;
    let curator_ctx = CuratorContext::from(&ctx);
    CuratorService::list_escalations(&curator_ctx).map_err(CuratorError::from)
}

/// Resolve an escalation by ID via CuratorService.
pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let ctx = build_service_context().await?;
    let curator_ctx = CuratorContext::from(&ctx);
    CuratorService::resolve_escalation(&curator_ctx, id, "cli-administrator")
        .map_err(CuratorError::from)
}

/// Dismiss an escalation by ID via CuratorService.
pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let ctx = build_service_context().await?;
    let curator_ctx = CuratorContext::from(&ctx);
    CuratorService::dismiss_escalation(&curator_ctx, id, "cli-administrator")
        .map_err(CuratorError::from)
}

/// Run a metacognition cycle and return a summary string via CuratorService.
///
/// Uses `CuratorContext::from_service_context()` which provides the CNS
/// runtime and dispatch required for metacognition operations.
pub async fn curator_metacognition() -> Result<String, CuratorError> {
    let ctx = build_service_context().await?;
    let curator_ctx = CuratorContext::from_service_context(&ctx).await;
    let summary = CuratorService::run_metacognition(&curator_ctx).await?;
    Ok(summary.summary_text)
}

/// CLI handler for `kask curator` subcommand
pub fn run_curator(
    rt: &tokio::runtime::Runtime,
    registry: &hkask_templates::SqliteRegistry,
    runtime: &hkask_mcp::runtime::McpRuntime,
    handle: &tokio::runtime::Handle,
    action: crate::cli::CuratorAction,
) {
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
                        &esc.id[..std::cmp::min(20, esc.id.len())],
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
    }
}
