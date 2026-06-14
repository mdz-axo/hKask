//! fal.ai backend — cloud inference via OpenAI-compatible API.
//!
//! fal.ai exposes `/v1/chat/completions` for text and vision models.
//! Requires Bearer token authentication via `FA_API_KEY`.
//!
//! Model listing: fal.ai does not expose a standard `/v1/models` endpoint.
//! Instead, a static catalog of known vision-capable models is used.

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

/// fal.ai backend for chat completions and vision inference.
#[derive(Debug)]
pub struct FalBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl FalBackend {
    /// Create a new fal.ai backend from the inference config.
    ///
    /// Returns an error if `fal_api_key` is empty.
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.fal_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "fal.ai API key not configured (set FA_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(InferenceError::Connection)?;
        Ok(Self {
            base_url: config.fal_base_url.clone(),
            api_key: config.fal_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to fal.ai.
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
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "fal.ai status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("fal.ai JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "FA",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "fal.ai inference completed"
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
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "fal.ai vision status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("fal.ai JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "FA",
            model = %result.model,
            tokens = result.usage.total_tokens,
            "fal.ai vision inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from fal.ai via SSE.
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
                    .header("Authorization", format!("Key {}", api_key))
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
                        "fal.ai streaming status {}: {}",
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

    /// List known fal.ai models from the static catalog.
    ///
    /// fal.ai does not expose a standard `/v1/models` endpoint.
    /// Returns a curated list of vision-capable models known to work
    /// with the OpenAI-compatible chat completions endpoint.
    pub async fn list_models(&self) -> Result<Vec<FalModelEntry>, InferenceError> {
        // Static catalog of known fal.ai vision models.
        // These are models confirmed to work via the chat completions endpoint.
        Ok(vec![
            FalModelEntry {
                id: "paddleocr".into(),
                description: Some("PaddleOCR — document OCR model".into()),
            },
            FalModelEntry {
                id: "nemotron-parse".into(),
                description: Some("Nemotron Parse — document parsing and OCR".into()),
            },
            FalModelEntry {
                id: "docres".into(),
                description: Some(
                    "DocRes — document enhancement: deshadow, deblur, binarize, dewarp".into(),
                ),
            },
        ])
    }
}

// ── fal.ai model types ──────────────────────────────────────────────────────

/// A model entry from fal.ai's static catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalModelEntry {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ: inf-fal-01 — Construction fails without API key
    #[test]
    fn construction_fails_without_api_key() {
        let config = InferenceConfig::default();
        assert!(config.fal_api_key.is_empty());
        let result = FalBackend::new(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("FA_API_KEY"),
            "error should mention FA_API_KEY, got: {}",
            err
        );
    }

    /// REQ: inf-fal-02 — Construction succeeds with API key
    #[test]
    fn construction_succeeds_with_api_key() {
        let config = InferenceConfig {
            fal_api_key: "test-key-123".into(),
            ..Default::default()
        };
        let result = FalBackend::new(&config);
        assert!(
            result.is_ok(),
            "should succeed with API key: {:?}",
            result.err()
        );
    }

    /// REQ: inf-fal-03 — Static catalog returns known vision models
    #[tokio::test]
    async fn static_catalog_returns_vision_models() {
        let config = InferenceConfig {
            fal_api_key: "test-key".into(),
            ..Default::default()
        };
        let backend = FalBackend::new(&config).unwrap();
        let models = backend.list_models().await.unwrap();
        assert!(!models.is_empty(), "catalog should not be empty");
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(
            ids.contains(&"paddleocr"),
            "catalog should include paddleocr"
        );
        assert!(
            ids.contains(&"nemotron-parse"),
            "catalog should include nemotron-parse"
        );
        assert!(ids.contains(&"docres"), "catalog should include docres");
    }

    /// REQ: inf-fal-04 — Vision support heuristic recognizes fal.ai models
    #[test]
    fn vision_support_heuristic_recognizes_fal_models() {
        use crate::RouterModelEntry;
        assert_eq!(
            RouterModelEntry::infer_vision_support("paddleocr", None),
            Some(true)
        );
        assert_eq!(
            RouterModelEntry::infer_vision_support("nemotron-parse", None),
            Some(true)
        );
        assert_eq!(
            RouterModelEntry::infer_vision_support("FA/paddleocr", None),
            Some(true)
        );
    }
}
