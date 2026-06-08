//! Curator chat routes
//
//! The `POST /api/chat` endpoint accepts an optional `model` field that
//! switches the LLM used for inference. Use `GET /api/models` to discover
//! valid model identifiers.

use std::sync::Arc;

use axum::{Json, extract::State, routing::Router};

use crate::ApiState;
use hkask_types::ports::InferencePort;
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
async fn chat(State(state): State<ApiState>, Json(req): Json<ChatRequest>) -> Json<ChatResponse> {
    use hkask_services::{InferenceContext, InferenceService};
    use hkask_types::LLMParameters;

    let model = req.model.as_deref().unwrap_or("qwen3:8b");
    let strategy = hkask_templates::PromptStrategy::from_input(&req.input);

    // Use the shared inference port when available (avoids per-request OkapiInference construction)
    let inference: Arc<dyn InferencePort> = match state.service_context.inference_port {
        Some(ref port) => Arc::clone(port),
        None => {
            let ctx = InferenceContext::from_parts(
                None,
                model,
                state.service_context.config.okapi_base_url.clone(),
            );
            match InferenceService::resolve_port(&ctx, model) {
                Ok(i) => i,
                Err(e) => {
                    return Json(ChatResponse {
                        output: format!("Failed to initialize inference: {}", e),
                        template_id: req
                            .template_id
                            .unwrap_or_else(|| strategy.name().to_string()),
                        model: model.to_string(),
                    });
                }
            }
        }
    };

    let prompt = match &req.template_id {
        Some(id) => format!("[template: {}] {}", id, req.input),
        None => strategy.frame(&req.input),
    };

    let params = LLMParameters {
        temperature: 0.7,
        top_p: 0.9,
        top_k: 40,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 512,
        seed: None,
    };

    let output = match inference
        .generate_with_model(&prompt, &params, Some(model))
        .await
    {
        Ok(result) => result.text,
        Err(e) => format!("Inference error: {}", e),
    };

    Json(ChatResponse {
        output,
        template_id: req
            .template_id
            .unwrap_or_else(|| strategy.name().to_string()),
        model: model.to_string(),
    })
}
