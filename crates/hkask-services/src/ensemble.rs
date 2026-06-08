//! Ensemble service — multi-agent session coordination.
//!
//! `EnsembleService` replaces duplicated session-not-found handling and
//! participant role mapping across CLI and API surfaces. Each surface
//! constructs an `EnsembleContext` from its own state and delegates
//! business logic to this service.
//!
//! # Design decisions
//!
//! - **Constraint: Prohibition (P1)** — MCP servers do NOT use this service.
//!   They continue using `SessionManager` directly because they run in
//!   separate processes and cannot share `EnsembleContext`.
//! - **Depth test** — Deleting this module would cause session-not-found
//!   error handling and participant role mapping to reappear in 10+ call
//!   sites across CLI and API. Passes deletion test.
//! - **Strangler fig** — `EnsembleContext` holds only `session_manager`.
//!   Chat and deliberation operations need only the session manager.
//!   Standing session operations are divergent (CLI reads YAML; API takes
//!   JSON body with MCP discovery) and remain surface-specific for now.
//! - **Standing sessions excluded** — `standing_start` and `standing_status`
//!   are classified as **Divergent** (CLI: YAML file bootstrap, API: JSON
//!   body + MCP tool discovery + gas governance wiring). Unifying them would
//!   require parameterizing surface-specific logic that adds more complexity
//!   than it removes. Standing sessions stay in surface code until a future
//!   normalization pass.
//! - **Improv operations excluded** — `improv_turn`, `improv_config`,
//!   `set_threshold`, `set_mode` are either divergent (improv_turn needs
//!   surface-specific inferencer) or surface-only (CLI-only). Not extracted.
//! - **Participant role mapping** — `register_participant` normalizes the
//!   `"orchestrator" => ParticipantRole::Curator, _ => Custom(role)`
//!   mapping that was duplicated in both CLI and API.

use std::sync::Arc;

use hkask_ensemble::session::SessionManager;
use hkask_ensemble::{AgentResponse, ChatMessage, ChatParticipant, ParticipantRole};
use hkask_types::WebID;
use tokio::sync::RwLock;

use crate::ServiceError;

/// Lightweight context for `EnsembleService` calls.
///
/// Contains only the session manager needed for chat and deliberation
/// operations. Surfaces construct this from their own state (CLI uses
/// the global `SESSION_MANAGER`, API uses `ApiState.session_manager`).
///
/// Standing session operations are not covered — they require
/// `StandingSessionStore` and `GasGovernancePort` which are surface-specific.
pub struct EnsembleContext {
    /// Session manager for chat and deliberation sessions.
    pub session_manager: Arc<RwLock<SessionManager>>,
}

impl EnsembleContext {
    /// Construct from individual parts.
    ///
    /// Surfaces pass their `SessionManager` instance:
    /// ```ignore
    /// let ctx = EnsembleContext::from_parts(session_manager);
    /// ```
    pub fn from_parts(session_manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { session_manager }
    }
}

/// Map a role string to a `ParticipantRole`.
///
/// Normalizes the role mapping that was duplicated in both CLI and API:
/// - `"orchestrator"` maps to `ParticipantRole::Curator`
/// - Any other string maps to `ParticipantRole::Custom(role)`
///
/// # REQ: svc-ens-003a — role mapping normalizes orchestrator to Curator
pub fn map_participant_role(role: &str) -> ParticipantRole {
    match role {
        "orchestrator" => ParticipantRole::Curator,
        other => ParticipantRole::Custom(other.to_string()),
    }
}

/// Ensemble service — chat and deliberation session operations.
///
/// Use `EnsembleService::create_chat()` etc. to delegate ensemble
/// operations through the service layer. Surfaces construct an
/// `EnsembleContext` from their own state and call service methods.
pub struct EnsembleService;

impl EnsembleService {
    /// Create a new chat session.
    ///
    /// # REQ: svc-ens-001 — create_chat creates a chat session via SessionManager
    pub async fn create_chat(ctx: &EnsembleContext, session_id: &str) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        manager.create_chat(session_id).await;
        Ok(())
    }

    /// List all active chat session IDs.
    ///
    /// # REQ: svc-ens-002 — list_chat_sessions returns all chat session IDs
    pub async fn list_chat_sessions(ctx: &EnsembleContext) -> Result<Vec<String>, ServiceError> {
        let manager = ctx.session_manager.read().await;
        Ok(manager.list_chat_sessions().await)
    }

    /// Register a participant in a chat session.
    ///
    /// Normalizes the participant role mapping (`"orchestrator"` →
    /// `ParticipantRole::Curator`, anything else →
    /// `ParticipantRole::Custom(role)`). Returns
    /// `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-003 — register_participant normalizes role and checks existence
    pub async fn register_participant(
        ctx: &EnsembleContext,
        session_id: &str,
        webid: WebID,
        role: &str,
        capabilities: Vec<String>,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let participant_role = map_participant_role(role);
        let mut chat_write = chat.write().await;
        chat_write.register_participant(ChatParticipant {
            webid,
            role: participant_role,
            pod_id: None,
            capabilities,
        });
        Ok(())
    }

    /// Send a message to a chat session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-004 — send_message checks session existence before sending
    pub async fn send_message(
        ctx: &EnsembleContext,
        session_id: &str,
        sender_webid: WebID,
        content: &str,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let mut chat_write = chat.write().await;
        let msg = ChatMessage::new(sender_webid, content.to_string());
        chat_write.add_message(msg);
        Ok(())
    }

    /// Create a new deliberation session.
    ///
    /// # REQ: svc-ens-005 — create_deliberation creates a deliberation session via SessionManager
    pub async fn create_deliberation(
        ctx: &EnsembleContext,
        session_id: &str,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        manager.create_deliberation(session_id).await;
        Ok(())
    }

    /// Start a deliberation session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-006 — start_deliberation checks existence before starting
    pub async fn start_deliberation(
        ctx: &EnsembleContext,
        session_id: &str,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        let deliberation = manager
            .get_deliberation(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let mut session_write = deliberation.write().await;
        session_write.start();
        Ok(())
    }

    /// Record a response in a deliberation session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-007 — record_response checks existence before recording
    pub async fn record_deliberation_response(
        ctx: &EnsembleContext,
        session_id: &str,
        agent_webid: WebID,
        content: String,
        confidence: f64,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        let deliberation = manager
            .get_deliberation(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let response = AgentResponse::new(agent_webid, content, confidence);
        let mut session_write = deliberation.write().await;
        session_write.record_response(response);
        Ok(())
    }

    /// Synthesize deliberation responses.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-008 — synthesize_deliberation checks existence before synthesizing
    pub async fn synthesize_deliberation(
        ctx: &EnsembleContext,
        session_id: &str,
    ) -> Result<String, ServiceError> {
        let manager = ctx.session_manager.read().await;
        let deliberation = manager
            .get_deliberation(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let session_read = deliberation.read().await;
        let result = session_read.synthesize();
        Ok(result.synthesized_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session_manager() -> Arc<RwLock<SessionManager>> {
        Arc::new(RwLock::new(SessionManager::new(WebID::new())))
    }

    fn test_ctx() -> EnsembleContext {
        EnsembleContext::from_parts(test_session_manager())
    }

    // REQ: svc-ens-001 — create_chat creates a chat session via SessionManager
    #[tokio::test]
    async fn create_chat_creates_session() {
        let ctx = test_ctx();
        let result = EnsembleService::create_chat(&ctx, "test-session").await;
        assert!(result.is_ok(), "create_chat should succeed");

        // Verify the session exists
        let manager = ctx.session_manager.read().await;
        let chat = manager.get_chat("test-session").await;
        assert!(chat.is_some(), "created session should be retrievable");
    }

    // REQ: svc-ens-002 — list_chat_sessions returns all chat session IDs
    #[tokio::test]
    async fn list_chat_sessions_returns_ids() {
        let ctx = test_ctx();
        EnsembleService::create_chat(&ctx, "session-1")
            .await
            .unwrap();
        EnsembleService::create_chat(&ctx, "session-2")
            .await
            .unwrap();

        let sessions = EnsembleService::list_chat_sessions(&ctx).await.unwrap();
        assert_eq!(sessions.len(), 2, "should have 2 sessions");
        assert!(
            sessions.contains(&"session-1".to_string()),
            "should contain session-1"
        );
        assert!(
            sessions.contains(&"session-2".to_string()),
            "should contain session-2"
        );
    }

    // REQ: svc-ens-003 — register_participant normalizes role and checks existence
    #[tokio::test]
    async fn register_participant_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::register_participant(
            &ctx,
            "nonexistent",
            WebID::new(),
            "orchestrator",
            vec![],
        )
        .await;
        assert!(
            result.is_err(),
            "register_participant should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-003a — role mapping normalizes orchestrator to Curator
    #[test]
    fn map_participant_role_normalizes_orchestrator() {
        assert!(matches!(
            map_participant_role("orchestrator"),
            ParticipantRole::Curator
        ));
        assert!(matches!(
            map_participant_role("expert"),
            ParticipantRole::Custom(ref r) if r == "expert"
        ));
    }

    // REQ: svc-ens-004 — send_message checks session existence before sending
    #[tokio::test]
    async fn send_message_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result =
            EnsembleService::send_message(&ctx, "nonexistent", WebID::new(), "hello").await;
        assert!(
            result.is_err(),
            "send_message should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-005 — create_deliberation creates a deliberation session via SessionManager
    #[tokio::test]
    async fn create_deliberation_creates_session() {
        let ctx = test_ctx();
        let result = EnsembleService::create_deliberation(&ctx, "delib-1").await;
        assert!(result.is_ok(), "create_deliberation should succeed");

        let manager = ctx.session_manager.read().await;
        let delib = manager.get_deliberation("delib-1").await;
        assert!(
            delib.is_some(),
            "created deliberation should be retrievable"
        );
    }

    // REQ: svc-ens-006 — start_deliberation checks existence before starting
    #[tokio::test]
    async fn start_deliberation_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::start_deliberation(&ctx, "nonexistent").await;
        assert!(
            result.is_err(),
            "start_deliberation should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-007 — record_response checks existence before recording
    #[tokio::test]
    async fn record_response_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::record_deliberation_response(
            &ctx,
            "nonexistent",
            WebID::new(),
            "response text".to_string(),
            0.95,
        )
        .await;
        assert!(
            result.is_err(),
            "record_response should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-008 — synthesize_deliberation checks existence before synthesizing
    #[tokio::test]
    async fn synthesize_deliberation_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::synthesize_deliberation(&ctx, "nonexistent").await;
        assert!(
            result.is_err(),
            "synthesize should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // Integration: register_participant succeeds for existing session
    #[tokio::test]
    async fn register_participant_succeeds_for_existing_session() {
        let ctx = test_ctx();
        EnsembleService::create_chat(&ctx, "test-chat")
            .await
            .unwrap();

        let result = EnsembleService::register_participant(
            &ctx,
            "test-chat",
            WebID::new(),
            "orchestrator",
            vec![],
        )
        .await;
        assert!(result.is_ok(), "register_participant should succeed");
    }

    // Integration: send_message succeeds for existing session
    #[tokio::test]
    async fn send_message_succeeds_for_existing_session() {
        let ctx = test_ctx();
        EnsembleService::create_chat(&ctx, "test-chat")
            .await
            .unwrap();

        let result =
            EnsembleService::send_message(&ctx, "test-chat", WebID::new(), "hello world").await;
        assert!(result.is_ok(), "send_message should succeed");
    }
}
