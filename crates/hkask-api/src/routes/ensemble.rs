//! Ensemble multi-agent routes (Phase 7)

use axum::extract::Extension;
use axum::{
    Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::Router,
};
use hkask_ensemble::StandingSessionConfig;
use hkask_ensemble::standing_session::StandingSession;
use hkask_ensemble::{AgentResponse, ChatMessage, ChatParticipant, ParticipantRole};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

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
            "/api/ensemble/chat/:session/improv",
            axum::routing::post(improv_turn),
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

/// Improv turn request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImprovTurnRequest {
    pub user_message: String,
}

/// Improv turn response
#[derive(Debug, Serialize, ToSchema)]
pub struct ImprovTurnResponse {
    pub user_message: String,
    pub judgment_count: usize,
    pub response_count: usize,
    pub curator_synthesis: Option<String>,
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
    State(state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    manager.read().await.create_chat(&req.session_id).await;
    let response = EnsembleResponse {
        success: true,
        message: format!("Chat session '{}' created", req.session_id),
    };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Get chat details
async fn get_chat(State(state): State<ApiState>, Path(session): Path<String>) -> impl IntoResponse {
    let manager = &state.session_manager;
    let chat = manager.read().await.get_chat(&session).await;
    match chat {
        Some(_) => {
            let response = EnsembleResponse {
                success: true,
                message: format!("Chat session '{}' details", session),
            };
            Json(response).into_response()
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: format!("Chat session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// List chat sessions
async fn list_chats(State(state): State<ApiState>) -> impl IntoResponse {
    let manager = &state.session_manager;
    let sessions = manager.read().await.list_chat_sessions().await;
    Json(sessions)
}

/// Register bot in chat
async fn register_bot(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<RegisterBotRequest>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    let chat = manager.read().await.get_chat(&session).await;
    match chat {
        Some(chat) => {
            let participant_role = match req.role.as_str() {
                "orchestrator" => ParticipantRole::Curator,
                _ => ParticipantRole::Custom(req.role.clone()),
            };
            let mut chat_write = chat.write().await;
            chat_write.register_participant(ChatParticipant {
                webid: hkask_types::WebID::new(),
                role: participant_role,
                pod_id: None,
                capabilities: vec![],
            });
            let response = EnsembleResponse {
                success: true,
                message: format!("Bot registered in session '{}'", session),
            };
            Json(response).into_response()
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: format!("Chat session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// Send message to chat
async fn send_message(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    let chat = manager.read().await.get_chat(&session).await;
    match chat {
        Some(chat) => {
            let mut chat_write = chat.write().await;
            let msg = ChatMessage::new(hkask_types::WebID::new(), req.content);
            chat_write.add_message(msg);
            let response = EnsembleResponse {
                success: true,
                message: format!("Message sent to session '{}'", session),
            };
            Json(response).into_response()
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: format!("Chat session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// Execute an improvisation turn in a chat session
#[utoipa::path(
    post,
    path = "/api/ensemble/chat/:session/improv",
    tag = "ensemble",
    request_body = ImprovTurnRequest,
    responses(
        (status = 200, description = "Improv turn completed", body = ImprovTurnResponse),
        (status = 404, description = "Chat session not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn improv_turn(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<ImprovTurnRequest>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    let chat = manager.read().await.get_chat(&session).await;
    match chat {
        Some(chat) => {
            if let Some(inferencer) = state.ensemble_inferencer_with_breaker() {
                let chat_read = chat.read().await;
                match chat_read.improv_turn(&inferencer, &req.user_message).await {
                    Ok(turn) => {
                        let response = ImprovTurnResponse {
                            user_message: turn.user_message,
                            judgment_count: turn.judgments.len(),
                            response_count: turn.responses.len(),
                            curator_synthesis: turn.curator_synthesis,
                        };
                        Json(response).into_response()
                    }
                    Err(e) => {
                        let response = EnsembleResponse {
                            success: false,
                            message: format!("Improv error: {}", e),
                        };
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                    }
                }
            } else {
                let response = EnsembleResponse {
                    success: false,
                    message: "No inference client configured".to_string(),
                };
                (StatusCode::SERVICE_UNAVAILABLE, Json(response)).into_response()
            }
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: format!("Chat session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// Create deliberation session
async fn create_deliberation(
    State(state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    manager
        .read()
        .await
        .create_deliberation(&req.session_id)
        .await;
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation session '{}' created", req.session_id),
    };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Start deliberation
async fn start_deliberation(
    State(state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    let deliberation = manager.read().await.get_deliberation(&session).await;
    match deliberation {
        Some(delib) => {
            let mut session_write = delib.write().await;
            session_write.start();
            let response = EnsembleResponse {
                success: true,
                message: format!("Deliberation '{}' started", session),
            };
            Json(response).into_response()
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: format!("Deliberation session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// Record response in deliberation
async fn record_response(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<RecordResponseRequest>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    let deliberation = manager.read().await.get_deliberation(&session).await;
    match deliberation {
        Some(delib) => {
            let agent_webid = hkask_types::WebID::new();
            let response = AgentResponse::new(agent_webid, req.content, req.confidence);
            let mut session_write = delib.write().await;
            session_write.record_response(response);
            let resp = EnsembleResponse {
                success: true,
                message: format!("Response recorded in deliberation '{}'", session),
            };
            Json(resp).into_response()
        }
        None => {
            let resp = EnsembleResponse {
                success: false,
                message: format!("Deliberation session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(resp)).into_response()
        }
    }
}

/// Synthesize deliberation responses
async fn synthesize_deliberation(
    State(state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let manager = &state.session_manager;
    let deliberation = manager.read().await.get_deliberation(&session).await;
    match deliberation {
        Some(delib) => {
            let session_read = delib.read().await;
            let result = session_read.synthesize();
            let response = EnsembleResponse {
                success: true,
                message: result.synthesized_response,
            };
            Json(response).into_response()
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: format!("Deliberation session '{}' not found", session),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// List deliberation sessions
async fn list_deliberations(State(state): State<ApiState>) -> impl IntoResponse {
    let manager = &state.session_manager;
    let sessions = manager.read().await.list_deliberation_sessions().await;
    Json(sessions)
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
) -> Result<(StatusCode, Json<StandingStartResponse>), ApiError> {
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
        gas: None,
    };

    let mut session = StandingSession::from_config(config.clone());

    // Wire storage if available — persist config and enable message archival
    if let Some(ref store) = state.standing_session_store {
        session = session.with_store(store.clone());
        let config_yaml = serde_yaml::to_string(&config).unwrap_or_default();
        session.persist_session(&config_yaml).ok();
    }

    session.post_initial_messages(&config);

    let status = session.get_status();
    let participant_count = status.participant_count;

    // Store the session in application state for later status queries
    {
        let mut sessions = state.standing_sessions.write().await;
        sessions.insert(req.session_id.clone(), Arc::new(RwLock::new(session)));
    }

    Ok((
        StatusCode::CREATED,
        Json(StandingStartResponse {
            session_id: req.session_id.clone(),
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
) -> Result<Json<StandingStatusResponse>, ApiError> {
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

            Ok(Json(response))
        }
        None => Err(ApiError::NotFound {
            resource: "standing_session".into(),
            id: "none".into(),
        }),
    }
}
