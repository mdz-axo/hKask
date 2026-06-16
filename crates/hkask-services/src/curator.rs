//! CuratorService — escalation management for CLI and API surfaces.
//!
//! Delegates to `AgentService::escalation_queue()` and wraps
//! `EscalationError` as `ServiceError::Escalation`. Handles
//! escalation CRUD (list, resolve, dismiss) and metacognition.
//! Both CLI and API surfaces were previously calling
//! `escalation_queue()` directly with duplicated error mapping.

use std::sync::Arc;

use hkask_agents::curator_agent::CuratorAgent;
use hkask_storage::EscalationEntry;
use hkask_types::CuratorHandle;
use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};

use crate::AgentService;
use crate::error::ServiceError;

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

/// Service for curator operations — delegates to EscalationQueue.
pub struct CuratorService;

impl CuratorService {
    /// List pending escalations.
    ///
    /// REQ: SVC-213
    /// pre:  ctx.escalation_queue() must be initialized
    /// post: returns Vec<EscalationResponse> of pending escalations; empty Vec if none; Err(Escalation) on queue error
    /// # Returns
    /// `ServiceError::Escalation` on queue error.
    pub fn list_escalations(ctx: &AgentService) -> Result<Vec<EscalationResponse>, ServiceError> {
        let queue = ctx.escalation_queue();
        let entries = queue.list_pending().map_err(ServiceError::Escalation)?;
        Ok(entries.into_iter().map(EscalationResponse::from).collect())
    }

    /// Resolve an escalation by ID.
    ///
    /// REQ: SVC-214
    /// pre:  ctx.escalation_queue() must be initialized; id must be a valid escalation ID; resolved_by must be non-empty
    /// post: escalation is resolved; CNS event emitted; Ok(()) on success; Err(EscalationNotFound) if ID not found; Err(Escalation) on queue error
    /// # Returns
    /// `ServiceError::EscalationNotFound` if the ID doesn't match any entry.
    /// `ServiceError::Escalation` on queue error.
    pub fn resolve(ctx: &AgentService, id: &str, resolved_by: &str) -> Result<(), ServiceError> {
        // CNS observability: record Curator resolution decision
        let span = Span::new(
            SpanNamespace::from(CnsSpan::Curation),
            "escalation_resolved",
        );
        let event = NuEvent::new(
            WebID::new(),
            span,
            Phase::Act,
            serde_json::json!({
                "escalation_id": id,
                "resolved_by": resolved_by,
            }),
            0,
        );
        let _ = ctx.event_sink().persist(&event);

        ctx.escalation_queue()
            .resolve(id, resolved_by)
            .map_err(|e| match e {
                hkask_storage::EscalationError::NotFound(id) => ServiceError::EscalationNotFound {
                    source: None,
                    message: id,
                },
                other => ServiceError::Escalation(other),
            })
    }

    /// Dismiss an escalation by ID.
    ///
    /// REQ: SVC-215
    /// pre:  ctx.escalation_queue() must be initialized; id must be a valid escalation ID; dismissed_by must be non-empty
    /// post: escalation is dismissed; CNS event emitted; Ok(()) on success; Err(EscalationNotFound) if ID not found; Err(Escalation) on queue error
    /// # Returns
    /// `ServiceError::EscalationNotFound` if the ID doesn't match any entry.
    /// `ServiceError::Escalation` on queue error.
    pub fn dismiss(ctx: &AgentService, id: &str, dismissed_by: &str) -> Result<(), ServiceError> {
        // CNS observability: record Curator dismissal decision
        let span = Span::new(
            SpanNamespace::from(CnsSpan::Curation),
            "escalation_dismissed",
        );
        let event = NuEvent::new(
            WebID::new(),
            span,
            Phase::Act,
            serde_json::json!({
                "escalation_id": id,
                "dismissed_by": dismissed_by,
            }),
            0,
        );
        let _ = ctx.event_sink().persist(&event);

        ctx.escalation_queue()
            .dismiss(id, dismissed_by)
            .map_err(|e| match e {
                hkask_storage::EscalationError::NotFound(id) => ServiceError::EscalationNotFound {
                    source: None,
                    message: id,
                },
                other => ServiceError::Escalation(other),
            })
    }

    /// Run a metacognition cycle and return a human-readable summary.
    ///
    /// Constructs a `CuratorAgent` from the AgentService's escalation queue
    /// and CNS runtime, runs one metacognition cycle, and generates a summary.
    ///
    /// REQ: SVC-216
    /// pre:  ctx.escalation_queue() and ctx.cns_runtime() must be initialized
    /// post: returns human-readable summary string from metacognition cycle; Err(Metacognition) on cycle failure; Err(Cns) if CNS runtime unavailable
    /// # Returns
    /// `ServiceError::Metacognition` on cycle failure.
    /// `ServiceError::Cns` if CNS runtime is unavailable.
    pub async fn metacognition(ctx: &AgentService) -> Result<String, ServiceError> {
        let queue = ctx.escalation_queue();
        // Use the live CNS runtime (RwLock<CnsRuntime>) — clone the inner
        // state so the CuratorAgent sees current alerts and variety, not zeros.
        let cns_lock = ctx.cns_runtime();
        let cns = Arc::new(cns_lock.read().await.clone());

        let agents_ctx = Arc::new(hkask_agents::CuratorContext::new(
            CuratorHandle::system(),
            cns,
            None,
            queue.clone(),
        ));
        let agent = CuratorAgent::new(agents_ctx);
        let snapshot = agent
            .metacognition()
            .run_cycle()
            .await
            .map_err(ServiceError::Metacognition)?;
        Ok(agent.metacognition().generate_summary(&snapshot))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::EscalationID;
    use uuid::Uuid;

    const FIXED_UUID_1: &str = "00000000-0000-0000-0000-000000000001";
    const FIXED_UUID_2: &str = "00000000-0000-0000-0000-000000000002";

    // REQ: MDS-curator-svc-001 — EscalationEntry → EscalationResponse maps all fields
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

    // REQ: MDS-curator-svc-002 — resolved escalation has resolution fields populated
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
