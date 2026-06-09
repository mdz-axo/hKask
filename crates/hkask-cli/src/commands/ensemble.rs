//! Ensemble command handlers — chat, deliberation, improv, and standing sessions
//!
//! Manages multi-agent ensemble sessions via `ServiceContext`. Session manager,
//! cybernetics loop, and standing session store come from ServiceContext rather
//! than global statics. The improv client is constructed per-call from
//! ServiceContext's inference port + circuit breaker.

use crate::block_on;
use crate::cli::EnsembleAction;
use hkask_cns::{CircuitBreaker, GasCost};
use hkask_ensemble::{
    AgentResponse, ChatMessage, ChatParticipant, CircuitBreakerInferenceAdapter, GasGovernancePort,
    ImprovMode, ImprovSessionConfig, ImprovTurn, InferencePortAdapter, ParticipantRole,
    bootstrap_standing_session_with_store,
};
use hkask_services::ServiceContext;
use hkask_types::WebID;
use hkask_types::ports::{CircuitBreakerPort, InferencePort};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::RwLock;

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
    /// Create a new gas adapter wrapping a CyberneticsLoop for a specific agent.
    ///
    /// The `gas_cap` is initialized from the CyberneticsLoop's registered budget
    /// for the agent. If no budget is registered, defaults to u64::MAX (no limit).
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
        // Fire-and-forget: report to CyberneticsLoop asynchronously
        let loop_ref = self.loop_ref.clone();
        let agent = self.agent;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let loop_read = loop_ref.read().await;
                let _ = loop_read.acquire_budget(&agent, GasCost(gas)).await;
            });
        }
    }
}

/// Create an improv client from ServiceContext's inference port + circuit breaker.
pub(crate) fn build_improv_client(
    ctx: &ServiceContext,
    inference_port: Option<Arc<dyn InferencePort>>,
) -> Arc<CircuitBreakerInferenceAdapter> {
    let breaker: Arc<dyn CircuitBreakerPort> =
        Arc::new(CircuitBreaker::default_for_inference("ensemble-inference"));

    match inference_port.or(ctx.inference_port.clone()) {
        Some(port) => {
            let adapter = InferencePortAdapter::new(port);
            Arc::new(CircuitBreakerInferenceAdapter::new(adapter, breaker))
        }
        None => {
            let infer_ctx = hkask_services::InferenceContext::from(ctx);
            let port = hkask_services::InferenceService::resolve_port(&infer_ctx, "qwen3:8b")
                .expect("Failed to create Okapi inference");
            let adapter = InferencePortAdapter::new(port);
            Arc::new(CircuitBreakerInferenceAdapter::new(adapter, breaker))
        }
    }
}

fn map_ensemble_role(role: &str) -> ParticipantRole {
    match role {
        "orchestrator" => ParticipantRole::Curator,
        other => ParticipantRole::Custom(other.to_string()),
    }
}

pub async fn ensemble_chat_create(ctx: &ServiceContext, session: String) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    manager.create_chat(&session).await;
    Ok(format!("Chat session '{}' created", session))
}

pub async fn ensemble_chat_register(
    ctx: &ServiceContext,
    session: String,
    bot: String,
    role: String,
) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(&session)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session))?;
    let participant = ChatParticipant {
        webid: WebID::new(),
        role: map_ensemble_role(&role),
        pod_id: None,
        capabilities: vec![],
    };
    let mut chat_write = chat.write().await;
    chat_write.register_participant(participant);
    Ok(format!(
        "Bot '{}' registered as {} in session '{}'",
        bot, role, session
    ))
}

pub async fn ensemble_chat_send(
    ctx: &ServiceContext,
    session: String,
    message: String,
) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(&session)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session))?;
    let msg = ChatMessage::new(WebID::new(), message);
    let mut chat_write = chat.write().await;
    chat_write.add_message(msg);
    Ok("Message sent".to_string())
}

pub async fn ensemble_chat_list(ctx: &ServiceContext) -> Result<Vec<String>, String> {
    let manager = ctx.session_manager.read().await;
    Ok(manager.list_chat_sessions().await)
}

pub async fn ensemble_improv_turn(
    ctx: &ServiceContext,
    session_id: &str,
    user_message: &str,
    inference_port: Option<Arc<dyn InferencePort>>,
) -> Result<ImprovTurn, String> {
    let client = build_improv_client(ctx, inference_port);
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(session_id)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    let turn = {
        let chat_read = chat.read().await;
        chat_read
            .improv_turn(&client, user_message)
            .await
            .map_err(|e| format!("Improv error: {}", e))?
    };
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

pub async fn ensemble_improv_config(
    ctx: &ServiceContext,
    session_id: &str,
) -> Result<ImprovSessionConfig, String> {
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(session_id)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    Ok(chat.read().await.improv_config().clone())
}

pub async fn ensemble_improv_set_threshold(
    ctx: &ServiceContext,
    session_id: &str,
    threshold: f64,
) -> Result<(), String> {
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(session_id)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    chat.write().await.set_participation_threshold(threshold);
    Ok(())
}

pub async fn ensemble_improv_set_mode(
    ctx: &ServiceContext,
    session_id: &str,
    mode: ImprovMode,
) -> Result<(), String> {
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(session_id)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    chat.write().await.set_improv_mode(mode);
    Ok(())
}

pub async fn ensemble_participants(
    ctx: &ServiceContext,
    session_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let manager = ctx.session_manager.read().await;
    let chat = manager
        .get_chat(session_id)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    let chat_read = chat.read().await;
    let participants = chat_read.get_participants();
    Ok(participants
        .values()
        .map(|p| {
            let role = format!("{:?}", p.role);
            (
                role.clone(),
                role,
                if p.capabilities.is_empty() {
                    "none".into()
                } else {
                    p.capabilities.join(", ")
                },
            )
        })
        .collect())
}

pub async fn ensemble_deliberation_create(
    ctx: &ServiceContext,
    session: String,
) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    manager.create_deliberation(&session).await;
    Ok(format!("Deliberation session '{}' created", session))
}

pub async fn ensemble_deliberation_start(
    ctx: &ServiceContext,
    session: String,
) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    let d = manager
        .get_deliberation(&session)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session))?;
    d.write().await.start();
    Ok("Deliberation started".to_string())
}

pub async fn ensemble_deliberation_record(
    ctx: &ServiceContext,
    session: String,
    _agent: String,
    content: String,
    confidence: f64,
) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    let d = manager
        .get_deliberation(&session)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session))?;
    let response = AgentResponse::new(WebID::new(), content, confidence);
    d.write().await.record_response(response);
    Ok("Response recorded".to_string())
}

pub async fn ensemble_deliberation_synthesize(
    ctx: &ServiceContext,
    session: String,
) -> Result<String, String> {
    let manager = ctx.session_manager.read().await;
    let d = manager
        .get_deliberation(&session)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session))?;
    let result = d.read().await.synthesize();
    Ok(result.synthesized_response)
}

pub async fn ensemble_deliberation_list(ctx: &ServiceContext) -> Result<Vec<String>, String> {
    // List deliberations is a thin delegation that doesn't normalize errors.
    // It stays as a direct SessionManager call because deleting this service
    // call wouldn't cause complexity to reappear in 8+ call sites.
    let manager = ctx.session_manager.clone();
    let sessions = {
        let manager_read = manager.read().await;
        manager_read.list_deliberation_sessions().await
    };
    Ok(sessions)
}

/// Bootstrap the standing ensemble session from a YAML manifest.
pub fn ensemble_standing_start(
    ctx: &ServiceContext,
    config_path: &std::path::Path,
) -> Result<hkask_ensemble::StandingSessionStatus, crate::errors::EnsembleError> {
    let store = ctx.standing_session_store.clone();
    let session = bootstrap_standing_session_with_store(config_path, store)?;
    Ok(session.get_status())
}

/// Get the current standing session status.
pub fn ensemble_standing_status(
    ctx: &ServiceContext,
) -> Result<hkask_ensemble::StandingSessionStatus, crate::errors::EnsembleError> {
    let config_path = std::path::Path::new("registry/manifests/standing-ensemble-session.yaml");
    if !config_path.exists() {
        return Err(crate::errors::EnsembleError::SessionNotFound(
            "Standing session not bootstrapped. Run 'kask ensemble standing-start' first."
                .to_string(),
        ));
    }

    let store = ctx.standing_session_store.clone();
    let session = bootstrap_standing_session_with_store(config_path, store)?;
    Ok(session.get_status())
}

/// Build a ServiceContext for standalone CLI ensemble commands.
fn build_service_context() -> Result<hkask_services::ServiceContext, crate::errors::EnsembleError> {
    let config = hkask_services::ServiceConfig::from_env().map_err(|e| {
        crate::errors::EnsembleError::SessionNotFound(format!("Config error: {}", e))
    })?;
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        crate::errors::EnsembleError::SessionNotFound(format!("Runtime error: {}", e))
    })?;
    rt.block_on(hkask_services::ServiceContext::build(config))
        .map_err(|e| {
            crate::errors::EnsembleError::SessionNotFound(format!("ServiceContext error: {}", e))
        })
}

/// Shorthand: build context and execute an async closure, printing the result.
fn with_ensemble_ctx(
    rt: &tokio::runtime::Runtime,
    label: &'static str,
    f: impl std::future::Future<Output = Result<String, String>>,
) {
    let _ctx = super::helpers::or_exit(build_service_context(), "Failed to build service context");
    println!("{}", super::helpers::or_exit(rt.block_on(f), label));
}

/// CLI handler for `kask ensemble` subcommand
pub fn run_ensemble(rt: &tokio::runtime::Runtime, action: crate::cli::EnsembleAction) {
    use crate::commands;
    let build_ctx =
        || super::helpers::or_exit(build_service_context(), "Failed to build service context");

    match action {
        EnsembleAction::ChatCreate { session } => {
            with_ensemble_ctx(rt, "Chat create failed", async move {
                let ctx = build_ctx();
                commands::ensemble_chat_create(&ctx, session).await
            });
        }
        EnsembleAction::ChatRegister { session, bot, role } => {
            with_ensemble_ctx(rt, "Chat register failed", async move {
                let ctx = build_ctx();
                commands::ensemble_chat_register(&ctx, session, bot, role).await
            });
        }
        EnsembleAction::ChatSend { session, message } => {
            with_ensemble_ctx(rt, "Chat send failed", async move {
                let ctx = build_ctx();
                commands::ensemble_chat_send(&ctx, session, message).await
            });
        }
        EnsembleAction::ChatList => {
            let ctx = build_ctx();
            let sessions = block_on!(rt, commands::ensemble_chat_list(&ctx), "Chat list failed");
            println!("Active chat sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::DeliberationCreate { session } => {
            with_ensemble_ctx(rt, "Deliberation create failed", async move {
                let ctx = build_ctx();
                commands::ensemble_deliberation_create(&ctx, session).await
            });
        }
        EnsembleAction::DeliberationStart { session } => {
            with_ensemble_ctx(rt, "Deliberation start failed", async move {
                let ctx = build_ctx();
                commands::ensemble_deliberation_start(&ctx, session).await
            });
        }
        EnsembleAction::DeliberationRecord {
            session,
            agent,
            content,
            confidence,
        } => {
            with_ensemble_ctx(rt, "Deliberation record failed", async move {
                let ctx = build_ctx();
                commands::ensemble_deliberation_record(&ctx, session, agent, content, confidence)
                    .await
            });
        }
        EnsembleAction::DeliberationSynthesize { session } => {
            let ctx = build_ctx();
            println!(
                "Synthesized response:\n{}",
                block_on!(
                    rt,
                    commands::ensemble_deliberation_synthesize(&ctx, session),
                    "Deliberation synthesize failed"
                )
            );
        }
        EnsembleAction::DeliberationList => {
            let ctx = build_ctx();
            let sessions = block_on!(
                rt,
                commands::ensemble_deliberation_list(&ctx),
                "Deliberation list failed"
            );
            println!("Active deliberation sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::StandingStart { config } => {
            let ctx = build_ctx();
            let status = super::helpers::or_exit(
                commands::ensemble_standing_start(&ctx, &config),
                "Standing session bootstrap failed",
            );
            println!(
                "Standing session bootstrapped:\n  Session ID: {}\n  Participants: {}\n  Initial messages: {}",
                status.session_id, status.participant_count, status.message_count
            );
        }
        EnsembleAction::StandingStatus => {
            let ctx = build_ctx();
            let status = super::helpers::or_exit(
                commands::ensemble_standing_status(&ctx),
                "Standing status failed",
            );
            println!(
                "Standing session status:\n  Session ID: {}\n  Participants: {}\n  Messages: {}",
                status.session_id, status.participant_count, status.message_count
            );
            println!("\nParticipants:");
            for p in &status.participants {
                println!("  - {} ({})", p.name, p.role);
            }
        }
    }
}
