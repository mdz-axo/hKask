//! Curator governance command handlers — escalations, metacognition
//!
//! Routed through `CuratorService` for business logic consistency
//! across CLI and API surfaces.

use std::sync::Arc;

use crate::block_on;
use crate::cli::CuratorAction;
use hkask_agents::EscalationEntry;
use hkask_services::{CuratorContext, CuratorService};

use crate::commands::config::open_registry_db;
use crate::errors::CuratorError;

/// List all pending escalations via CuratorService.
pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, CuratorError> {
    let conn = open_registry_db()?;
    let queue = Arc::new(hkask_agents::EscalationQueue::new(conn)?);
    let ctx = CuratorContext::from_parts(queue, None, None);
    CuratorService::list_escalations(&ctx).map_err(CuratorError::from)
}

/// Resolve an escalation by ID via CuratorService.
pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = Arc::new(hkask_agents::EscalationQueue::new(conn)?);
    let ctx = CuratorContext::from_parts(queue, None, None);
    CuratorService::resolve_escalation(&ctx, id, "cli-administrator").map_err(CuratorError::from)
}

/// Dismiss an escalation by ID via CuratorService.
pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = Arc::new(hkask_agents::EscalationQueue::new(conn)?);
    let ctx = CuratorContext::from_parts(queue, None, None);
    CuratorService::dismiss_escalation(&ctx, id, "cli-administrator").map_err(CuratorError::from)
}

/// Run a metacognition cycle and return a summary string via CuratorService.
pub async fn curator_metacognition() -> Result<String, CuratorError> {
    let conn = open_registry_db()?;
    let queue = Arc::new(hkask_agents::EscalationQueue::new(conn)?);
    let cns = Arc::new(hkask_cns::CnsRuntime::with_threshold(
        hkask_cns::DEFAULT_THRESHOLD,
    ));
    let dispatch = Arc::new(hkask_agents::communication::MessageDispatch::new());

    let ctx = CuratorContext::from_parts(queue, Some(cns), Some(dispatch));
    let summary = CuratorService::run_metacognition(&ctx).await?;
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
