//! Pod lifecycle service — create, activate, deactivate, and query agent pods.
//!
//! `PodService` replaces duplicated UUID parsing and error normalization across
//! CLI and API surfaces. Each surface constructs a `PodContext` from its own
//! state and delegates pod operations to this service.
//!
//! # Design decisions
//!
//! - **Depth test** — Deleting this module would cause UUID parsing and
//!   not-found error handling to reappear in 10 call sites across CLI and API.
//!   Passes deletion test (≥8 call sites).
//! - **Constraint: Prohibition (P1)** — MCP servers do NOT use this service.
//!   They continue using `PodManager` directly.
//! - **Constraint: Guideline** — Auth/capability checks stay in the API surface.
//!   The service layer does not enforce who can create pods.
//! - **Constraint: Guardrail** — `deactivate_pod` normalizes the CLI bug where
//!   errors were silently swallowed (`let _ = ...`). Both surfaces now get
//!   consistent `ServiceError::Pod` or `ServiceError::PodNotFound`.
//! - **PodManager lifecycle** — `PodContext` holds `Arc<PodManager>`. CLI
//!   constructs per-invocation (mock or builder); API clones from shared state.
//!   The service layer does not own PodManager construction (Task 7).
//! - **Persona parsing** — Surfaces parse YAML into `AgentPersona` before calling
//!   the service. CLI reads from file; API receives JSON body. File I/O and
//!   request deserialization are surface concerns.

use std::sync::Arc;

use hkask_agents::pod::{AgentPersona, AgentPodError, PodID, PodManager, PodStatus};
use uuid::Uuid;

use crate::ServiceError;

/// Lightweight context for `PodService` calls.
///
/// Contains only the pod manager needed for lifecycle operations. Surfaces
/// construct this from their own state (CLI creates per-invocation; API
/// clones from `ApiState`).
pub struct PodContext {
    /// Pod manager for lifecycle operations.
    pub pod_manager: Arc<PodManager>,
}

impl PodContext {
    /// Construct from individual parts.
    ///
    /// Surfaces pass their `PodManager` instance:
    /// ```ignore
    /// let ctx = PodContext::from_parts(Arc::new(pod_manager));
    /// ```
    pub fn from_parts(pod_manager: Arc<PodManager>) -> Self {
        Self { pod_manager }
    }
}

impl From<&crate::ServiceContext> for PodContext {
    fn from(ctx: &crate::ServiceContext) -> Self {
        Self {
            pod_manager: ctx.pod_manager.clone(),
        }
    }
}

/// Pod lifecycle service — create, activate, deactivate, and query agent pods.
///
/// Use `PodService::get_pod_status()` etc. to delegate pod operations through
/// the service layer. Surfaces construct a `PodContext` from their own state
/// and call service methods.
pub struct PodService;

impl PodService {
    /// Parse a pod ID string into a `PodID`, normalizing UUID validation.
    ///
    /// Both CLI and API previously duplicated `Uuid::parse_str` + `PodID::from_uuid`.
    /// This helper centralizes the parsing and returns a consistent
    /// `ServiceError::PodNotFound` for invalid IDs.
    ///
    /// # REQ: svc-pod-001 — parse_pod_id validates UUID format
    pub fn parse_pod_id(id: &str) -> Result<PodID, ServiceError> {
        Uuid::parse_str(id)
            .map(PodID::from_uuid)
            .map_err(|e| ServiceError::PodNotFound(format!("Invalid pod ID: {}", e)))
    }

    /// Get pod status by ID string.
    ///
    /// # REQ: svc-pod-002 — get_pod_status normalizes not-found errors
    pub async fn get_pod_status(ctx: &PodContext, pod_id: &str) -> Result<PodStatus, ServiceError> {
        let id = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager
            .get_pod_status(&id)
            .await
            .map_err(normalize_pod_error)
    }

    /// List all pods.
    ///
    /// # REQ: svc-pod-003 — list_pods delegates to PodManager with consistent error mapping
    pub async fn list_pods(ctx: &PodContext) -> Result<Vec<PodStatus>, ServiceError> {
        ctx.pod_manager
            .list_pods()
            .await
            .map_err(normalize_pod_error)
    }

    /// Create a pod from template and parsed persona.
    ///
    /// # REQ: svc-pod-004 — create_pod delegates to PodManager with consistent error mapping
    pub async fn create_pod(
        ctx: &PodContext,
        template: &str,
        persona: &AgentPersona,
        name: Option<String>,
    ) -> Result<String, ServiceError> {
        ctx.pod_manager
            .create_pod(template, persona, name)
            .await
            .map(|id| id.to_string())
            .map_err(normalize_pod_error)
    }

    /// Activate a pod by ID string.
    ///
    /// # REQ: svc-pod-005 — activate_pod normalizes not-found errors
    pub async fn activate_pod(ctx: &PodContext, pod_id: &str) -> Result<(), ServiceError> {
        let id = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager
            .activate_pod(&id)
            .await
            .map_err(normalize_pod_error)
    }

    /// Deactivate a pod by ID string.
    ///
    /// Normalizes the CLI bug where deactivation errors were silently
    /// swallowed (`let _ = ...`). Both surfaces now receive proper errors.
    ///
    /// # REQ: svc-pod-006 — deactivate_pod normalizes not-found errors and fixes CLI error swallowing
    pub async fn deactivate_pod(ctx: &PodContext, pod_id: &str) -> Result<(), ServiceError> {
        let id = Self::parse_pod_id(pod_id)?;
        ctx.pod_manager
            .deactivate_pod(&id)
            .await
            .map_err(normalize_pod_error)
    }
}

/// Normalize `AgentPodError` into `ServiceError`.
///
/// Maps `PodNotFound(PodID)` to the `ServiceError::PodNotFound(String)` sentinel
/// for consistent not-found handling across surfaces. All other variants map to
/// `ServiceError::Pod(AgentPodError)`.
fn normalize_pod_error(e: AgentPodError) -> ServiceError {
    match e {
        AgentPodError::PodNotFound(id) => ServiceError::PodNotFound(id.to_string()),
        other => ServiceError::Pod(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_agents::pod::PodManager;

    fn test_ctx() -> PodContext {
        PodContext::from_parts(Arc::new(PodManager::new_mock()))
    }

    // REQ: svc-pod-001 — parse_pod_id validates UUID format
    #[test]
    fn parse_pod_id_accepts_valid_uuid() {
        let uuid = uuid::Uuid::new_v4().to_string();
        let result = PodService::parse_pod_id(&uuid);
        assert!(result.is_ok(), "valid UUID should parse successfully");
    }

    // REQ: svc-pod-001 — parse_pod_id rejects invalid UUID
    #[test]
    fn parse_pod_id_rejects_invalid_uuid() {
        let result = PodService::parse_pod_id("not-a-uuid");
        assert!(result.is_err(), "invalid UUID should fail");
        match result {
            Err(ServiceError::PodNotFound(msg)) => {
                assert!(
                    msg.contains("Invalid pod ID"),
                    "expected invalid pod ID message, got: {}",
                    msg
                );
            }
            other => panic!("expected PodNotFound, got {:?}", other),
        }
    }

    // REQ: svc-pod-002 — get_pod_status normalizes not-found errors
    #[tokio::test]
    async fn get_pod_status_returns_not_found_for_missing_pod() {
        let ctx = test_ctx();
        let missing_id = uuid::Uuid::new_v4().to_string();
        let result = PodService::get_pod_status(&ctx, &missing_id).await;
        assert!(
            result.is_err(),
            "get_pod_status should fail for nonexistent pod"
        );
        match result {
            Err(ServiceError::PodNotFound(id)) => {
                assert_eq!(id, missing_id);
            }
            other => panic!("expected PodNotFound, got {:?}", other),
        }
    }

    // REQ: svc-pod-003 — list_pods returns empty list for new manager
    #[tokio::test]
    async fn list_pods_returns_empty_for_new_manager() {
        let ctx = test_ctx();
        let pods = PodService::list_pods(&ctx).await;
        assert!(pods.is_ok(), "list_pods should succeed");
        assert!(pods.unwrap().is_empty(), "new manager should have no pods");
    }

    // REQ: svc-pod-005 — activate_pod normalizes not-found errors
    #[tokio::test]
    async fn activate_pod_returns_not_found_for_missing_pod() {
        let ctx = test_ctx();
        let missing_id = uuid::Uuid::new_v4().to_string();
        let result = PodService::activate_pod(&ctx, &missing_id).await;
        assert!(
            result.is_err(),
            "activate_pod should fail for nonexistent pod"
        );
        match result {
            Err(ServiceError::PodNotFound(id)) => {
                assert_eq!(id, missing_id);
            }
            other => panic!("expected PodNotFound, got {:?}", other),
        }
    }

    // REQ: svc-pod-006 — deactivate_pod normalizes not-found errors (fixes CLI error swallowing)
    #[tokio::test]
    async fn deactivate_pod_returns_not_found_for_missing_pod() {
        let ctx = test_ctx();
        let missing_id = uuid::Uuid::new_v4().to_string();
        let result = PodService::deactivate_pod(&ctx, &missing_id).await;
        assert!(
            result.is_err(),
            "deactivate_pod should fail for nonexistent pod"
        );
        match result {
            Err(ServiceError::PodNotFound(id)) => {
                assert_eq!(id, missing_id);
            }
            other => panic!("expected PodNotFound, got {:?}", other),
        }
    }
}
