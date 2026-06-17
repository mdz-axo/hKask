//! PodService — agent pod lifecycle management for CLI and API surfaces.
//!
//! Delegates to `AgentService::pod_manager()` and wraps `AgentPodError`
//! as `ServiceError::Pod`. Both CLI and API surfaces were previously
//! calling `pod_manager()` directly with duplicated error mapping and
//! pod ID parsing logic.

use hkask_agents::pod::{AgentPersona, PodID, PodStatus};

use crate::ServiceError;
use hkask_services_context::AgentService;

/// Request to create a new agent pod.
pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}

/// Response after pod creation.
pub struct PodResponse {
    pub pod_id: String,
}

/// Response for pod status query.
pub struct PodStatusResponse {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub template: String,
    pub created_at: i64,
}

impl From<PodStatus> for PodStatusResponse {
    fn from(s: PodStatus) -> Self {
        Self {
            pod_id: s.pod_id,
            name: s.name,
            state: s.state.to_string(),
            webid: s.webid,
            agent_type: s.agent_type.to_string(),
            template: s.template,
            created_at: s.created_at,
        }
    }
}

/// Service for pod lifecycle management — delegates to PodManager.
pub struct PodService;

impl PodService {
    /// Create a new agent pod from a template and persona YAML.
    ///
    /// REQ: P1-svc-pods-128
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized; req.template must be non-empty; req.persona_yaml must be valid YAML
    /// post: pod is created and returns PodResponse with pod_id; Err(ValidationError) on invalid persona YAML; Err(Pod) on upstream error
    /// # Returns
    /// `ServiceError::Pod` on upstream pod error.
    /// `ServiceError::ValidationError` on invalid persona YAML.
    pub async fn create_pod(
        ctx: &AgentService,
        req: CreatePodRequest,
    ) -> Result<PodResponse, ServiceError> {
        let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| {
            let msg = format!("Invalid persona YAML: {e}");
            ServiceError::ValidationError {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
        let pm = ctx.pod_manager();
        let pod_id = pm
            .create_pod(&req.template, &persona, req.name)
            .await
            .map_err(ServiceError::Pod)?;
        Ok(PodResponse {
            pod_id: pod_id.to_string(),
        })
    }

    /// List all registered pods.
    ///
    /// REQ: P1-svc-pods-129
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized
    /// post: returns Vec<PodStatusResponse> for all pods; empty Vec if none; Err(Pod) on upstream error
    pub async fn list_pods(ctx: &AgentService) -> Result<Vec<PodStatusResponse>, ServiceError> {
        let pm = ctx.pod_manager();
        let pods = pm.list_pods().await.map_err(ServiceError::Pod)?;
        Ok(pods.into_iter().map(PodStatusResponse::from).collect())
    }

    /// Activate a pod by ID.
    ///
    /// REQ: P1-svc-pods-130
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized; pod_id must be a valid UUID
    /// post: pod is activated; Ok(()) on success; Err(PodNotFound) on invalid UUID; Err(Pod) on upstream error
    pub async fn activate_pod(ctx: &AgentService, pod_id: &str) -> Result<(), ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager()
            .activate_pod(&pid)
            .await
            .map_err(ServiceError::Pod)?;
        Ok(())
    }

    /// Deactivate a pod by ID.
    ///
    /// REQ: P1-svc-pods-131
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized; pod_id must be a valid UUID
    /// post: pod is deactivated; Ok(()) on success; Err(PodNotFound) on invalid UUID; Err(Pod) on upstream error
    pub async fn deactivate_pod(ctx: &AgentService, pod_id: &str) -> Result<(), ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager()
            .deactivate_pod(&pid)
            .await
            .map_err(ServiceError::Pod)?;
        Ok(())
    }

    /// Get pod status by ID.
    ///
    /// REQ: P1-svc-pods-132
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized; pod_id must be a valid UUID
    /// post: returns PodStatusResponse with pod state, webid, agent_type, template, etc.; Err(PodNotFound) on invalid UUID; Err(Pod) on upstream error
    pub async fn get_pod_status(
        ctx: &AgentService,
        pod_id: &str,
    ) -> Result<PodStatusResponse, ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        let status = ctx
            .pod_manager()
            .get_pod_status(&pid)
            .await
            .map_err(ServiceError::Pod)?;
        Ok(PodStatusResponse::from(status))
    }

    /// Parse a pod ID string into a PodID.
    fn parse_pod_id(id: &str) -> Result<PodID, ServiceError> {
        use uuid::Uuid;
        Uuid::parse_str(id)
            .map(PodID::from_uuid)
            .map_err(|_| ServiceError::PodNotFound {
                source: None,
                message: format!("Invalid pod ID '{}'", id),
            })
    }

    /// Assign an MCP role to a replicant by name.
    ///
    /// REQ: P1-svc-pods-133
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized; name and role must be non-empty
    /// post: role is assigned to the replicant; Ok(()) on success; Err(Pod) on upstream error
    pub async fn assign_role(
        ctx: &AgentService,
        name: &str,
        role: &str,
    ) -> Result<(), ServiceError> {
        ctx.pod_manager()
            .assign_role(name, role)
            .await
            .map_err(ServiceError::Pod)
    }

    /// Set the agent mode for a replicant by name.
    /// Mode: "server" (requires role), "chat", or "exit".
    ///
    /// REQ: P1-svc-pods-134
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.pod_manager() must be initialized; name and mode must be non-empty; mode must be "server", "chat", or "exit"
    /// post: agent mode is set; Ok(()) on success; Err(Pod) on upstream error
    pub async fn set_mode(
        ctx: &AgentService,
        name: &str,
        mode: &str,
        role: Option<&str>,
    ) -> Result<(), ServiceError> {
        ctx.pod_manager()
            .set_mode(name, mode, role)
            .await
            .map_err(ServiceError::Pod)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P1-svc-pods-001 — parse_pod_id validates UUID format
    #[test]
    fn parse_pod_id_rejects_invalid_uuid() {
        let result = PodService::parse_pod_id("not-a-uuid");
        assert!(result.is_err());
        match result {
            Err(ServiceError::PodNotFound { message: msg, .. }) => {
                assert!(msg.contains("Invalid pod ID"));
            }
            _ => panic!("Expected PodNotFound error"),
        }
    }

    // REQ: P1-svc-pods-002 — parse_pod_id accepts valid UUID
    #[test]
    fn parse_pod_id_accepts_valid_uuid() {
        let valid = uuid::Uuid::new_v4().to_string();
        let result = PodService::parse_pod_id(&valid);
        assert!(result.is_ok());
    }

    // REQ: P1-svc-pods-003 — PodStatus → PodStatusResponse preserves all fields
    #[test]
    fn pod_status_to_response_maps_fields() {
        let status = PodStatus {
            pod_id: "pod-1".into(),
            name: Some("TestPod".into()),
            state: hkask_agents::pod::PodLifecycleState::Registered,
            webid: "webid-1".into(),
            agent_type: hkask_agents::pod::AgentKind::Replicant,
            template: "test".into(),
            created_at: 1234567890,
        };
        let resp = PodStatusResponse::from(status);
        assert_eq!(resp.pod_id, "pod-1");
        assert_eq!(resp.name, Some("TestPod".into()));
        assert_eq!(resp.state, "registered");
        assert_eq!(resp.webid, "webid-1");
        assert_eq!(resp.agent_type, "Replicant");
        assert_eq!(resp.template, "test");
        assert_eq!(resp.created_at, 1234567890);
    }
}
