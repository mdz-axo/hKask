//! Ensemble multi-agent routes (Phase 7)

use axum::{
    Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiState;

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
    // For now, just return success - actual implementation would use EnsembleChatManager
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
