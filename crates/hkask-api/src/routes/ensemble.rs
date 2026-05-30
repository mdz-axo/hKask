//! Ensemble multi-agent routes (Phase 7)

use axum::extract::Extension;
use axum::{
    Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::Router,
};
use hkask_ensemble::StandingSessionConfig;
use hkask_ensemble::standing_session::StandingSession;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

use crate::middleware::AuthContext;
use crate::{ApiState, ErrorResponse};

/// Create ensemble router
pub fn ensemble_router() -> Router<ApiState> {
    Router::new()
        .route("/api/ensemble/chat", axum::routing::post(create_chat))
        .route("/api/ensemble/chat/:session", axum::routing::get(get_chat))
        .route(
            "/api/ensemble/chat/:session/list",
            axum::routing::get(list_chats),
        )
        .route(
            "/api/ensemble/chat/:session/register",
            axum::routing::post(register_bot),
        )
        .route(
            "/api/ensemble/chat/:session/send",
            axum::routing::post(send_message),
        )
        .route(
            "/api/ensemble/deliberation",
            axum::routing::post(create_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/:session/start",
            axum::routing::post(start_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/:session/record",
            axum::routing::post(record_response),
        )
        .route(
            "/api/ensemble/deliberation/:session/synthesize",
            axum::routing::post(synthesize_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/list",
            axum::routing::get(list_deliberations),
        )
        // Standing session routes (v1)
        .route(
            "/api/v1/ensemble/standing-start",
            axum::routing::post(standing_start),
        )
        .route(
            "/api/v1/ensemble/standing-status",
            axum::routing::get(standing_status),
        )
}

/// Create chat request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChatRequest {
    pub session_id: String,
}

/// Register bot request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterBotRequest {
    pub bot_webid: String,
    pub role: String,
}

/// Send message request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
}

/// Record response request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordResponseRequest {
    pub agent_webid: String,
    pub content: String,
    pub confidence: f64,
}

/// Chat response
#[derive(Debug, Serialize, ToSchema)]
pub struct EnsembleResponse {
    pub success: bool,
    pub message: String,
}

/// Create chat session
#[utoipa::path(
    post,
    path = "/api/ensemble/chat",
    tag = "ensemble",
    request_body = CreateChatRequest,
    responses(
        (status = 201, description = "Chat session created", body = EnsembleResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn create_chat(
    State(_state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Chat session '{}' created", req.session_id),
    };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Get chat details
async fn get_chat(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Chat session '{}' details", session),
    };
    Json(response)
}

/// List chat sessions
async fn list_chats(State(_state): State<ApiState>) -> impl IntoResponse {
    Json(vec![String::from("default_session")])
}

/// Register bot in chat
async fn register_bot(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
    Json(_req): Json<RegisterBotRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Bot registered in session '{}'", session),
    };
    Json(response)
}

/// Send message to chat
async fn send_message(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
    Json(_req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Message sent to session '{}'", session),
    };
    Json(response)
}

/// Create deliberation session
async fn create_deliberation(
    State(_state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation session '{}' created", req.session_id),
    };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Start deliberation
async fn start_deliberation(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation '{}' started", session),
    };
    Json(response)
}

/// Record response in deliberation
async fn record_response(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
    Json(_req): Json<RecordResponseRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Response recorded in deliberation '{}'", session),
    };
    Json(response)
}

/// Synthesize deliberation responses
async fn synthesize_deliberation(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation '{}' synthesized", session),
    };
    Json(response)
}

/// List deliberation sessions
async fn list_deliberations(State(_state): State<ApiState>) -> impl IntoResponse {
    Json(vec![String::from("default_deliberation")])
}

// Standing session request/response types

/// Standing session start request
#[derive(Debug, Deserialize, ToSchema)]
pub struct StandingStartRequest {
    pub session_id: String,
    pub name: String,
    pub description: String,
    pub initial_context: String,
    pub participants: Vec<ParticipantEntryRequest>,
}

/// Participant entry for standing session start
#[derive(Debug, Deserialize, ToSchema)]
pub struct ParticipantEntryRequest {
    pub agent: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub role: String,
    pub description: String,
}

/// Standing session start response
#[derive(Debug, Serialize, ToSchema)]
pub struct StandingStartResponse {
    pub session_id: String,
    pub description: String,
    pub participant_count: usize,
    pub message: String,
}

/// Participant status in standing session response
#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantStatusResponse {
    pub name: String,
    pub webid: String,
    pub role: String,
    pub description: String,
}

/// Standing session status response
#[derive(Debug, Serialize, ToSchema)]
pub struct StandingStatusResponse {
    pub session_id: String,
    pub description: String,
    pub participant_count: usize,
    pub message_count: usize,
    pub participants: Vec<ParticipantStatusResponse>,
}

// Standing session handlers

/// Start a standing ensemble session with initial context
#[utoipa::path(
    post,
    path = "/api/v1/ensemble/standing-start",
    tag = "ensemble",
    request_body = StandingStartRequest,
    responses(
        (status = 201, description = "Standing session started", body = StandingStartResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn standing_start(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<StandingStartRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.ensemble.standing_start.start"),
        Phase::Observe,
        serde_json::json!({
            "session_id": req.session_id,
            "participants": req.participants.len(),
        }),
    );

    // Build a StandingSessionConfig from the request
    let config = StandingSessionConfig {
        session: hkask_ensemble::standing_session::SessionMetadata {
            id: req.session_id.clone(),
            name: req.name,
            description: req.description.clone(),
        },
        participants: req
            .participants
            .into_iter()
            .map(|p| hkask_ensemble::standing_session::ParticipantEntry {
                agent: p.agent,
                agent_type: p.agent_type,
                role: p.role,
                description: p.description,
                domains: vec![],
            })
            .collect(),
        bootstrap: hkask_ensemble::standing_session::BootstrapConfig {
            initial_message: hkask_ensemble::standing_session::InitialMessage {
                from: "Curator".to_string(),
                message_type: "system".to_string(),
                content: req.initial_context,
            },
            initial_reports: vec![],
        },
    };

    let mut session = StandingSession::from_config(config.clone());
    session.post_initial_messages(&config);

    let status = session.get_status();
    let participant_count = status.participant_count;

    // Store the session in application state for later status queries
    {
        let mut sessions = state.standing_sessions.write().await;
        sessions.insert(req.session_id.clone(), Arc::new(RwLock::new(session)));
    }

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.ensemble.standing_start.success"),
        Phase::Observe,
        serde_json::json!({
            "session_id": req.session_id,
            "participant_count": participant_count,
        }),
    );

    Ok((
        StatusCode::CREATED,
        Json(StandingStartResponse {
            session_id: req.session_id,
            description: status.description,
            participant_count,
            message: "Standing session started".to_string(),
        }),
    ))
}

/// Get standing session status
#[utoipa::path(
    get,
    path = "/api/v1/ensemble/standing-status",
    tag = "ensemble",
    responses(
        (status = 200, description = "Standing session status", body = StandingStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No standing session found"),
    ),
)]
async fn standing_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<StandingStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.ensemble.standing_status.start"),
        Phase::Observe,
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    // Return the first available standing session, or 404 if none exist
    let sessions = state.standing_sessions.read().await;
    let session_guard = sessions.values().next();

    match session_guard {
        Some(session_lock) => {
            let session = session_lock.read().await;
            let status = session.get_status();

            let participants: Vec<ParticipantStatusResponse> = status
                .participants
                .into_iter()
                .map(|p| ParticipantStatusResponse {
                    name: p.name,
                    webid: p.webid,
                    role: p.role,
                    description: p.description,
                })
                .collect();

            let response = StandingStatusResponse {
                session_id: status.session_id,
                description: status.description,
                participant_count: status.participant_count,
                message_count: status.message_count,
                participants,
            };

            state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.ensemble.standing_status.success"),
        Phase::Observe,
                serde_json::json!({
                    "session_id": response.session_id,
                }),
            );

            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "no_standing_session".to_string(),
                code: "ENSEMBLE_NOT_FOUND".to_string(),
                details: Some(serde_json::json!({
                    "message": "No standing session exists. Start one with /api/v1/ensemble/standing-start"
                })),
            }),
        )),
    }
}
