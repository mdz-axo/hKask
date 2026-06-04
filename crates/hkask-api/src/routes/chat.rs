//! Curator chat routes
//
//! The `POST /api/chat` endpoint accepts an optional `model` field that
//! switches the LLM used for inference. Use `GET /api/models` to discover
//! valid model identifiers.

use std::sync::Arc;

use axum::{Json, extract::State, routing::Router};

use crate::{ApiState, ChatRequest, ChatResponse};

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
    use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
    use hkask_types::LLMParameters;

    let model = req.model.as_deref().unwrap_or("qwen3:8b");

    // Use the shared inference port when available (avoids per-request OkapiInference construction)
    let inference: Arc<dyn InferencePort> = match state.inference_port {
        Some(ref port) => Arc::clone(port),
        None => {
            let config = OkapiConfig::local_dev();
            match OkapiInference::new(model, config) {
                Ok(i) => Arc::new(i) as Arc<dyn InferencePort>,
                Err(e) => {
                    return Json(ChatResponse {
                        output: format!("Failed to initialize Okapi: {}", e),
                        template_id: req.template_id.unwrap_or("error".to_string()),
                        model: model.to_string(),
                    });
                }
            }
        }
    };

    let prompt = match &req.template_id {
        Some(id) => format!("[template: {}] {}", id, req.input),
        None => {
            if req.input.contains('?') || req.input.contains("what") || req.input.contains("how") {
                format!("Answer concisely: {}", req.input)
            } else if req.input.contains("create")
                || req.input.contains("make")
                || req.input.contains("build")
            {
                format!("Provide step-by-step instructions: {}", req.input)
            } else {
                format!("Respond helpfully: {}", req.input)
            }
        }
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
        template_id: req.template_id.unwrap_or("auto-select".to_string()),
        model: model.to_string(),
    })
}
