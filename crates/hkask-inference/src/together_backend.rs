//! Together AI backend — cloud inference via OpenAI-compatible API.
//!
//! Together AI exposes `/v1/chat/completions` and `/v1/models`.
//! Requires Bearer token authentication via `TOGETHER_API_KEY`.

use crate::chat_protocol::{
    ChatResponse, build_chat_request, chat_response_to_result, parse_sse_stream, validate_prompt,
};
use crate::config::InferenceConfig;
use futures_util::StreamExt;
use hkask_types::ports::{InferenceError, InferenceResult, InferenceStreamChunk};
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
    /// REQ: INFER-016
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
            target: "hkask.inference",
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
    /// REQ: INFER-017
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
        let model = model.to_string();
        let prompt = prompt.to_string();
        let params = params.clone();
        let client = Arc::clone(&self.client);
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        Box::pin(
            futures_util::stream::once(async move {
                let request = build_chat_request(&model, &prompt, None, &params, Some(true), None);

                let response = match client
                    .post(format!("{}/v1/chat/completions", base_url))
                    .header("Authorization", format!("Bearer {}", api_key))
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
                        "Together AI streaming status {}: {}",
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

    /// Vision/multimodal inference with base64-encoded images.
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
