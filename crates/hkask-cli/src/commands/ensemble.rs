//! Ensemble command handlers — chat, deliberation, improv, and standing sessions
//!
//! Manages multi-agent ensemble sessions via singleton patterns for session
//! manager and improv client. Also handles standing session bootstrap via
//! hkask-ensemble registry manifests.

use hkask_ensemble::{
    AgentResponse, ChatMessage, ChatParticipant, ImprovMode, ImprovSessionConfig, OkapiClient,
    ParticipantRole, SessionManager,
};
use hkask_types::WebID;
use std::sync::Arc;
use tokio::sync::RwLock;

static SESSION_MANAGER: std::sync::OnceLock<Arc<RwLock<SessionManager>>> =
    std::sync::OnceLock::new();
static IMPROV_CLIENT: std::sync::OnceLock<Arc<OkapiClient>> = std::sync::OnceLock::new();

fn get_session_manager() -> Arc<RwLock<SessionManager>> {
    SESSION_MANAGER
        .get_or_init(|| Arc::new(RwLock::new(SessionManager::new(WebID::new()))))
        .clone()
}

fn get_improv_client() -> Arc<OkapiClient> {
    IMPROV_CLIENT
        .get_or_init(|| Arc::new(OkapiClient::from_env()))
        .clone()
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
) -> Result<hkask_ensemble::ImprovTurn, String> {
    let manager = get_session_manager();
    let chat = {
        let manager_read = manager.read().await;
        manager_read.get_chat(session_id).await
    }
    .ok_or_else(|| format!("Chat session '{}' not found", session_id))?;

    let client = get_improv_client();
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
    let session = hkask_ensemble::bootstrap_standing_session(config_path)
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

    let session = hkask_ensemble::bootstrap_standing_session(config_path)
        .map_err(|e| crate::errors::EnsembleError::SessionCreationFailed(e.to_string()))?;
    Ok(session.get_status())
}
