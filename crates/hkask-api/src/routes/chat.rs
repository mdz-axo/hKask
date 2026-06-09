//! Curator chat routes
//
//! The `POST /api/chat` endpoint accepts an optional `model` field that
//! switches the LLM used for inference. Use `GET /api/models` to discover
//! valid model identifiers.

use axum::extract::Extension;
use axum::{Json, extract::State, routing::Router};

use crate::ApiState;
use crate::middleware::auth::AuthContext;
use hkask_services::{ChatRequest as ServiceChatRequest, ChatService};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Chat request sent to the Curator or a specified agent.
///
/// The `model` field allows switching the LLM at request time. When omitted,
/// the server default (qwen3:8b) is used. Use `GET /api/models` to discover
/// available models, and `GET /api/models/search?q=...` for fuzzy matching.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatRequest {
    /// User input message
    pub input: String,
    /// Optional template ID to contextualize the prompt
    pub template_id: Option<String>,
    /// Model override for inference (e.g., "qwen3:8b"). If unset, uses the server default.
    #[serde(default)]
    pub model: Option<String>,
}

/// Chat response from the Curator or agent.
///
/// The `model` field echoes which LLM was used, confirming model switching.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatResponse {
    /// Generated response text
    pub output: String,
    /// Template ID that was applied (or "auto-select")
    pub template_id: String,
    /// Model identifier used for inference
    pub model: String,
}

/// Create chat router
pub fn chat_router() -> Router<ApiState> {
    Router::new().route("/api/chat", axum::routing::post(chat))
}

/// Chat with the Curator or a specified agent.
///
/// Accepts an optional `model` field to switch the LLM at request time.
/// When omitted, the server default (`qwen3:8b`) is used. The response
/// echoes the `model` used, confirming which LLM generated the output.
///
/// Use `GET /api/models` or `GET /api/models/search?q=...` to discover
/// available model identifiers.
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "chat",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Chat response", body = ChatResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn chat(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let model_str = req.model.clone().unwrap_or_else(|| "qwen3:8b".to_string());
    let model: &str = &model_str;
    let strategy = hkask_templates::PromptStrategy::from_input(&req.input);

    // Frame the prompt via PromptStrategy (API-specific — uses template_id)
    let prompt = match &req.template_id {
        Some(id) => format!("[template: {}] {}", id, req.input),
        None => strategy.frame(&req.input).to_string(),
    };

    let svc_req = ServiceChatRequest {
        input: prompt,
        agent_name: Some("Curator".to_string()),
        model_override: req.model,
        system_prompt_suffix: None,
        tool_section: None,
        inference_port_override: None,
        episodic_storage_override: None,
        semantic_storage_override: None,
        auth_context: Some(auth),
    };

    let result = match ChatService::chat(&state.service_context, svc_req).await {
        Ok(resp) => resp,
        Err(e) => hkask_services::ChatResponse {
            text: format!("Chat error: {}", e),
            usage: None,
            finish_reason: "error".to_string(),
            tool_calls: vec![],
        },
    };

    Json(ChatResponse {
        output: result.text,
        template_id: req
            .template_id
            .unwrap_or_else(|| strategy.name().to_string()),
        model: model.to_string(),
    })
}
