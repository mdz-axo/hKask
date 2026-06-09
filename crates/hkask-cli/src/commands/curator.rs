//! Curator commands — escalate to EscalationQueue directly.

use std::sync::Arc;

use hkask_agents::EscalationEntry;
use hkask_agents::EscalationQueue;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_cns::CnsRuntime;
use hkask_types::CuratorHandle;

use crate::block_on;
use crate::cli::CuratorAction;
use crate::errors::CuratorError;

/// Open DB, build escalation queue, build CNS + dispatch for metacognition.
async fn build_curator_infra() -> Result<
    (
        Arc<EscalationQueue>,
        Option<Arc<CnsRuntime>>,
        Option<Arc<MessageDispatch>>,
    ),
    CuratorError,
> {
    let config = hkask_services::ServiceConfig::from_env().map_err(CuratorError::from)?;
    let db = hkask_storage::Database::open(&config.db_path, &config.db_passphrase)
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))?;
    let conn = db.conn_arc();
    let queue = Arc::new(
        EscalationQueue::new(conn)
            .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))?,
    );
    let cns = Some(Arc::new(CnsRuntime::with_threshold(config.cns_threshold)));
    let dispatch = Some(Arc::new(MessageDispatch::new()));
    Ok((queue, cns, dispatch))
}

pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, CuratorError> {
    let (queue, _, _) = build_curator_infra().await?;
    queue
        .list_pending()
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))
}

pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let (queue, _, _) = build_curator_infra().await?;
    if queue
        .get(id)
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))?
        .is_none()
    {
        return Err(CuratorError::from(
            hkask_services::ServiceError::EscalationNotFound(id.to_string()),
        ));
    }
    queue
        .resolve(id, "cli-administrator")
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))
}

pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let (queue, _, _) = build_curator_infra().await?;
    if queue
        .get(id)
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))?
        .is_none()
    {
        return Err(CuratorError::from(
            hkask_services::ServiceError::EscalationNotFound(id.to_string()),
        ));
    }
    queue
        .dismiss(id, "cli-administrator")
        .map_err(|e| CuratorError::from(hkask_services::ServiceError::from(e)))
}

pub async fn curator_metacognition() -> Result<String, CuratorError> {
    let (queue, cns, dispatch) = build_curator_infra().await?;
    let cns = cns.unwrap();
    let dispatch = dispatch.unwrap();
    let agents_ctx = Arc::new(hkask_agents::CuratorContext::new(
        CuratorHandle::system(),
        cns,
        dispatch,
        queue.clone(),
    ));
    let agent = CuratorAgent::new(agents_ctx);
    let snapshot = agent.metacognition().run_cycle().await?;
    Ok(agent.metacognition().generate_summary(&snapshot))
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
