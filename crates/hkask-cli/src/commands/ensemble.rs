//! Ensemble command handlers — chat, deliberation, improv, and standing sessions
//!
//! Manages multi-agent ensemble sessions via singleton patterns for session
//! manager and improv client. Also handles standing session bootstrap via
//! hkask-ensemble registry manifests.

use crate::cli::EnsembleAction;
use hkask_cns::{CircuitBreaker, CyberneticsLoop};
use hkask_ensemble::{
    AgentResponse, ChatMessage, ChatParticipant, CircuitBreakerInferenceAdapter, GasGovernancePort,
    ImprovMode, ImprovSessionConfig, InferencePortAdapter, ParticipantRole, SessionManager,
    bootstrap_standing_session_with_store,
};
use hkask_templates::OkapiConfig;
use hkask_templates::OkapiInference;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::ports::{CircuitBreakerPort, InferencePort, StandingSessionPort};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::RwLock;

static SESSION_MANAGER: std::sync::OnceLock<Arc<RwLock<SessionManager>>> =
    std::sync::OnceLock::new();
static IMPROV_CLIENT: std::sync::OnceLock<Arc<CircuitBreakerInferenceAdapter>> =
    std::sync::OnceLock::new();

/// Adapter bridging `CyberneticsLoop` to the ensemble's `GasGovernancePort`.
///
/// Provides synchronous access to the CyberneticsLoop's gas governance by
/// using an atomic counter for `can_proceed` (approximate) and a fire-and-forget
/// task spawn for `acquire` (actual budget consumption via async call).
pub struct CyberneticsLoopGasAdapter {
    loop_ref: Arc<RwLock<CyberneticsLoop>>,
    agent: WebID,
    gas_used: AtomicU64,
    gas_cap: AtomicU64,
}

impl CyberneticsLoopGasAdapter {
    /// Create a new gas adapter wrapping a CyberneticsLoop for a specific agent.
    ///
    /// The `gas_cap` is initialized from the CyberneticsLoop's registered budget
    /// for the agent. If no budget is registered, defaults to u64::MAX (no limit).
    pub fn new(loop_ref: Arc<RwLock<CyberneticsLoop>>, agent: WebID, cap: u64) -> Self {
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
                let _ = loop_read.acquire_budget(&agent, gas).await;
            });
        }
    }
}

static CYBERNETICS_LOOP: std::sync::OnceLock<Arc<RwLock<CyberneticsLoop>>> =
    std::sync::OnceLock::new();

fn get_cybernetics_loop() -> Arc<RwLock<CyberneticsLoop>> {
    CYBERNETICS_LOOP
        .get_or_init(|| {
            let cns = Arc::new(RwLock::new(hkask_cns::CnsRuntime::default()));
            let (dispatch_tx, _) = tokio::sync::mpsc::unbounded_channel();
            let event_sink: Arc<dyn NuEventSink> = Arc::new(hkask_storage::NuEventStore::new(
                hkask_storage::Database::in_memory()
                    .expect("ensemble event db")
                    .conn_arc(),
            ));
            Arc::new(RwLock::new(
                CyberneticsLoop::new(cns, dispatch_tx).with_event_sink(event_sink),
            ))
        })
        .clone()
}

/// Default gas cap for ensemble sessions (150k = same as GasBudgetConfig default)
const ENSEMBLE_GAS_CAP: u64 = 150_000;

pub fn get_session_manager() -> Arc<RwLock<SessionManager>> {
    SESSION_MANAGER
        .get_or_init(|| {
            let webid = WebID::new();
            let governance: Arc<dyn GasGovernancePort> = Arc::new(CyberneticsLoopGasAdapter::new(
                get_cybernetics_loop(),
                webid,
                ENSEMBLE_GAS_CAP,
            ));
            let manager = SessionManager::new(webid).with_gas_governance(governance);
            Arc::new(RwLock::new(manager))
        })
        .clone()
}

pub fn get_improv_client(
    inference_port: Option<Arc<dyn InferencePort>>,
) -> Arc<CircuitBreakerInferenceAdapter> {
    IMPROV_CLIENT
        .get_or_init(|| {
            let breaker: Arc<dyn CircuitBreakerPort> =
                Arc::new(CircuitBreaker::default_for_inference("ensemble-inference"));

            match inference_port {
                Some(port) => {
                    let adapter = InferencePortAdapter::new(port);
                    Arc::new(CircuitBreakerInferenceAdapter::new(adapter, breaker))
                }
                None => {
                    let base_url = std::env::var("OKAPI_BASE_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
                    let config = OkapiConfig {
                        base_url,
                        ..OkapiConfig::default()
                    };
                    let inference = OkapiInference::new("qwen3:8b", config)
                        .expect("Failed to create Okapi inference");
                    let port: Arc<dyn InferencePort> = Arc::new(inference);
                    let adapter = InferencePortAdapter::new(port);
                    Arc::new(CircuitBreakerInferenceAdapter::new(adapter, breaker))
                }
            }
        })
        .clone()
}

/// Open a StandingSessionStore from environment config, or in-memory as fallback.
fn open_standing_session_store() -> Arc<dyn StandingSessionPort> {
    let conn = match std::env::var("HKASK_API_DB")
        .ok()
        .zip(std::env::var("HKASK_DB_PASSPHRASE").ok())
    {
        Some((path, passphrase)) => hkask_storage::Database::open(&path, &passphrase)
            .expect("Failed to open standing session database")
            .conn_arc(),
        None => hkask_storage::Database::in_memory()
            .expect("in-memory standing session db")
            .conn_arc(),
    };
    let store = hkask_storage::StandingSessionStore::new(conn);
    store
        .initialize_schema()
        .expect("standing session schema init");
    Arc::new(store)
}

// ── Chat Sessions ──────────────────────────────────────────────────────────

/// Create chat session
pub async fn ensemble_chat_create(session: String) -> Result<String, String> {
    let manager = get_session_manager();
    manager.read().await.create_chat(&session).await;
    Ok(format!("Chat session '{}' created", session))
}

/// Register bot in chat
pub async fn ensemble_chat_register(
    session: String,
    bot: String,
    role: String,
) -> Result<String, String> {
    let manager = get_session_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(&session).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session))?;

    let participant_role = match role.as_str() {
        "orchestrator" => ParticipantRole::Curator,
        _ => ParticipantRole::Custom(role.clone()),
    };

    let mut chat_write = chat.write().await;
    chat_write.register_participant(ChatParticipant {
        webid: WebID::new(),
        role: participant_role,
        pod_id: None,
        capabilities: vec![],
    });

    Ok(format!(
        "Bot '{}' registered as {} in session '{}'",
        bot, role, session
    ))
}

/// Send message to chat
pub async fn ensemble_chat_send(session: String, message: String) -> Result<String, String> {
    let manager = get_session_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(&session).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session))?;

    let mut chat_write = chat.write().await;
    let msg = ChatMessage::new(WebID::new(), message);
    chat_write.add_message(msg);

    Ok("Message sent".to_string())
}

/// List chat sessions
pub async fn ensemble_chat_list() -> Result<Vec<String>, String> {
    let manager = get_session_manager();
    let sessions = {
        let manager_read = manager.read().await;
        manager_read.list_chat_sessions().await
    };
    Ok(sessions)
}

// ── Improv ─────────────────────────────────────────────────────────────────

pub async fn ensemble_improv_turn(
    session_id: &str,
    user_message: &str,
    inference_port: Option<Arc<dyn InferencePort>>,
) -> Result<hkask_ensemble::ImprovTurn, String> {
    let manager = get_session_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let client = get_improv_client(inference_port);
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

pub async fn ensemble_improv_config(session_id: &str) -> Result<ImprovSessionConfig, String> {
    let manager = get_session_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let chat_read = chat.read().await;
    Ok(chat_read.improv_config().clone())
}

pub async fn ensemble_improv_set_threshold(session_id: &str, threshold: f64) -> Result<(), String> {
    let manager = get_session_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let mut chat_write = chat.write().await;
    chat_write.set_participation_threshold(threshold);
    Ok(())
}

pub async fn ensemble_improv_set_mode(session_id: &str, mode: ImprovMode) -> Result<(), String> {
    let manager = get_session_manager();
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
    session_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let manager = get_session_manager();
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

// ── Deliberation ───────────────────────────────────────────────────────────

pub async fn ensemble_deliberation_create(session: String) -> Result<String, String> {
    let manager = get_session_manager();
    manager.read().await.create_deliberation(&session).await;
    Ok(format!("Deliberation session '{}' created", session))
}

pub async fn ensemble_deliberation_start(session: String) -> Result<String, String> {
    let manager = get_session_manager();
    let deliberation = {
        let manager_read = manager.read().await;
        manager_read.get_deliberation(&session).await
    }
    .ok_or_else(|| format!("Deliberation session '{}' not found", session))?;

    let mut session_write = deliberation.write().await;
    session_write.start();
    Ok("Deliberation started".to_string())
}

pub async fn ensemble_deliberation_record(
    session: String,
    _agent: String,
    content: String,
    confidence: f64,
) -> Result<String, String> {
    let manager = get_session_manager();
    let deliberation = {
        let manager_read = manager.read().await;
        manager_read.get_deliberation(&session).await
    }
    .ok_or_else(|| format!("Deliberation session '{}' not found", session))?;

    let agent_webid = WebID::new();
    let response = AgentResponse::new(agent_webid, content, confidence);
    let mut session_write = deliberation.write().await;
    session_write.record_response(response);

    Ok("Response recorded".to_string())
}

pub async fn ensemble_deliberation_synthesize(session: String) -> Result<String, String> {
    let manager = get_session_manager();
    let result = {
        let manager_read = manager.read().await;
        let deliberation = manager_read
            .get_deliberation(&session)
            .await
            .ok_or_else(|| format!("Deliberation session '{}' not found", session))?;
        let session_read = deliberation.read().await;
        session_read.synthesize()
    };
    Ok(result.synthesized_response)
}

pub async fn ensemble_deliberation_list() -> Result<Vec<String>, String> {
    let manager = get_session_manager();
    let sessions = {
        let manager_read = manager.read().await;
        manager_read.list_deliberation_sessions().await
    };
    Ok(sessions)
}

// ── Standing Session ───────────────────────────────────────────────────────

/// Bootstrap the standing ensemble session from a YAML manifest.
pub fn ensemble_standing_start(
    config_path: &std::path::Path,
) -> Result<hkask_ensemble::StandingSessionStatus, crate::errors::EnsembleError> {
    let store = open_standing_session_store();
    let session = bootstrap_standing_session_with_store(config_path, store)
        .map_err(|e| crate::errors::EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}

/// Get the current standing session status.
pub fn ensemble_standing_status()
-> Result<hkask_ensemble::StandingSessionStatus, crate::errors::EnsembleError> {
    let config_path = std::path::Path::new("registry/manifests/standing-ensemble-session.yaml");
    if !config_path.exists() {
        return Err(crate::errors::EnsembleError::SessionNotFound(
            "Standing session not bootstrapped. Run 'kask ensemble standing-start' first."
                .to_string(),
        ));
    }

    let store = open_standing_session_store();
    let session = bootstrap_standing_session_with_store(config_path, store)
        .map_err(|e| crate::errors::EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}

/// CLI handler for `kask ensemble` subcommand
pub fn run_ensemble(rt: &tokio::runtime::Runtime, action: crate::cli::EnsembleAction) {
    use crate::commands;

    match action {
        EnsembleAction::ChatCreate { session } => {
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_chat_create(session.clone())),
                    "Chat create failed",
                )
            );
        }
        EnsembleAction::ChatRegister { session, bot, role } => {
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_chat_register(
                        session.clone(),
                        bot.clone(),
                        role.clone(),
                    )),
                    "Chat register failed",
                )
            );
        }
        EnsembleAction::ChatSend { session, message } => {
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_chat_send(
                        session.clone(),
                        message.clone(),
                    )),
                    "Chat send failed",
                )
            );
        }
        EnsembleAction::ChatList => {
            let sessions = super::helpers::or_exit(
                rt.block_on(commands::ensemble_chat_list()),
                "Chat list failed",
            );
            println!("Active chat sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::DeliberationCreate { session } => {
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_create(session.clone())),
                    "Deliberation create failed",
                )
            );
        }
        EnsembleAction::DeliberationStart { session } => {
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_start(session.clone())),
                    "Deliberation start failed",
                )
            );
        }
        EnsembleAction::DeliberationRecord {
            session,
            agent,
            content,
            confidence,
        } => {
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_record(
                        session.clone(),
                        agent.clone(),
                        content.clone(),
                        confidence,
                    )),
                    "Deliberation record failed",
                )
            );
        }
        EnsembleAction::DeliberationSynthesize { session } => {
            println!(
                "Synthesized response:\n{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_synthesize(session.clone())),
                    "Deliberation synthesize failed",
                )
            );
        }
        EnsembleAction::DeliberationList => {
            let sessions = super::helpers::or_exit(
                rt.block_on(commands::ensemble_deliberation_list()),
                "Deliberation list failed",
            );
            println!("Active deliberation sessions:");
            for s in sessions {
                println!("  - {}", s);
            }
        }
        EnsembleAction::StandingStart { config } => {
            let status = super::helpers::or_exit(
                commands::ensemble_standing_start(&config),
                "Standing session bootstrap failed",
            );
            println!("Standing session bootstrapped:");
            println!("  Session ID: {}", status.session_id);
            println!("  Participants: {}", status.participant_count);
            println!("  Initial messages: {}", status.message_count);
        }
        EnsembleAction::StandingStatus => {
            let status = super::helpers::or_exit(
                commands::ensemble_standing_status(),
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
