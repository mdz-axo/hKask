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
//! - **Improv operations included** — `improv_turn`, `improv_config`,
//!   `set_threshold`, `set_mode`, and `list_participants` are now service-layer
//!   operations. They normalize the session-not-found pattern that was
//!   duplicated across 5 CLI functions and 1 API route. `improv_turn` is the
//!   deep operation (session lookup + inference + turn + message persistence).
//!   The config/threshold/mode/list operations are thin but benefit from
//!   consistent error handling.
//! - **Participant role mapping** — `register_participant` normalizes the
//!   `"orchestrator" => ParticipantRole::Curator, _ => Custom(role)`
//!   mapping that was duplicated in both CLI and API.

use std::sync::Arc;

use hkask_ensemble::session::SessionManager;
use hkask_ensemble::{
    AgentResponse, ChatMessage, ChatParticipant, CircuitBreakerInferenceAdapter, ImprovMode,
    ImprovSessionConfig, ImprovTurn, ParticipantRole,
};
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

impl From<&crate::ServiceContext> for EnsembleContext {
    fn from(ctx: &crate::ServiceContext) -> Self {
        Self {
            session_manager: ctx.session_manager.clone(),
        }
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

/// Participant info returned by `EnsembleService::list_participants()`.
///
/// Decoupled from `ChatParticipant` which includes `WebID` and `pod_id`
/// that are surface-internal concerns.
#[derive(Debug)]
pub struct ParticipantInfo {
    pub name: String,
    pub role: String,
    pub capabilities: String,
}

/// Ensemble service — chat, deliberation, and improv session operations.
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

    // ── Improv operations ───────────────────────────────────────────────

    /// Execute an improv turn in a chat session.
    ///
    /// Looks up the chat session, runs the improv turn with the provided
    /// inference adapter, and persists the user message and agent responses
    /// back to the chat session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-009 — improv_turn checks session, runs turn, persists messages
    pub async fn improv_turn(
        ctx: &EnsembleContext,
        session_id: &str,
        user_message: &str,
        inference_adapter: &Arc<CircuitBreakerInferenceAdapter>,
    ) -> Result<ImprovTurn, ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;

        let turn = {
            let chat_read = chat.read().await;
            chat_read
                .improv_turn(inference_adapter, user_message)
                .await
                .map_err(|e| ServiceError::Improv(e.to_string()))?
        };

        // Persist user message and agent responses
        {
            let mut chat_write = chat.write().await;
            let curator_webid = *chat_write.curator();
            chat_write.add_message(ChatMessage::new(curator_webid, user_message.to_string()));
            for response in &turn.responses {
                chat_write.add_message(ChatMessage::new(
                    response.agent_webid,
                    response.content.clone(),
                ));
            }
        }

        Ok(turn)
    }

    /// Get the improv configuration for a chat session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-010 — improv_config returns session config
    pub async fn improv_config(
        ctx: &EnsembleContext,
        session_id: &str,
    ) -> Result<ImprovSessionConfig, ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let chat_read = chat.read().await;
        Ok(chat_read.improv_config().clone())
    }

    /// Set the participation threshold for a chat session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-011 — set_improv_threshold updates threshold
    pub async fn set_improv_threshold(
        ctx: &EnsembleContext,
        session_id: &str,
        threshold: f64,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let mut chat_write = chat.write().await;
        chat_write.set_participation_threshold(threshold);
        Ok(())
    }

    /// Set the improv mode for a chat session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-012 — set_improv_mode updates mode
    pub async fn set_improv_mode(
        ctx: &EnsembleContext,
        session_id: &str,
        mode: ImprovMode,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let mut chat_write = chat.write().await;
        chat_write.set_improv_mode(mode);
        Ok(())
    }

    /// List participants in a chat session.
    ///
    /// Returns `ServiceError::SessionNotFound` if the session doesn't exist.
    ///
    /// # REQ: svc-ens-013 — list_participants returns participant info
    pub async fn list_participants(
        ctx: &EnsembleContext,
        session_id: &str,
    ) -> Result<Vec<ParticipantInfo>, ServiceError> {
        let manager = ctx.session_manager.read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let chat_read = chat.read().await;
        let participants = chat_read.get_participants();
        let result = participants
            .values()
            .map(|p| ParticipantInfo {
                name: format!("{:?}", p.role),
                role: format!("{:?}", p.role),
                capabilities: if p.capabilities.is_empty() {
                    "none".to_string()
                } else {
                    p.capabilities.join(", ")
                },
            })
            .collect();
        Ok(result)
    }

    /// Get a chat session by ID.
    ///
    /// # REQ: svc-ens-014 — get_chat returns session existence check via SessionManager
    pub async fn get_chat(ctx: &EnsembleContext, session_id: &str) -> Result<(), ServiceError> {
        let manager = ctx.session_manager.read().await;
        manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        Ok(())
    }

    /// List all deliberation session IDs.
    ///
    /// # REQ: svc-ens-015 — list_deliberations returns all deliberation session IDs
    pub async fn list_deliberations(ctx: &EnsembleContext) -> Vec<String> {
        let manager = ctx.session_manager.read().await;
        manager.list_deliberation_sessions().await
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

    // REQ: svc-ens-010 — improv_config returns session config
    #[tokio::test]
    async fn improv_config_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::improv_config(&ctx, "nonexistent").await;
        assert!(
            result.is_err(),
            "improv_config should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-011 — set_improv_threshold checks session existence
    #[tokio::test]
    async fn set_improv_threshold_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::set_improv_threshold(&ctx, "nonexistent", 0.5).await;
        assert!(
            result.is_err(),
            "set_improv_threshold should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-012 — set_improv_mode checks session existence
    #[tokio::test]
    async fn set_improv_mode_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result =
            EnsembleService::set_improv_mode(&ctx, "nonexistent", ImprovMode::Freeform).await;
        assert!(
            result.is_err(),
            "set_improv_mode should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-013 — list_participants checks session existence
    #[tokio::test]
    async fn list_participants_returns_not_found_for_missing_session() {
        let ctx = test_ctx();
        let result = EnsembleService::list_participants(&ctx, "nonexistent").await;
        assert!(
            result.is_err(),
            "list_participants should fail for nonexistent session"
        );
        match result {
            Err(ServiceError::SessionNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("expected SessionNotFound, got {:?}", other),
        }
    }

    // REQ: svc-ens-010 — improv_config succeeds for existing session
    #[tokio::test]
    async fn improv_config_succeeds_for_existing_session() {
        let ctx = test_ctx();
        EnsembleService::create_chat(&ctx, "test-chat")
            .await
            .unwrap();
        let result = EnsembleService::improv_config(&ctx, "test-chat").await;
        assert!(
            result.is_ok(),
            "improv_config should succeed for existing session"
        );
    }

    // REQ: svc-ens-013 — list_participants returns curator for new session
    #[tokio::test]
    async fn list_participants_returns_empty_for_new_session() {
        let ctx = test_ctx();
        EnsembleService::create_chat(&ctx, "test-chat")
            .await
            .unwrap();
        let result = EnsembleService::list_participants(&ctx, "test-chat").await;
        assert!(result.is_ok(), "list_participants should succeed");
        // New sessions include the curator as a default participant
        assert!(
            !result.unwrap().is_empty(),
            "new session should have curator participant"
        );
    }
}
