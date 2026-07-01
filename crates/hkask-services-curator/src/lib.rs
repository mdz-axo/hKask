//! CuratorService — metacognition management for CLI and API surfaces.
//!
//! Escalation CRUD has moved to `GovernanceContext`. This struct
//! retains `_direct` delegation methods for granular callers (MCP servers)
//! and the metacognition cycle.

use std::sync::Arc;

use hkask_agents::curator_agent::CuratorAgent;
use hkask_cns::types::loops::CuratorHandle;
use hkask_services_context::AgentService;
use hkask_services_context::governance;
use hkask_services_core::ServiceError;

/// Service for curator metacognition — delegates to agent infrastructure.
pub struct CuratorService;

impl CuratorService {
    /// List pending escalations (granular — no `AgentService` required).
    #[must_use = "result must be used"]
    pub fn list_escalations_direct(
        queue: &hkask_storage::EscalationQueue,
    ) -> Result<Vec<governance::EscalationResponse>, ServiceError> {
        governance::list_escalations_direct(queue)
    }

    /// Resolve an escalation by ID (granular — no `AgentService` required).
    #[must_use = "result must be used"]
    pub fn resolve_direct(
        queue: &hkask_storage::EscalationQueue,
        events: &Arc<dyn hkask_types::event::NuEventSink>,
        id: &str,
        resolved_by: &str,
    ) -> Result<(), ServiceError> {
        governance::resolve_direct(queue, events, id, resolved_by)
    }

    /// Dismiss an escalation by ID (granular — no `AgentService` required).
    #[must_use = "result must be used"]
    pub fn dismiss_direct(
        queue: &hkask_storage::EscalationQueue,
        events: &Arc<dyn hkask_types::event::NuEventSink>,
        id: &str,
        dismissed_by: &str,
    ) -> Result<(), ServiceError> {
        governance::dismiss_direct(queue, events, id, dismissed_by)
    }

    /// Run a metacognition cycle and return a human-readable summary.
    #[must_use = "result must be used"]
    pub async fn metacognition(ctx: &AgentService) -> Result<String, ServiceError> {
        let queue = Arc::clone(&ctx.governance().escalations);
        let cns_lock = &ctx.cns().runtime;
        let cns = Arc::new(cns_lock.read().await.clone());

        let agents_ctx = Arc::new(hkask_agents::CuratorContext::new(
            CuratorHandle::system(),
            cns,
            None,
            queue,
        ));
        let agent = CuratorAgent::new(agents_ctx);
        let agent = if let Some(curator_id) = ctx.infra().pods.clone().find_by_name("curator").await
        {
            if let Some(persona) = ctx.infra().pods.clone().persona(&curator_id).await {
                if let Some(posture) = persona.communication_posture {
                    agent.with_communication_posture(posture)
                } else {
                    agent
                }
            } else {
                agent
            }
        } else {
            agent
        };
        let snapshot =
            agent
                .metacognition()
                .run_cycle()
                .await
                .map_err(|e| ServiceError::Metacognition {
                    source: None,
                    message: e.to_string(),
                })?;
        let summary = agent.metacognition().generate_summary(&snapshot);

        Self::post_to_matrix_if_configured(ctx, &summary).await;

        Ok(summary)
    }

    async fn post_to_matrix_if_configured(ctx: &AgentService, summary: &str) {
        let room_id = match std::env::var("HKASK_CURATOR_ROOM_ID") {
            Ok(id) if !id.is_empty() => id,
            _ => return,
        };

        let transport = match ctx.infra().matrix.as_ref() {
            Some(t) => t,
            None => return,
        };

        use hkask_communication::matrix::RoomId;
        let room = RoomId(room_id);
        if let Err(e) = transport
            .lock()
            .await
            .send_message(&room, summary, None)
            .await
        {
            tracing::warn!(
                target: "cns.curation.matrix",
                room_id = %room.0,
                error = %e,
                "Failed to post metacognition summary to Matrix"
            );
        } else {
            tracing::info!(
                target: "cns.curation.matrix",
                room_id = %room.0,
                "Metacognition summary posted to Matrix standing session"
            );
        }
    }
}
