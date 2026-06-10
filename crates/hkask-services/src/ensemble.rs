//! EnsembleService — multi-agent session management for CLI and API surfaces.
//!
//! Wraps `AgentService::session_manager()` for chat/deliberation sessions,
//! handles improv turn orchestration, and provides standing session bootstrap.
//! The `CyberneticsLoopGasAdapter` and `build_improv_client()` are moved from
//! `hkask-cli/src/commands/ensemble.rs` — they are business-logic adapters,
//! not CLI presentation code.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use hkask_agents::ensemble::{
    AgentResponse, CircuitBreakerInferenceAdapter, GasGovernancePort, ImprovSessionConfig,
    ImprovTurn, InferencePortAdapter, ParticipantRole, StandingSessionStatus,
    bootstrap_standing_session_with_store,
};
use hkask_cns::{CircuitBreaker, EnergyCost};
use hkask_types::WebID;
use hkask_types::ports::{CircuitBreakerPort, InferencePort};
use tokio::sync::RwLock;

use crate::error::ServiceError;
use crate::{AgentService, InferenceContext, InferenceService};

// ── Gas adapter (moved from CLI) ────────────────────────────────────────────

/// Adapter bridging `CyberneticsLoop` to the ensemble's `GasGovernancePort`.
///
/// Provides synchronous access to the CyberneticsLoop's gas governance by
/// using an atomic counter for `can_proceed` (approximate) and a fire-and-forget
/// task spawn for `acquire` (actual budget consumption via async call).
pub struct CyberneticsLoopGasAdapter {
    loop_ref: Arc<RwLock<hkask_cns::CyberneticsLoop>>,
    agent: WebID,
    gas_used: AtomicU64,
    gas_cap: AtomicU64,
}

impl CyberneticsLoopGasAdapter {
    pub fn new(loop_ref: Arc<RwLock<hkask_cns::CyberneticsLoop>>, agent: WebID, cap: u64) -> Self {
        Self {
            loop_ref,
            agent,
            gas_used: AtomicU64::new(0),
            gas_cap: AtomicU64::new(cap),
        }
    }
}

impl GasGovernancePort for CyberneticsLoopGasAdapter {
    fn can_proceed(&self, gas: u64) -> bool {
        let used = self.gas_used.load(Ordering::Relaxed);
        let cap = self.gas_cap.load(Ordering::Relaxed);
        used.saturating_add(gas) <= cap
    }

    fn acquire(&self, gas: u64) {
        self.gas_used.fetch_add(gas, Ordering::Relaxed);
        let loop_ref = self.loop_ref.clone();
        let agent = self.agent;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let loop_read = loop_ref.read().await;
                let _ = loop_read.acquire_budget(&agent, EnergyCost(gas)).await;
            });
        }
    }
}

// ── Service ─────────────────────────────────────────────────────────────────

/// Service for ensemble session management — delegates to SessionManager.
pub struct EnsembleService;

impl EnsembleService {
    // ── Helpers ──────────────────────────────────────────────────────────

    /// Build an improv inference client from AgentService's inference port.
    pub fn build_improv_client(
        ctx: &AgentService,
        inference_port: Option<Arc<dyn InferencePort>>,
    ) -> Result<Arc<CircuitBreakerInferenceAdapter>, ServiceError> {
        let breaker: Arc<dyn CircuitBreakerPort> =
            Arc::new(CircuitBreaker::default_for_inference("ensemble-inference"));

        let port = match inference_port.or(ctx.inference_port().clone()) {
            Some(p) => p,
            None => {
                let infer_ctx = InferenceContext::from(ctx);
                InferenceService::resolve_port(&infer_ctx, "qwen3:8b")?
            }
        };
        let adapter = InferencePortAdapter::new(port);
        Ok(Arc::new(CircuitBreakerInferenceAdapter::new(
            adapter, breaker,
        )))
    }

    /// Map a role string to ParticipantRole.
    pub fn map_role(role: &str) -> ParticipantRole {
        match role {
            "orchestrator" => ParticipantRole::Curator,
            other => ParticipantRole::Custom(other.to_string()),
        }
    }

    // ── Chat sessions ────────────────────────────────────────────────────

    /// Create a chat session.
    pub async fn create_chat(ctx: &AgentService, session_id: &str) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        manager.create_chat(session_id).await;
        Ok(())
    }

    /// Check if a chat session exists.
    pub async fn has_chat(ctx: &AgentService, session_id: &str) -> Result<bool, ServiceError> {
        let manager = ctx.session_manager().read().await;
        Ok(manager.get_chat(session_id).await.is_some())
    }

    /// List active chat sessions.
    pub async fn list_chats(ctx: &AgentService) -> Result<Vec<String>, ServiceError> {
        let manager = ctx.session_manager().read().await;
        Ok(manager.list_chat_sessions().await)
    }

    /// Register a participant in a chat session.
    pub async fn register_participant(
        ctx: &AgentService,
        session_id: &str,
        role: ParticipantRole,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let participant = hkask_agents::ensemble::ChatParticipant {
            webid: WebID::new(),
            role,
            pod_id: None,
            capabilities: vec![],
        };
        let mut chat_write = chat.write().await;
        chat_write.register_participant(participant);
        Ok(())
    }

    /// Send a message to a chat session.
    pub async fn send_message(
        ctx: &AgentService,
        session_id: &str,
        content: &str,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let msg = hkask_agents::ensemble::ChatMessage::new(WebID::new(), content.to_string());
        let mut chat_write = chat.write().await;
        chat_write.add_message(msg);
        Ok(())
    }

    // ── Improv ───────────────────────────────────────────────────────────

    /// Run an improv turn and record responses in the chat session.
    pub async fn improv_turn(
        ctx: &AgentService,
        session_id: &str,
        user_message: &str,
        inference_port: Option<Arc<dyn InferencePort>>,
    ) -> Result<ImprovTurn, ServiceError> {
        let client = Self::build_improv_client(ctx, inference_port)?;
        let manager = ctx.session_manager().read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let turn = {
            let chat_read = chat.read().await;
            chat_read
                .improv_turn(&client, user_message)
                .await
                .map_err(|e| ServiceError::Improv(e.to_string()))?
        };
        {
            let mut chat_write = chat.write().await;
            let curator_webid = *chat_write.curator();
            chat_write.add_message(hkask_agents::ensemble::ChatMessage::new(
                curator_webid,
                user_message.to_string(),
            ));
            for response in &turn.responses {
                chat_write.add_message(hkask_agents::ensemble::ChatMessage::new(
                    response.agent_webid,
                    response.content.clone(),
                ));
            }
        }
        Ok(turn)
    }

    /// Get improv configuration for a session.
    pub async fn improv_config(
        ctx: &AgentService,
        session_id: &str,
    ) -> Result<ImprovSessionConfig, ServiceError> {
        let manager = ctx.session_manager().read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        Ok(chat.read().await.improv_config().clone())
    }

    /// Set participation threshold for improvisation.
    pub async fn set_participation_threshold(
        ctx: &AgentService,
        session_id: &str,
        threshold: f64,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        chat.write().await.set_participation_threshold(threshold);
        Ok(())
    }

    /// Set improv mode for a session.
    pub async fn set_improv_mode(
        ctx: &AgentService,
        session_id: &str,
        mode: hkask_agents::ensemble::ImprovMode,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        let chat = manager
            .get_chat(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        chat.write().await.set_improv_mode(mode);
        Ok(())
    }

    // ── Deliberation ─────────────────────────────────────────────────────

    /// Create a deliberation session.
    pub async fn create_deliberation(
        ctx: &AgentService,
        session_id: &str,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        manager.create_deliberation(session_id).await;
        Ok(())
    }

    /// Start a deliberation session.
    pub async fn start_deliberation(
        ctx: &AgentService,
        session_id: &str,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        let d = manager
            .get_deliberation(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        d.write().await.start();
        Ok(())
    }

    /// Record a response in a deliberation session.
    pub async fn record_response(
        ctx: &AgentService,
        session_id: &str,
        content: &str,
        confidence: f64,
    ) -> Result<(), ServiceError> {
        let manager = ctx.session_manager().read().await;
        let d = manager
            .get_deliberation(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        let response = AgentResponse::new(WebID::new(), content.to_string(), confidence);
        d.write().await.record_response(response);
        Ok(())
    }

    /// Synthesize deliberation responses.
    pub async fn synthesize_deliberation(
        ctx: &AgentService,
        session_id: &str,
    ) -> Result<String, ServiceError> {
        let manager = ctx.session_manager().read().await;
        let d = manager
            .get_deliberation(session_id)
            .await
            .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;
        Ok(d.read().await.synthesize().synthesized_response)
    }

    /// List deliberation session IDs.
    pub async fn list_deliberations(ctx: &AgentService) -> Result<Vec<String>, ServiceError> {
        let manager = ctx.session_manager().read().await;
        Ok(manager.list_deliberation_sessions().await)
    }

    // ── Standing sessions ────────────────────────────────────────────────

    /// Bootstrap a standing ensemble session from a YAML config file.
    pub fn bootstrap_standing(
        ctx: &AgentService,
        config_path: &std::path::Path,
    ) -> Result<StandingSessionStatus, ServiceError> {
        let store = ctx.standing_session_store().clone();
        let session = bootstrap_standing_session_with_store(config_path, store)
            .map_err(ServiceError::StandingSession)?;
        Ok(session.get_status())
    }
}
