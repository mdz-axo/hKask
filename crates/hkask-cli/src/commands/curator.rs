//! Curator commands — delegates to CuratorService.

use hkask_agents::EscalationEntry;
use hkask_services::{AgentService, CuratorService, ServiceConfig};

use crate::block_on;
use crate::cli::CuratorAction;
use crate::errors::CuratorError;

fn build_service_context() -> Result<AgentService, CuratorError> {
    let config = ServiceConfig::from_env().map_err(CuratorError::from)?;
    let rt = tokio::runtime::Runtime::new().expect("runtime should start");
    rt.block_on(AgentService::build(config))
        .map_err(CuratorError::from)
}

pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, CuratorError> {
    let ctx = build_service_context()?;
    // Use the escalation queue via AgentService for raw EscalationEntry access.
    // The CuratorService provides typed EscalationResponse; the CLI needs raw
    // fields (like bot_id.as_uuid()) for formatted display.
    let queue = ctx.escalation_queue();
    queue
        .list_pending()
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))
}

pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let ctx = build_service_context()?;
    CuratorService::resolve(&ctx, id, "cli-administrator").map_err(CuratorError::from)
}

pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let ctx = build_service_context()?;
    CuratorService::dismiss(&ctx, id, "cli-administrator").map_err(CuratorError::from)
}

pub async fn curator_metacognition() -> Result<String, CuratorError> {
    let ctx = build_service_context()?;
    CuratorService::metacognition(&ctx)
        .await
        .map_err(CuratorError::from)
}

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
