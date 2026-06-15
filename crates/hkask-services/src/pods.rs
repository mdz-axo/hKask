//! PodService — agent pod lifecycle management for CLI and API surfaces.
//!
//! Delegates to `AgentService::pod_manager()` and wraps `AgentPodError`
//! as `ServiceError::Pod`. Both CLI and API surfaces were previously
//! calling `pod_manager()` directly with duplicated error mapping and
//! pod ID parsing logic.

use hkask_agents::pod::{AgentPersona, PodID, PodStatus};

use crate::AgentService;
use crate::error::ServiceError;

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
    pub async fn list_pods(ctx: &AgentService) -> Result<Vec<PodStatusResponse>, ServiceError> {
        let pm = ctx.pod_manager();
        let pods = pm.list_pods().await.map_err(ServiceError::Pod)?;
        Ok(pods.into_iter().map(PodStatusResponse::from).collect())
    }

    /// Activate a pod by ID.
    pub async fn activate_pod(ctx: &AgentService, pod_id: &str) -> Result<(), ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager()
            .activate_pod(&pid)
            .await
            .map_err(ServiceError::Pod)?;
        Ok(())
    }

    /// Deactivate a pod by ID.
    pub async fn deactivate_pod(ctx: &AgentService, pod_id: &str) -> Result<(), ServiceError> {
        let pid = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager()
            .deactivate_pod(&pid)
            .await
            .map_err(ServiceError::Pod)?;
        Ok(())
    }

    /// Get pod status by ID.
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

    // REQ: MDS-pod-svc-001 — parse_pod_id validates UUID format
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

    // REQ: MDS-pod-svc-002 — parse_pod_id accepts valid UUID
    #[test]
    fn parse_pod_id_accepts_valid_uuid() {
        let valid = uuid::Uuid::new_v4().to_string();
        let result = PodService::parse_pod_id(&valid);
        assert!(result.is_ok());
    }

    // REQ: MDS-pod-svc-003 — PodStatus → PodStatusResponse preserves all fields
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
