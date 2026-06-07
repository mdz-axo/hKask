//! Curator governance command handlers — escalations, metacognition

use crate::block_on;
use crate::cli::CuratorAction;
use hkask_agents::EscalationEntry;
use std::sync::Arc;

use crate::commands::config::open_registry_db;
use crate::errors::CuratorError;

/// List all pending escalations
pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)?;
    // Explicit type annotation required: `?` on a bare expression
    // doesn't infer the Result type without it (compiler can't pick the
    // conversion target).
    let escalations: Vec<EscalationEntry> = queue.list_pending()?;
    Ok(escalations)
}

/// Resolve an escalation by ID
pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)?;
    queue.resolve(id, "cli-administrator")?;
    Ok(())
}

/// Dismiss an escalation by ID
pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)?;
    queue.dismiss(id, "cli-administrator")?;
    Ok(())
}

/// Run a metacognition cycle and return a summary string
pub async fn curator_metacognition() -> Result<String, CuratorError> {
    use hkask_agents::MessageDispatch;
    use hkask_agents::curator::CuratorContext;
    use hkask_agents::curator_agent::CuratorAgent;
    use hkask_cns::CnsRuntime;
    use hkask_types::loops::curation::CuratorHandle;

    let conn = open_registry_db()?;
    let queue = Arc::new(hkask_agents::EscalationQueue::new(conn)?);

    let cns = Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD));
    let dispatch = Arc::new(MessageDispatch::new());
    let curator_handle = CuratorHandle::system();
    let context = Arc::new(CuratorContext::new(curator_handle, cns, dispatch, queue));
    let agent = CuratorAgent::new(context);
    let metacognition = agent.metacognition();

    let snapshot = metacognition.run_cycle().await?;
    Ok(metacognition.generate_summary(&snapshot))
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
