//! Ensemble multi-agent routes (Phase 7)

use axum::extract::Extension;
use axum::{
    Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::Router,
};
use hkask_agents::ensemble::StandingSessionConfig;
use hkask_agents::ensemble::standing_session::StandingSession;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

/// Create ensemble router
pub fn ensemble_router() -> Router<ApiState> {
    use axum::routing::{get, post};
    Router::new()
        .route("/api/ensemble/chat", post(create_chat))
        .route("/api/ensemble/chat/:session", get(get_chat))
        .route("/api/ensemble/chat/:session/list", get(list_chats))
        .route("/api/ensemble/chat/:session/register", post(register_bot))
        .route("/api/ensemble/chat/:session/send", post(send_message))
        .route("/api/ensemble/chat/:session/improv", post(improv_turn))
        .route("/api/ensemble/deliberation", post(create_deliberation))
        .route(
            "/api/ensemble/deliberation/:session/start",
            post(start_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/:session/record",
            post(record_response),
        )
        .route(
            "/api/ensemble/deliberation/:session/synthesize",
            post(synthesize_deliberation),
        )
        .route("/api/ensemble/deliberation/list", get(list_deliberations))
        .route("/api/v1/ensemble/standing-start", post(standing_start))
        .route("/api/v1/ensemble/standing-status", get(standing_status))
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

/// Respond with EnsembleResponse as (StatusCode, Json)
fn respond(code: StatusCode, success: bool, msg: impl Into<String>) -> impl IntoResponse {
    (
        code,
        Json(EnsembleResponse {
            success,
            message: msg.into(),
        }),
    )
        .into_response()
}

/// Improv turn request
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
    state
        .service_context
        .session_manager
        .read()
        .await
        .create_chat(&req.session_id)
        .await;
    respond(
        StatusCode::CREATED,
        true,
        format!("Chat session '{}' created", req.session_id),
    )
}

/// Get chat details
async fn get_chat(State(state): State<ApiState>, Path(session): Path<String>) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    let (exists, msg) = if manager.get_chat(&session).await.is_some() {
        (true, format!("Chat session '{}' details", session))
    } else {
        (false, format!("Chat session '{}' not found", session))
    };
    respond(
        if exists {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        },
        exists,
        msg,
    )
}

/// List chat sessions
async fn list_chats(State(state): State<ApiState>) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    Json(manager.list_chat_sessions().await).into_response()
}

/// Register bot in chat
async fn register_bot(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<RegisterBotRequest>,
) -> impl IntoResponse {
    let role = match req.role.as_str() {
        "orchestrator" => hkask_agents::ensemble::ParticipantRole::Curator,
        other => hkask_agents::ensemble::ParticipantRole::Custom(other.to_string()),
    };
    let manager = state.service_context.session_manager.read().await;
    let (code, body) = match manager.get_chat(&session).await {
        Some(chat) => {
            let mut w = chat.write().await;
            w.register_participant(hkask_agents::ensemble::ChatParticipant {
                webid: hkask_types::WebID::new(),
                role,
                pod_id: None,
                capabilities: vec![],
            });
            (
                StatusCode::OK,
                EnsembleResponse {
                    success: true,
                    message: format!("Bot registered in session '{}'", session),
                },
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            EnsembleResponse {
                success: false,
                message: format!("Session '{}' not found", session),
            },
        ),
    };
    (code, Json(body)).into_response()
}

/// Send message to chat
async fn send_message(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    let (code, body) = match manager.get_chat(&session).await {
        Some(chat) => {
            let mut w = chat.write().await;
            w.add_message(hkask_agents::ensemble::ChatMessage::new(
                hkask_types::WebID::new(),
                req.content.clone(),
            ));
            (
                StatusCode::OK,
                EnsembleResponse {
                    success: true,
                    message: format!("Message sent to session '{}'", session),
                },
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            EnsembleResponse {
                success: false,
                message: format!("Session '{}' not found", session),
            },
        ),
    };
    (code, Json(body)).into_response()
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
    match state.ensemble_inferencer_with_breaker() {
        Some(inferencer) => {
            let manager = state.service_context.session_manager.read().await;
            match manager.get_chat(&session).await {
                Some(chat) => {
                    let turn = {
                        let chat_read = chat.read().await;
                        match chat_read.improv_turn(&inferencer, &req.user_message).await {
                            Ok(t) => t,
                            Err(e) => {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(EnsembleResponse {
                                        success: false,
                                        message: e.to_string(),
                                    }),
                                )
                                    .into_response();
                            }
                        }
                    };
                    {
                        let mut chat_write = chat.write().await;
                        let curator_webid = *chat_write.curator();
                        chat_write.add_message(hkask_agents::ensemble::ChatMessage::new(
                            curator_webid,
                            req.user_message.clone(),
                        ));
                        for r in &turn.responses {
                            chat_write.add_message(hkask_agents::ensemble::ChatMessage::new(
                                r.agent_webid,
                                r.content.clone(),
                            ));
                        }
                    }
                    let response = ImprovTurnResponse {
                        user_message: turn.user_message,
                        judgment_count: turn.judgments.len(),
                        response_count: turn.responses.len(),
                        curator_synthesis: turn.curator_synthesis,
                    };
                    Json(response).into_response()
                }
                None => {
                    let response = EnsembleResponse {
                        success: false,
                        message: format!("Session '{session}' not found"),
                    };
                    (StatusCode::NOT_FOUND, Json(response)).into_response()
                }
            }
        }
        None => {
            let response = EnsembleResponse {
                success: false,
                message: "No inference client configured".to_string(),
            };
            (StatusCode::SERVICE_UNAVAILABLE, Json(response)).into_response()
        }
    }
}

/// Create deliberation session
async fn create_deliberation(
    State(state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    manager.create_deliberation(&req.session_id).await;
    (
        StatusCode::CREATED,
        Json(EnsembleResponse {
            success: true,
            message: format!("Deliberation session '{}' created", req.session_id),
        }),
    )
        .into_response()
}

/// Start deliberation
async fn start_deliberation(
    State(state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    let (code, body) = match manager.get_deliberation(&session).await {
        Some(d) => {
            d.write().await.start();
            (
                StatusCode::OK,
                EnsembleResponse {
                    success: true,
                    message: format!("Deliberation '{}' started", session),
                },
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            EnsembleResponse {
                success: false,
                message: format!("Session '{}' not found", session),
            },
        ),
    };
    (code, Json(body)).into_response()
}

/// Record response in deliberation
async fn record_response(
    State(state): State<ApiState>,
    Path(session): Path<String>,
    Json(req): Json<RecordResponseRequest>,
) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    let (code, body) = match manager.get_deliberation(&session).await {
        Some(d) => {
            let r = hkask_agents::ensemble::AgentResponse::new(
                hkask_types::WebID::new(),
                req.content.clone(),
                req.confidence,
            );
            d.write().await.record_response(r);
            (
                StatusCode::OK,
                EnsembleResponse {
                    success: true,
                    message: format!("Response recorded in deliberation '{}'", session),
                },
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            EnsembleResponse {
                success: false,
                message: format!("Session '{}' not found", session),
            },
        ),
    };
    (code, Json(body)).into_response()
}

/// Synthesize deliberation responses
async fn synthesize_deliberation(
    State(state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    let value = match manager.get_deliberation(&session).await {
        Some(d) => d.read().await.synthesize().synthesized_response,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::Value::String(format!(
                    "Session '{}' not found",
                    session
                ))),
            )
                .into_response();
        }
    };
    (StatusCode::OK, Json(serde_json::Value::String(value))).into_response()
}

/// List deliberation sessions
async fn list_deliberations(State(state): State<ApiState>) -> impl IntoResponse {
    let manager = state.service_context.session_manager.read().await;
    Json(manager.list_deliberation_sessions().await)
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
    /// Template domains this participant owns (e.g., `["cns", "storage"]`).
    /// Converted to capability specs (`"tool:<domain>:execute"`) internally.
    #[serde(default)]
    pub domains: Vec<String>,
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
        session: hkask_agents::ensemble::standing_session::SessionMetadata {
            id: req.session_id.clone(),
            name: req.name,
            description: req.description.clone(),
        },
        participants: req
            .participants
            .into_iter()
            .map(
                |p| hkask_agents::ensemble::standing_session::ParticipantEntry {
                    agent: p.agent,
                    agent_type: p.agent_type,
                    role: p.role,
                    description: p.description,
                    domains: p.domains,
                },
            )
            .collect(),
        bootstrap: hkask_agents::ensemble::standing_session::BootstrapConfig {
            initial_message: hkask_agents::ensemble::standing_session::InitialMessage {
                from: "Curator".to_string(),
                message_type: "system".to_string(),
                content: req.initial_context,
            },
            initial_reports: vec![],
        },
        gas: None,
    };

    let mut session = StandingSession::from_config(config.clone());

    // Discover available MCP tools and wire intersection-based tool scoping.
    // This enables `intersection_tools()` to filter the tool section
    // to only tools visible across all participants.
    {
        let tool_names = state.service_context.mcp_runtime.discover_tools().await;
        let mut tools: Vec<hkask_types::ports::ToolInfo> = Vec::new();
        for name in &tool_names {
            if let Some(info) = state.service_context.mcp_runtime.get_tool_info(name).await {
                tools.push(info);
            }
        }
        if !tools.is_empty() {
            session = session.with_available_tools(tools);
        }
    }

    // Wire storage — persist config and enable message archival
    session = session.with_store(state.service_context.standing_session_store.clone());
    // Wire gas governance — CNS observability for standing session gas usage
    session = session.with_gas_governance(state.gas_governance.clone());
    let config_yaml = serde_yaml::to_string(&config).unwrap_or_default();
    session.persist_session(&config_yaml).ok();

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
