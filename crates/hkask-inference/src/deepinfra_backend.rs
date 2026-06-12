//! DeepInfra backend — cloud inference via OpenAI-compatible API.
//!
//! DeepInfra exposes `/v1/chat/completions` and `/v1/models` at
//! `https://api.deepinfra.com/v1/openai`. Requires Bearer token
//! authentication via `DI_API_KEY`.
//!
//! DeepInfra has the broadest open-source model catalog and the
//! lowest per-token pricing among GPU cloud providers.

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

/// DeepInfra backend for chat completions and model listing.
pub struct DeepInfraBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl DeepInfraBackend {
    /// Create a new DeepInfra backend from the inference config.
    ///
    /// Returns an error if `deepinfra_api_key` is empty.
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.deepinfra_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "DeepInfra API key not configured (set DI_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| InferenceError::Connection(e))?;
        Ok(Self {
            base_url: config.deepinfra_base_url.clone(),
            api_key: config.deepinfra_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to DeepInfra.
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
                "DeepInfra status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("DeepInfra JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "DI",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "DeepInfra inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from DeepInfra via SSE.
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
                        "DeepInfra streaming status {}: {}",
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

    /// List models from DeepInfra via `/v1/models`, filtered to last 6 months.
    pub async fn list_models(&self) -> Result<Vec<DeepInfraModelEntry>, InferenceError> {
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

        let list: DeepInfraModelList = response
            .json()
            .await
            .map_err(|_| InferenceError::Json("DeepInfra models parse error".into()))?;

        // Filter to models updated in the last 6 months
        let cutoff = chrono::Utc::now() - chrono::Duration::days(180);
        let filtered: Vec<DeepInfraModelEntry> = list
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

// ── DeepInfra model types ────────────────────────────────────────────────────

/// A model entry from DeepInfra's `/v1/models` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepInfraModelEntry {
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
struct DeepInfraModelList {
    data: Vec<DeepInfraModelEntry>,
}
