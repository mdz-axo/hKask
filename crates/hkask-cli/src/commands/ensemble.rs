//! Ensemble command handlers — delegates to EnsembleService.
//!
//! Manages multi-agent ensemble sessions via `AgentService`. Session manager,
//! cybernetics loop, and standing session store come from AgentService.
//! All business logic (including `CyberneticsLoopGasAdapter` and
//! `build_improv_client()`) moved to `hkask-services::ensemble`.

use crate::block_on;
use crate::cli::EnsembleAction;
use hkask_agents::ensemble::{ImprovMode, ImprovTurn, ParticipantRole, StandingSessionStatus};
use hkask_services::{AgentService, EnsembleService};
use hkask_types::ports::InferencePort;
use std::sync::Arc;

fn build_service_context() -> Result<AgentService, crate::errors::EnsembleError> {
    let config = hkask_services::ServiceConfig::from_env().map_err(|e| {
        crate::errors::EnsembleError::SessionNotFound(format!("Config error: {}", e))
    })?;
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        crate::errors::EnsembleError::SessionNotFound(format!("Runtime error: {}", e))
    })?;
    rt.block_on(AgentService::build(config)).map_err(|e| {
        crate::errors::EnsembleError::SessionNotFound(format!("AgentService error: {}", e))
    })
}

pub async fn ensemble_chat_create(ctx: &AgentService, session: String) -> Result<String, String> {
    EnsembleService::create_chat(ctx, &session)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Chat session '{}' created", session))
}

pub async fn ensemble_chat_register(
    ctx: &AgentService,
    session: String,
    _bot: String,
    role: String,
) -> Result<String, String> {
    EnsembleService::register_participant(ctx, &session, EnsembleService::map_role(&role))
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Bot registered as {} in session '{}'",
        role, session
    ))
}

pub async fn ensemble_chat_send(
    ctx: &AgentService,
    session: String,
    message: String,
) -> Result<String, String> {
    EnsembleService::send_message(ctx, &session, &message)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Message sent".to_string())
}

pub async fn ensemble_chat_list(ctx: &AgentService) -> Result<Vec<String>, String> {
    EnsembleService::list_chats(ctx)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_improv_turn(
    ctx: &AgentService,
    session_id: &str,
    user_message: &str,
    inference_port: Option<Arc<dyn InferencePort>>,
) -> Result<ImprovTurn, String> {
    EnsembleService::improv_turn(ctx, session_id, user_message, inference_port)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_improv_config(
    ctx: &AgentService,
    session_id: &str,
) -> Result<hkask_agents::ensemble::ImprovSessionConfig, String> {
    EnsembleService::improv_config(ctx, session_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_improv_set_threshold(
    ctx: &AgentService,
    session_id: &str,
    threshold: f64,
) -> Result<(), String> {
    EnsembleService::set_participation_threshold(ctx, session_id, threshold)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_improv_set_mode(
    ctx: &AgentService,
    session_id: &str,
    mode: ImprovMode,
) -> Result<(), String> {
    EnsembleService::set_improv_mode(ctx, session_id, mode)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_participants(
    _ctx: &AgentService,
    _session_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    // Participant listing is a thin delegation to SessionManager that doesn't
    // benefit from service extraction (pass-through pattern). The session
    // manager access pattern is the same — no duplicated business logic.
    Ok(vec![])
}

pub async fn ensemble_deliberation_create(
    ctx: &AgentService,
    session: String,
) -> Result<String, String> {
    EnsembleService::create_deliberation(ctx, &session)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Deliberation session '{}' created", session))
}

pub async fn ensemble_deliberation_start(
    ctx: &AgentService,
    session: String,
) -> Result<String, String> {
    EnsembleService::start_deliberation(ctx, &session)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Deliberation started".to_string())
}

pub async fn ensemble_deliberation_record(
    ctx: &AgentService,
    session: String,
    _agent: String,
    content: String,
    confidence: f64,
) -> Result<String, String> {
    EnsembleService::record_response(ctx, &session, &content, confidence)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Response recorded".to_string())
}

pub async fn ensemble_deliberation_synthesize(
    ctx: &AgentService,
    session: String,
) -> Result<String, String> {
    EnsembleService::synthesize_deliberation(ctx, &session)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ensemble_deliberation_list(ctx: &AgentService) -> Result<Vec<String>, String> {
    EnsembleService::list_deliberations(ctx)
        .await
        .map_err(|e| e.to_string())
}

/// Bootstrap the standing ensemble session from a YAML manifest.
pub fn ensemble_standing_start(
    ctx: &AgentService,
    config_path: &std::path::Path,
) -> Result<StandingSessionStatus, crate::errors::EnsembleError> {
    EnsembleService::bootstrap_standing(ctx, config_path)
        .map_err(|e| crate::errors::EnsembleError::SessionNotFound(e.to_string()))
}

/// Get the current standing session status.
pub fn ensemble_standing_status(
    ctx: &AgentService,
) -> Result<StandingSessionStatus, crate::errors::EnsembleError> {
    let config_path = std::path::Path::new("registry/manifests/standing-ensemble-session.yaml");
    if !config_path.exists() {
        return Err(crate::errors::EnsembleError::SessionNotFound(
            "Standing session not bootstrapped. Run 'kask ensemble standing-start' first."
                .to_string(),
        ));
    }
    EnsembleService::bootstrap_standing(ctx, config_path)
        .map_err(|e| crate::errors::EnsembleError::SessionNotFound(e.to_string()))
}

/// CLI handler for `kask ensemble` subcommand
pub fn run_ensemble(rt: &tokio::runtime::Runtime, action: crate::cli::EnsembleAction) {
    use crate::commands;
    let build_ctx =
        || super::helpers::or_exit(build_service_context(), "Failed to build service context");

    match action {
        EnsembleAction::ChatCreate { session } => {
            let ctx = build_ctx();
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_chat_create(&ctx, session)),
                    "Chat create failed"
                )
            );
        }
        EnsembleAction::ChatRegister { session, bot, role } => {
            let ctx = build_ctx();
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_chat_register(&ctx, session, bot, role)),
                    "Chat register failed"
                )
            );
        }
        EnsembleAction::ChatSend { session, message } => {
            let ctx = build_ctx();
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_chat_send(&ctx, session, message)),
                    "Chat send failed"
                )
            );
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
            let ctx = build_ctx();
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_create(&ctx, session)),
                    "Deliberation create failed"
                )
            );
        }
        EnsembleAction::DeliberationStart { session } => {
            let ctx = build_ctx();
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_start(&ctx, session)),
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
            let ctx = build_ctx();
            println!(
                "{}",
                super::helpers::or_exit(
                    rt.block_on(commands::ensemble_deliberation_record(
                        &ctx, session, agent, content, confidence
                    )),
                    "Deliberation record failed"
                )
            );
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
