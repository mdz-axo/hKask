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
    ChatMessage, CircuitBreakerInferenceAdapter, GasGovernancePort, ImprovMode,
    ImprovSessionConfig, InferencePortAdapter, bootstrap_standing_session_with_store,
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

pub async fn ensemble_chat_create(ctx: &ServiceContext, session: String) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::create_chat(&ens_ctx, &session)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Chat session '{}' created", session))
}

/// Register bot in chat
pub async fn ensemble_chat_register(
    ctx: &ServiceContext,
    session: String,
    bot: String,
    role: String,
) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::register_participant(
        &ens_ctx,
        &session,
        WebID::new(),
        &role,
        vec![],
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(format!(
        "Bot '{}' registered as {} in session '{}'",
        bot, role, session
    ))
}

/// Send message to chat
pub async fn ensemble_chat_send(
    ctx: &ServiceContext,
    session: String,
    message: String,
) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::send_message(&ens_ctx, &session, WebID::new(), &message)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Message sent".to_string())
}

/// List chat sessions
pub async fn ensemble_chat_list(ctx: &ServiceContext) -> Result<Vec<String>, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::list_chat_sessions(&ens_ctx)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_improv_turn(
    ctx: &ServiceContext,
    session_id: &str,
    user_message: &str,
    inference_port: Option<Arc<dyn InferencePort>>,
) -> Result<hkask_ensemble::ImprovTurn, String> {
    let manager = ctx.session_manager.clone();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let client = build_improv_client(ctx, inference_port);
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
    let manager = ctx.session_manager.clone();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let chat_read = chat.read().await;
    Ok(chat_read.improv_config().clone())
}

pub async fn ensemble_improv_set_threshold(
    ctx: &ServiceContext,
    session_id: &str,
    threshold: f64,
) -> Result<(), String> {
    let manager = ctx.session_manager.clone();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let mut chat_write = chat.write().await;
    chat_write.set_participation_threshold(threshold);
    Ok(())
}

pub async fn ensemble_improv_set_mode(
    ctx: &ServiceContext,
    session_id: &str,
    mode: ImprovMode,
) -> Result<(), String> {
    let manager = ctx.session_manager.clone();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let mut chat_write = chat.write().await;
    chat_write.set_improv_mode(mode);
    Ok(())
}

pub async fn ensemble_participants(
    ctx: &ServiceContext,
    session_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let manager = ctx.session_manager.clone();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let chat_read = chat.read().await;
    let participants = chat_read.get_participants();
    let mut result = Vec::new();
    for p in participants.values() {
        let name = format!("{:?}", p.role);
        let role_str = format!("{:?}", p.role);
        let caps = if p.capabilities.is_empty() {
            "none".to_string()
        } else {
            p.capabilities.join(", ")
        };
        result.push((name, role_str, caps));
    }
    Ok(result)
}

pub async fn ensemble_deliberation_create(
    ctx: &ServiceContext,
    session: String,
) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::create_deliberation(&ens_ctx, &session)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Deliberation session '{}' created", session))
}

pub async fn ensemble_deliberation_start(
    ctx: &ServiceContext,
    session: String,
) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::start_deliberation(&ens_ctx, &session)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Deliberation started".to_string())
}

pub async fn ensemble_deliberation_record(
    ctx: &ServiceContext,
    session: String,
    _agent: String,
    content: String,
    confidence: f64,
) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::record_deliberation_response(
        &ens_ctx,
        &session,
        WebID::new(),
        content,
        confidence,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok("Response recorded".to_string())
}

pub async fn ensemble_deliberation_synthesize(
    ctx: &ServiceContext,
    session: String,
) -> Result<String, String> {
    let ens_ctx = hkask_services::EnsembleContext::from(ctx);
    hkask_services::EnsembleService::synthesize_deliberation(&ens_ctx, &session)
        .await
        .map_err(|e| e.to_string())
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

/// CLI handler for `kask ensemble` subcommand
pub fn run_ensemble(rt: &tokio::runtime::Runtime, action: crate::cli::EnsembleAction) {
    use crate::commands;

    match action {
        EnsembleAction::ChatCreate { session } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::ensemble_chat_create(&ctx, session.clone()),
                    "Chat create failed"
                )
            );
        }
        EnsembleAction::ChatRegister { session, bot, role } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::ensemble_chat_register(
                        &ctx,
                        session.clone(),
                        bot.clone(),
                        role.clone(),
                    ),
                    "Chat register failed"
                )
            );
        }
        EnsembleAction::ChatSend { session, message } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::ensemble_chat_send(&ctx, session.clone(), message.clone(),),
                    "Chat send failed"
                )
            );
        }
        EnsembleAction::ChatList => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            let sessions = block_on!(rt, commands::ensemble_chat_list(&ctx), "Chat list failed");
            println!("Active chat sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::DeliberationCreate { session } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::ensemble_deliberation_create(&ctx, session.clone()),
                    "Deliberation create failed"
                )
            );
        }
        EnsembleAction::DeliberationStart { session } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::ensemble_deliberation_start(&ctx, session.clone()),
                    "Deliberation start failed"
                )
            );
        }
        EnsembleAction::DeliberationRecord {
            session,
            agent,
            content,
            confidence,
        } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::ensemble_deliberation_record(
                        &ctx,
                        session.clone(),
                        agent.clone(),
                        content.clone(),
                        confidence,
                    ),
                    "Deliberation record failed"
                )
            );
        }
        EnsembleAction::DeliberationSynthesize { session } => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            println!(
                "Synthesized response:\n{}",
                block_on!(
                    rt,
                    commands::ensemble_deliberation_synthesize(&ctx, session.clone()),
                    "Deliberation synthesize failed"
                )
            );
        }
        EnsembleAction::DeliberationList => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
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
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            let status = super::helpers::or_exit(
                commands::ensemble_standing_start(&ctx, &config),
                "Standing session bootstrap failed",
            );
            println!("Standing session bootstrapped:");
            println!("  Session ID: {}", status.session_id);
            println!("  Participants: {}", status.participant_count);
            println!("  Initial messages: {}", status.message_count);
        }
        EnsembleAction::StandingStatus => {
            let ctx =
                super::helpers::or_exit(build_service_context(), "Failed to build service context");
            let status = super::helpers::or_exit(
                commands::ensemble_standing_status(&ctx),
                "Standing status failed",
            );
            println!("Standing session status:");
            println!("  Session ID: {}", status.session_id);
            println!("  Participants: {}", status.participant_count);
            println!("  Messages: {}", status.message_count);
            println!("\nParticipants:");
            for p in &status.participants {
                println!("  - {} ({})", p.name, p.role);
            }
        }
    }
}
