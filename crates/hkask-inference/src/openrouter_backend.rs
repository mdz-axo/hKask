//! OpenRouter backend — cloud inference via OpenAI-compatible API.
//!
//! OpenRouter exposes `/v1/chat/completions` and `/v1/models` at
//! `https://openrouter.ai/api`. Requires Bearer token
//! authentication via `OPENROUTER_API_KEY`.
//!
//! OpenRouter provides a unified API to hundreds of models from
//! multiple providers through a single endpoint.

use crate::chat_protocol::{
    ChatResponse, build_chat_request, chat_response_to_result, stream_chat_completion,
    validate_prompt,
};
use crate::config::InferenceConfig;
use hkask_types::ports::{InferenceError, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// OpenRouter backend for chat completions and model listing.
pub struct OpenRouterBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

/// A model entry returned by OpenRouter's `/v1/models` endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub object: Option<String>,
    pub created: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelList {
    data: Vec<OpenRouterModel>,
}

impl OpenRouterBackend {
    /// Create a new OpenRouter backend from inference config.
    ///
    /// Returns an error if `openrouter_api_key` is empty.
    ///
    /// expect: "The system creates provider membranes requiring valid API keys"
    /// \[P4\] Motivating: Clear Boundaries — OpenRouter provider membrane requires valid API key
    /// pre:  config.openrouter_api_key is set
    /// post: returns OpenRouterBackend with configured HTTP client
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.openrouter_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "OpenRouter API key not configured (set OPENROUTER_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(InferenceError::Connection)?;
        Ok(Self {
            base_url: config.openrouter_base_url.clone(),
            api_key: config.openrouter_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to OpenRouter.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated text generation
    /// pre:  model is a valid OpenRouter model name
    /// pre:  prompt is non-empty (validated by validate_prompt)
    /// pre:  params is a valid LLMParameters
    /// post: returns Ok(InferenceResult) with generated text, model, usage stats
    /// post: if connection fails → Err(InferenceError::Connection)
    /// post: if prompt is empty → Err(InferenceError::Generation)
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(format!("OpenRouter request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "OpenRouter returned {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("OpenRouter JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "cns.inference",
            provider = "OR",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "OpenRouter inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from OpenRouter via SSE.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated streaming text generation
    /// pre:  model is a valid OpenRouter model name
    /// post: returns stream of inference chunks
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
        let auth = format!("Bearer {}", self.api_key);
        stream_chat_completion(
            Arc::clone(&self.client),
            self.base_url.clone(),
            auth,
            model.to_string(),
            prompt.to_string(),
            params.clone(),
        )
    }

    /// Vision/multimodal inference with base64-encoded images.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated multimodal generation
    /// pre:  model is a valid OpenRouter vision-capable model name
    /// pre:  prompt is non-empty
    /// pre:  images is non-empty (at least one base64-encoded image)
    /// pre:  params is a valid LLMParameters
    /// post: returns Ok(InferenceResult) with vision-generated text
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn generate_vision(
        &self,
        model: &str,
        prompt: &str,
        images: &[String],
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("OpenRouter vision request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "OpenRouter returned {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("OpenRouter vision JSON parse: {}", e)))?;

        chat_response_to_result(chat_response)
    }

    /// List available models from OpenRouter.
    ///
    /// expect: "I can discover available models across providers"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — model variety discovery
    /// pre:  self.client and self.base_url are initialized
    /// post: returns Ok(`Vec<OpenRouterModel>`) with all available models
    /// post: if API returns non-success → Err(InferenceError::Connection)
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn list_models(&self) -> Result<Vec<OpenRouterModel>, InferenceError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("OpenRouter models request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "OpenRouter models error {}: {}",
                status, body
            )));
        }

        let list: OpenRouterModelList = response.json().await.map_err(|e| {
            InferenceError::Connection(format!("OpenRouter models parse error: {}", e))
        })?;

        info!(
            target: "hkask.inference.openrouter",
            count = list.data.len(),
            "Fetched OpenRouter model list"
        );

        Ok(list.data)
    }
}
