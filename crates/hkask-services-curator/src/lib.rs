//! CuratorService — escalation management for CLI and API surfaces.
//!
//! Delegates to `AgentService::governance().escalations` and wraps
//! `EscalationError` as `ServiceError::Escalation`. Handles
//! escalation CRUD (list, resolve, dismiss) and metacognition.

use std::sync::Arc;

use hkask_agents::curator_agent::CuratorAgent;
use hkask_cns::types::loops::CuratorHandle;
use hkask_storage::EscalationEntry;
use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{CyclePhase, NuEvent, Span, SpanNamespace};

use hkask_services_context::AgentService;
use hkask_services_core::ServiceError;

/// Response for a single escalation entry.
pub struct EscalationResponse {
    pub id: String,
    pub template_id: String,
    pub bot_id: String,
    pub output: String,
    pub confidence: f64,
    pub retry_count: u32,
    pub error_context: String,
    pub created_at: String,
    pub status: String,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
}

impl From<EscalationEntry> for EscalationResponse {
    fn from(e: EscalationEntry) -> Self {
        Self {
            id: e.id.to_string(),
            template_id: e.template_id.to_string(),
            bot_id: e.bot_id.to_string(),
            output: e.output,
            confidence: e.confidence,
            retry_count: e.retry_count,
            error_context: e.error_context,
            created_at: e.created_at.to_rfc3339(),
            status: format!("{:?}", e.status).to_lowercase(),
            resolved_at: e.resolved_at.map(|dt| dt.to_rfc3339()),
            resolved_by: e.resolved_by,
        }
    }
}

/// Service for curator operations — delegates to governance context.
pub struct CuratorService;

/// Emit a CNS ν-event for an escalation operation (resolve/dismiss).
fn emit_escalation_event(
    ctx: &AgentService,
    operation: &str,
    actor_key: &str,
    escalation_id: &str,
    actor: &str,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::Curation), operation);
    let event = NuEvent::new(
        WebID::from_persona(b"curator"),
        span,
        CyclePhase::Act,
        serde_json::json!({
            "escalation_id": escalation_id,
            actor_key: actor,
        }),
        0,
    );
    if let Err(e) = ctx.event_sink().persist(&event) {
        tracing::warn!(
            target: "cns.curation",
            escalation_id = %escalation_id,
            error = %e,
            operation = operation,
            "CNS event persist failed — observability gap"
        );
    }
}

impl CuratorService {
    /// List pending escalations.
    pub fn list_escalations(ctx: &AgentService) -> Result<Vec<EscalationResponse>, ServiceError> {
        let entries =
            ctx.governance()
                .escalations
                .list_pending()
                .map_err(|e| ServiceError::Escalation {
                    message: e.to_string(),
                })?;
        Ok(entries.into_iter().map(EscalationResponse::from).collect())
    }

    /// Resolve an escalation by ID.
    pub fn resolve(ctx: &AgentService, id: &str, resolved_by: &str) -> Result<(), ServiceError> {
        emit_escalation_event(ctx, "escalation_resolved", "resolved_by", id, resolved_by);

        ctx.governance()
            .escalations
            .resolve(id, resolved_by)
            .map_err(|e| match e {
                hkask_storage::EscalationError::NotFound(id) => ServiceError::EscalationNotFound {
                    source: None,
                    message: id,
                },
                other => ServiceError::Escalation {
                    message: other.to_string(),
                },
            })
    }

    /// Dismiss an escalation by ID.
    pub fn dismiss(ctx: &AgentService, id: &str, dismissed_by: &str) -> Result<(), ServiceError> {
        emit_escalation_event(
            ctx,
            "escalation_dismissed",
            "dismissed_by",
            id,
            dismissed_by,
        );

        ctx.governance()
            .escalations
            .dismiss(id, dismissed_by)
            .map_err(|e| match e {
                hkask_storage::EscalationError::NotFound(id) => ServiceError::EscalationNotFound {
                    source: None,
                    message: id,
                },
                other => ServiceError::Escalation {
                    message: other.to_string(),
                },
            })
    }

    /// Run a metacognition cycle and return a human-readable summary.
    pub async fn metacognition(ctx: &AgentService) -> Result<String, ServiceError> {
        let queue = Arc::clone(&ctx.governance().escalations);
        let cns_lock = ctx.cns_runtime();
        let cns = Arc::new(cns_lock.read().await.clone());

        let agents_ctx = Arc::new(hkask_agents::CuratorContext::new(
            CuratorHandle::system(),
            cns,
            None,
            queue,
        ));
        let agent = CuratorAgent::new(agents_ctx);
        let agent = if let Some(curator_id) = ctx.pod_manager().find_by_name("curator").await {
            if let Some(persona) = ctx.pod_manager().persona(&curator_id).await {
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

        let transport = match ctx.matrix_transport() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::EscalationID;
    use uuid::Uuid;

    const FIXED_UUID_1: &str = "00000000-0000-0000-0000-000000000001";
    const FIXED_UUID_2: &str = "00000000-0000-0000-0000-000000000002";

    #[test]
    fn escalation_entry_to_response_maps_fields() {
        let entry = EscalationEntry {
            id: EscalationID::from_uuid(Uuid::parse_str(FIXED_UUID_1).unwrap()),
            template_id: hkask_types::TemplateID::new(),
            bot_id: hkask_types::BotID::new(),
            output: "test output".into(),
            confidence: 0.85,
            retry_count: 2,
            error_context: "some error".into(),
            created_at: chrono::Utc::now(),
            status: hkask_storage::EscalationStatus::Pending,
            resolved_at: None,
            resolved_by: None,
        };
        let resp = EscalationResponse::from(entry);
        assert_eq!(resp.id, FIXED_UUID_1);
        assert_eq!(resp.output, "test output");
        assert!((resp.confidence - 0.85).abs() < 0.001);
        assert_eq!(resp.retry_count, 2);
        assert_eq!(resp.status, "pending");
        assert!(resp.resolved_at.is_none());
    }

    #[test]
    fn escalation_entry_resolved_maps_resolution_fields() {
        let now = chrono::Utc::now();
        let entry = EscalationEntry {
            id: EscalationID::from_uuid(Uuid::parse_str(FIXED_UUID_2).unwrap()),
            template_id: hkask_types::TemplateID::new(),
            bot_id: hkask_types::BotID::new(),
            output: "done".into(),
            confidence: 1.0,
            retry_count: 0,
            error_context: String::new(),
            created_at: now,
            status: hkask_storage::EscalationStatus::Resolved,
            resolved_at: Some(now),
            resolved_by: Some("admin".into()),
        };
        let resp = EscalationResponse::from(entry);
        assert_eq!(resp.status, "resolved");
        assert!(resp.resolved_at.is_some());
        assert_eq!(resp.resolved_by, Some("admin".into()));
    }
}
