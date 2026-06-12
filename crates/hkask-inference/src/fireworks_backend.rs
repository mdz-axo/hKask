//! Fireworks.ai backend — cloud inference via OpenAI-compatible API.
//!
//! Fireworks exposes `/v1/chat/completions` and `/v1/models`.
//! Requires Bearer token authentication via `FW_API_KEY`.

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

/// Fireworks.ai backend for chat completions and model listing.
pub struct FireworksBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl FireworksBackend {
    /// Create a new Fireworks backend from the inference config.
    ///
    /// Returns an error if `fireworks_api_key` is empty.
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.fireworks_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "Fireworks API key not configured (set FW_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| InferenceError::Connection(e))?;
        Ok(Self {
            base_url: config.fireworks_base_url.clone(),
            api_key: config.fireworks_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to Fireworks.
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
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "Fireworks status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("Fireworks JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "FW",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "Fireworks inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from Fireworks via SSE.
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
                        "Fireworks streaming status {}: {}",
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

    /// List models from Fireworks via `/v1/models`, filtered to last 6 months.
    pub async fn list_models(&self) -> Result<Vec<FireworksModelEntry>, InferenceError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(Vec::new()); // Graceful degradation
        }

        let list: FireworksModelList = response
            .json()
            .await
            .map_err(|_| InferenceError::Json("Fireworks models parse error".into()))?;

        // Filter to models updated in the last 6 months
        let cutoff = chrono::Utc::now() - chrono::Duration::days(180);
        let filtered: Vec<FireworksModelEntry> = list
            .data
            .into_iter()
            .filter(|m| {
                m.created_at
                    .as_ref()
                    .and_then(|ts| {
                        chrono::DateTime::parse_from_rfc3339(ts)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    })
                    .map(|dt| dt >= cutoff)
                    .unwrap_or(false)
            })
            .collect();

        Ok(filtered)
    }
}

// ── Fireworks model types ────────────────────────────────────────────────────

/// A model entry from Fireworks' `/v1/models` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FireworksModelEntry {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

/// OpenAI-compatible model list response.
#[derive(Debug, Deserialize)]
struct FireworksModelList {
    data: Vec<FireworksModelEntry>,
}
