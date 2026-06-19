//! Curator commands — delegates to CuratorService.


use hkask_services::{CuratorService, ServiceError};
use hkask_storage::EscalationEntry;

use crate::block_on;
use crate::cli::CuratorAction;

pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    // Use the escalation queue via AgentService for raw EscalationEntry access.
    let queue = ctx.escalation_queue();
    queue.list_pending().map_err(|e| ServiceError::Escalation {
        message: e.to_string(),
    })
}

pub async fn curator_resolve(id: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    CuratorService::resolve(&ctx, id, "cli-administrator")
}

pub async fn curator_dismiss(id: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    CuratorService::dismiss(&ctx, id, "cli-administrator")
}

pub async fn curator_metacognition() -> Result<String, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    CuratorService::metacognition(&ctx).await
}

