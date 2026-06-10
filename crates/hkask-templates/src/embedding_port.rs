//! Okapi embedding generation port for sentence vectorization
//
//! `EmbeddingGenerationPort` trait lives in hkask-types (port membrane).
//! This module provides `OkapiEmbedding` — the concrete implementation.

use crate::okapi_config::OkapiConfig;
use hkask_types::cns::RetryConfig;
use hkask_types::ports::EmbeddingGenerationError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Okapi-backed embedding generation implementation
pub struct OkapiEmbedding {
    model: String,
    config: OkapiConfig,
    retry_config: RetryConfig,
    client: Arc<reqwest::Client>,
}

impl OkapiEmbedding {
    /// Create a new OkapiEmbedding with the default model (qwen3-embedding:0.6b)
    pub fn new(config: OkapiConfig) -> Result<Self, EmbeddingGenerationError> {
        Self::with_model("qwen3-embedding:0.6b", config)
    }

    /// Create OkapiEmbedding with a specific model
    pub fn with_model(model: &str, config: OkapiConfig) -> Result<Self, EmbeddingGenerationError> {
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| EmbeddingGenerationError::Connection(e.to_string()))?;

        Ok(Self {
            model: model.to_string(),
            retry_config: RetryConfig::default(),
            config,
            client,
        })
    }

    /// Execute HTTP request to Okapi embedding API
    async fn execute_request(
        &self,
        request: OkapiEmbedRequest,
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        let span =
            tracing::debug_span!(target: "cns.inference", "embedding_request", model = %self.model);
        let _enter = span.enter();

        let endpoint = if self.config.api_key.is_some() {
            format!("{}/api/embed/sentences", self.config.base_url)
        } else {
            format!("{}/api/embed", self.config.base_url)
        };
        let mut req = self.client.post(&endpoint).json(&request);

        // Add authorization header if configured
        if let Some(auth_header) = self.config.get_authorization_header() {
            req = req.header("Authorization", auth_header);
        }

        let response = req
            .send()
            .await
            .map_err(|e| EmbeddingGenerationError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Check if retryable
            if self.retry_config.is_retryable_status(status.as_u16()) {
                return Err(EmbeddingGenerationError::Connection(format!(
                    "Retryable status {}: {}",
                    status, error_text
                )));
            }

            return Err(EmbeddingGenerationError::Api(status.as_u16(), error_text));
        }

        let okapi_response: OkapiEmbedResponse = response.json().await.map_err(|e| {
            EmbeddingGenerationError::Json(format!("Okapi embed JSON parse error: {}", e))
        })?;

        if okapi_response.embeddings.is_empty() {
            return Err(EmbeddingGenerationError::EmptyResponse);
        }

        // Validate dimensions are consistent
        let expected_dim = okapi_response.embeddings[0].len();
        for vec in okapi_response.embeddings.iter() {
            if vec.len() != expected_dim {
                return Err(EmbeddingGenerationError::DimensionMismatch {
                    expected: expected_dim,
                    actual: vec.len(),
                });
            }
        }

        Ok(okapi_response.embeddings)
    }

    /// Execute request with retry logic
    async fn execute_with_retry(
        &self,
        request: OkapiEmbedRequest,
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match self.execute_request(request.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.retry_config.max_retries {
                        let delay_ms = self.retry_config.delay_for_attempt(attempt);
                        warn!(
                            target: "cns.inference",
                            attempt = %attempt,
                            delay_ms = %delay_ms,
                            error = ?last_error,
                            "Retryable embedding error, waiting before retry"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.expect("retry loop always records the last error"))
    }

    /// Generate embedding vectors for multiple sentences.
    /// One vector per input sentence, same order. Dimension set by model.
    pub async fn embed_sentences(
        &self,
        sentences: &[&str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        if sentences.is_empty() {
            return Err(EmbeddingGenerationError::EmptyResponse);
        }

        let texts: Vec<String> = sentences.iter().map(|s| s.to_string()).collect();
        let request = if self.config.api_key.is_some() {
            OkapiEmbedRequest::for_okapi(self.model.clone(), texts)
        } else {
            OkapiEmbedRequest::for_ollama(self.model.clone(), texts)
        };

        let result = self.execute_with_retry(request).await?;

        if result.len() != sentences.len() {
            return Err(EmbeddingGenerationError::DimensionMismatch {
                expected: sentences.len(),
                actual: result.len(),
            });
        }

        info!(
            target: "cns.inference",
            model = %self.model,
            count = sentences.len(),
            dim = result.first().map(|v| v.len()).unwrap_or(0),
            "Embedding generation completed"
        );

        Ok(result)
    }

    /// Convenience wrapper around `embed_sentences`.
    pub async fn embed_sentence(
        &self,
        sentence: &str,
    ) -> Result<Vec<f32>, EmbeddingGenerationError> {
        let results = self.embed_sentences(&[sentence]).await?;
        results
            .into_iter()
            .next()
            .ok_or(EmbeddingGenerationError::EmptyResponse)
    }
}

// Okapi/Ollama wire-format types (private)

/// Request — uses Ollama's native `input` format when no API key (local mode),
/// Okapi's `sentences` format when authenticated.
#[derive(Debug, Clone, Serialize)]
struct OkapiEmbedRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sentences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<Vec<String>>,
}

impl OkapiEmbedRequest {
    fn for_okapi(model: String, sentences: Vec<String>) -> Self {
        Self {
            model,
            sentences: Some(sentences),
            input: None,
        }
    }

    fn for_ollama(model: String, input: Vec<String>) -> Self {
        Self {
            model,
            sentences: None,
            input: Some(input),
        }
    }
}

/// Okapi embed API response structure
#[derive(Debug, Deserialize)]
struct OkapiEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}
