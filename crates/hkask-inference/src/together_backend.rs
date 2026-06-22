//! Together AI backend — cloud inference via OpenAI-compatible API.
//!
//! Together AI exposes `/v1/chat/completions` and `/v1/models`.
//! Requires Bearer token authentication via `TOGETHER_API_KEY`.

use crate::chat_protocol::{
    ChatResponse, build_chat_request, chat_response_to_result, stream_chat_completion,
    validate_prompt,
};
use crate::config::InferenceConfig;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Together AI backend for chat completions and model listing.
pub struct TogetherBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

/// A model entry returned by Together AI's `/v1/models` endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct TogetherModel {
    pub id: String,
    pub object: String,
    pub created: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TogetherModelList {
    data: Vec<TogetherModel>,
}

impl TogetherBackend {
    /// Create a new Together backend from inference config.
    ///
    /// Returns an error if `together_api_key` is empty.
    ///
    /// expect: "The system creates provider membranes requiring valid API keys"
    /// \[P4\] Motivating: Clear Boundaries — Together AI provider membrane requires valid API key
    /// pre:  config.together_api_key is set
    /// post: returns TogetherBackend with configured HTTP client
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.together_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "Together AI API key not configured (set TOGETHER_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(InferenceError::Connection)?;
        Ok(Self {
            base_url: config.together_base_url.clone(),
            api_key: config.together_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to Together AI.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated text generation
    /// pre:  model is a valid Together AI model name
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
        tools: Option<&[ChatToolDefinition]>,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        let tools = tools.map(|t| t.to_vec());
        let request = build_chat_request(model, prompt, None, params, Some(false), Some(5), tools);

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("Together AI request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "Together AI returned {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Together AI JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "cns.inference",
            provider = "TG",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "Together AI inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from Together AI via SSE.
    /// Generate a streaming completion from Together.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated streaming text generation
    /// pre:  model is a valid Together model name
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
    /// pre:  model is a valid Together AI vision-capable model name
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
            None,
        );

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("Together AI vision request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "Together AI returned {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Together AI vision JSON parse: {}", e)))?;

        chat_response_to_result(chat_response)
    }

    /// List available models from Together AI.
    ///
    /// expect: "I can discover available models across providers"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — model variety discovery
    /// pre:  self.client and self.base_url are initialized
    /// post: returns Ok(`Vec<TogetherModel>`) with all available models
    /// post: if API returns non-success → Err(InferenceError::Connection)
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn list_models(&self) -> Result<Vec<TogetherModel>, InferenceError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("Together AI models request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "Together AI models error {}: {}",
                status, body
            )));
        }

        let list: TogetherModelList = response.json().await.map_err(|e| {
            InferenceError::Connection(format!("Together AI models parse error: {}", e))
        })?;

        info!(
            target: "hkask.inference.together",
            count = list.data.len(),
            "Fetched Together AI model list"
        );

        Ok(list.data)
    }
}
