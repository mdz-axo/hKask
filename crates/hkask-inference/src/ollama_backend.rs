//! Ollama backend — local inference via Ollama's OpenAI-compatible API.
//!
//! Ollama exposes `/v1/chat/completions` (OpenAI-compatible) and
//! `/api/tags` (native model listing). No authentication required
//! for local instances.

use crate::chat_protocol::{
    build_chat_request, chat_response_to_result, parse_sse_stream, validate_prompt,
};
use crate::config::InferenceConfig;
use futures_util::StreamExt;
use hkask_types::LLMParameters;
use hkask_types::ports::{InferenceError, InferenceResult, InferenceStreamChunk};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Ollama backend for chat completions and model listing.
pub struct OllamaBackend {
    base_url: String,
    client: Arc<reqwest::Client>,
}

impl OllamaBackend {
    /// Create a new Ollama backend from the inference config.
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| InferenceError::Connection(e))?;
        Ok(Self {
            base_url: config.ollama_base_url.clone(),
            client,
        })
    }

    /// Send a chat completion request to Ollama.
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        let request = build_chat_request(model, prompt, None, params, Some(false), Some(5));

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "Ollama status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Ollama JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "OM",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "Ollama inference completed"
        );
        Ok(result)
    }

    /// Vision/multimodal inference with base64-encoded images.
    pub async fn generate_vision(
        &self,
        model: &str,
        prompt: &str,
        images: &[String],
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        if images.is_empty() {
            return Err(InferenceError::Generation("No images provided".into()));
        }
        let request = build_chat_request(
            model,
            prompt,
            Some(images.to_vec()),
            params,
            Some(false),
            Some(5),
        );

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "Ollama vision status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Ollama JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "OM",
            model = %result.model,
            tokens = result.usage.total_tokens,
            "Ollama vision inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from Ollama via SSE.
    pub fn generate_stream(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        let model = model.to_string();
        let prompt = prompt.to_string();
        let params = params.clone();
        let client = Arc::clone(&self.client);
        let base_url = self.base_url.clone();

        Box::pin(
            futures_util::stream::once(async move {
                let request = build_chat_request(&model, &prompt, None, &params, Some(true), None);

                let response = match client
                    .post(format!("{}/v1/chat/completions", base_url))
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| InferenceError::Connection(e.to_string()))
                {
                    Ok(r) => r,
                    Err(e) => return vec![Err(e)],
                };

                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return vec![Err(InferenceError::Connection(format!(
                        "Ollama streaming status {}: {}",
                        status, error_text
                    )))];
                }

                let body = match response
                    .text()
                    .await
                    .map_err(|e| InferenceError::Connection(e.to_string()))
                {
                    Ok(b) => b,
                    Err(e) => return vec![Err(e)],
                };

                parse_sse_stream(&body, &model)
            })
            .map(futures_util::stream::iter)
            .flatten(),
        )
    }

    /// List models available in the local Ollama instance via `/api/tags`.
    pub async fn list_models(&self) -> Result<Vec<OllamaModelEntry>, InferenceError> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(Vec::new()); // Graceful degradation
        }

        let tags: OllamaTagsResponse = response
            .json()
            .await
            .map_err(|_| InferenceError::Json("Ollama tags parse error".into()))?;

        Ok(tags.models)
    }
}

// ── Ollama model types ───────────────────────────────────────────────────────

/// A model entry from Ollama's `/api/tags` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelEntry {
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub details: Option<OllamaModelDetails>,
}

/// Model details from Ollama's `/api/tags` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub parameter_size: Option<String>,
    #[serde(default)]
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaTagsResponse {
    pub models: Vec<OllamaModelEntry>,
}
