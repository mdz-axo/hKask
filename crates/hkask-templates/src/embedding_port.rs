//! Okapi embedding generation port for sentence vectorization
//
//! `EmbeddingGenerationPort` trait lives in hkask-types (port membrane).
//! This module provides `OkapiEmbedding` — the concrete implementation.

use crate::okapi_config::OkapiConfig;
use hkask_types::cns::RetryConfig;
use hkask_types::ports::{EmbeddingGenerationError, EmbeddingGenerationPort};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Default embedding model for Okapi
pub const DEFAULT_EMBEDDING_MODEL: &str = "qwen3-embedding:0.6b";

/// Okapi-backed embedding generation implementation
pub struct OkapiEmbedding {
    model: String,
    config: OkapiConfig,
    retry_config: RetryConfig,
    client: Arc<reqwest::Client>,
    /// Embedding dimension for the current model (e.g., 384 for qwen3-embedding:0.6b)
    dim: usize,
}

impl OkapiEmbedding {
    /// Create a new OkapiEmbedding with the default model
    pub fn new(config: OkapiConfig) -> Result<Self, EmbeddingGenerationError> {
        Self::with_model(DEFAULT_EMBEDDING_MODEL, config)
    }

    /// Create OkapiEmbedding with a specific model
    pub fn with_model(model: &str, config: OkapiConfig) -> Result<Self, EmbeddingGenerationError> {
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| EmbeddingGenerationError::Connection(e.to_string()))?;

        let dim = embedding_dim_for_model(model);

        Ok(Self {
            model: model.to_string(),
            retry_config: RetryConfig::default(),
            config,
            client,
            dim,
        })
    }

    /// Create OkapiEmbedding with a shared HTTP client
    pub fn with_shared_client(
        model: &str,
        config: OkapiConfig,
        client: Arc<reqwest::Client>,
    ) -> Self {
        Self {
            model: model.to_string(),
            retry_config: RetryConfig::default(),
            config,
            client,
            dim: embedding_dim_for_model(model),
        }
    }

    /// Default local Okapi endpoint (no auth)
    pub fn local() -> Result<Self, EmbeddingGenerationError> {
        Self::new(OkapiConfig::local_dev())
    }

    /// Builder method to override the model
    pub fn model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self.dim = embedding_dim_for_model(model);
        self
    }

    /// Execute HTTP request to Okapi embedding API
    async fn execute_request(
        &self,
        request: OkapiEmbedRequest,
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        let span =
            tracing::debug_span!(target: "cns.inference", "embedding_request", model = %self.model);
        let _enter = span.enter();

        let mut req = self
            .client
            .post(format!("{}/api/embed/sentences", self.config.base_url))
            .json(&request);

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
}

#[async_trait::async_trait]
impl EmbeddingGenerationPort for OkapiEmbedding {
    async fn embed_sentences(
        &self,
        sentences: &[&str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        if sentences.is_empty() {
            return Err(EmbeddingGenerationError::EmptyResponse);
        }

        let request = OkapiEmbedRequest {
            model: self.model.clone(),
            sentences: sentences.iter().map(|s| s.to_string()).collect(),
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

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

// =============================================================================
// Model dimension lookup
// =============================================================================

/// Return the embedding dimension for a known model, or a sensible default.
fn embedding_dim_for_model(model: &str) -> usize {
    // Common Okapi embedding models and their dimensions
    match model {
        "qwen3-embedding:0.6b" => 384,
        "nomic-embed-text" => 768,
        "mxbai-embed-large" => 1024,
        "all-minilm" => 384,
        _ => {
            // Default to 384 — will be validated on first API call
            384
        }
    }
}

// =============================================================================
// Okapi wire-format types (private)
// =============================================================================

/// Okapi embed API request structure
#[derive(Debug, Clone, Serialize)]
struct OkapiEmbedRequest {
    model: String,
    sentences: Vec<String>,
}

/// Okapi embed API response structure
#[derive(Debug, Deserialize)]
struct OkapiEmbedResponse {
    #[allow(dead_code)]
    model: String,
    embeddings: Vec<Vec<f32>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_qwen3_embedding() {
        assert_eq!(DEFAULT_EMBEDDING_MODEL, "qwen3-embedding:0.6b");
    }

    #[test]
    fn embedding_dim_lookup_known_models() {
        assert_eq!(embedding_dim_for_model("qwen3-embedding:0.6b"), 384);
        assert_eq!(embedding_dim_for_model("nomic-embed-text"), 768);
        assert_eq!(embedding_dim_for_model("mxbai-embed-large"), 1024);
        assert_eq!(embedding_dim_for_model("all-minilm"), 384);
    }

    #[test]
    fn embedding_dim_lookup_unknown_defaults_to_384() {
        assert_eq!(embedding_dim_for_model("some-unknown-model"), 384);
    }

    #[test]
    fn with_model_sets_dim() {
        let config = OkapiConfig::local_dev();
        let emb = OkapiEmbedding::with_model("nomic-embed-text", config).unwrap();
        assert_eq!(emb.dim, 768);
        assert_eq!(emb.model, "nomic-embed-text");
    }

    #[test]
    fn model_builder_overrides() {
        let config = OkapiConfig::local_dev();
        let emb = OkapiEmbedding::with_model("qwen3-embedding:0.6b", config)
            .unwrap()
            .model("nomic-embed-text");
        assert_eq!(emb.model, "nomic-embed-text");
        assert_eq!(emb.dim, 768);
    }

    #[tokio::test]
    async fn embed_sentences_empty_input_returns_error() {
        let config = OkapiConfig::local_dev();
        let emb = OkapiEmbedding::new(config).unwrap();
        let result = emb.embed_sentences(&[]).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingGenerationError::EmptyResponse => {}
            other => panic!("Expected EmptyResponse, got {:?}", other),
        }
    }
}
