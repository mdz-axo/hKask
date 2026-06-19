//! Embedding router — multi-provider embedding generation.
//!
//! Routes embedding requests to DeepInfra based on
//! the 2-letter provider prefix. DeepInfra uses OpenAI-compatible `/v1/embeddings`.

use hkask_rsolidity::contract;

use crate::config::{InferenceConfig, ProviderId};
use hkask_types::ports::EmbeddingGenerationError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Multi-provider embedding router.
///
/// DeepInfra uses `/v1/embeddings` (OpenAI).
pub struct EmbeddingRouter {
    config: InferenceConfig,
    deepinfra_client: Option<Arc<reqwest::Client>>,
}

impl EmbeddingRouter {
    /// Create a new embedding router from an `InferenceConfig`.
    ///
    /// \[P4\] Motivating: Clear Boundaries — embedding provider membrane gated by API key
    pub fn new(config: InferenceConfig) -> Self {
        let deepinfra_client = Self::build_deepinfra_client(&config);
        Self {
            config,
            deepinfra_client,
        }
    }

    /// Create an embedding router with a shared HTTP client.
    ///
    /// \[P4\] Motivating: Clear Boundaries — embedding provider with shared connection pool
    pub fn with_client(config: &InferenceConfig, client: Arc<reqwest::Client>) -> Self {
        let deepinfra_client = if config.deepinfra_api_key.is_empty() {
            warn!(target: "cns.inference", "DeepInfra embeddings unavailable (no API key)");
            None
        } else {
            Some(client)
        };

        Self {
            config: config.clone(),
            deepinfra_client,
        }
    }

    fn build_deepinfra_client(config: &InferenceConfig) -> Option<Arc<reqwest::Client>> {
        if config.deepinfra_api_key.is_empty() {
            warn!(target: "cns.inference", "DeepInfra embeddings unavailable (no API key)");
            return None;
        }
        config
            .build_client()
            .map(Arc::new)
            .map_err(|e| warn!(target: "cns.inference", "Embedding client build failed: {}", e))
            .ok()
    }

    /// Resolve provider and stripped model name from a model identifier.
    fn resolve(&self, model: &str) -> Result<(ProviderId, String), EmbeddingGenerationError> {
        let (provider, stripped) =
            ProviderId::parse_from_model(model).unwrap_or((self.config.default_provider, model));

        let available = match provider {
            ProviderId::DeepInfra => self.deepinfra_client.is_some(),
            ProviderId::Fal => false,
            ProviderId::Together => false,
            ProviderId::Runpod | ProviderId::Baseten => false,
        };

        if !available {
            return Err(EmbeddingGenerationError::Connection(format!(
                "Provider {} is not available for embeddings",
                provider.as_str()
            )));
        }

        Ok((provider, stripped.to_string()))
    }

    /// Generate embedding vectors for multiple sentences.
    ///
    /// One vector per input sentence, same order. Dimension set by model.
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated batch embedding generation
    pub async fn embed_sentences(
        &self,
        model: &str,
        sentences: &[&str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        if sentences.is_empty() {
            return Err(EmbeddingGenerationError::EmptyResponse);
        }

        let (provider, model) = self.resolve(model)?;
        let texts: Vec<String> = sentences.iter().map(|s| s.to_string()).collect();

        let result = match provider {
            ProviderId::DeepInfra => {
                let client = self.deepinfra_client.as_ref().ok_or_else(|| {
                    EmbeddingGenerationError::Connection("DeepInfra client not initialized".into())
                })?;
                self.embed_openai(
                    client,
                    &self.config.deepinfra_base_url,
                    &self.config.deepinfra_api_key,
                    &model,
                    &texts,
                )
                .await?
            }
            ProviderId::Fal => {
                return Err(EmbeddingGenerationError::Connection(
                    "fal.ai does not support embeddings".into(),
                ));
            }
            ProviderId::Together => {
                return Err(EmbeddingGenerationError::Connection(
                    "Together AI embedding client not yet implemented".into(),
                ));
            }
            ProviderId::Runpod | ProviderId::Baseten => {
                return Err(EmbeddingGenerationError::Connection(
                    "Runpod/Baseten are adapter-composition providers".into(),
                ));
            }
        };

        if result.len() != sentences.len() {
            return Err(EmbeddingGenerationError::DimensionMismatch {
                expected: sentences.len(),
                actual: result.len(),
            });
        }

        info!(
            target: "cns.inference",
            provider = %provider.as_str(),
            model = %model,
            count = sentences.len(),
            dim = result.first().map(|v| v.len()).unwrap_or(0),
            "Embedding generation completed"
        );

        Ok(result)
    }

    /// Convenience wrapper around `embed_sentences`.
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated single embedding generation
    pub async fn embed_sentence(
        &self,
        model: &str,
        sentence: &str,
    ) -> Result<Vec<f32>, EmbeddingGenerationError> {
        let results = self.embed_sentences(model, &[sentence]).await?;
        results
            .into_iter()
            .next()
            .ok_or(EmbeddingGenerationError::EmptyResponse)
    }

    /// OpenAI-compatible embedding via `/v1/embeddings` (DeepInfra).
    async fn embed_openai(
        &self,
        client: &reqwest::Client,
        base_url: &str,
        api_key: &str,
        model: &str,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingGenerationError> {
        let request = OpenAiEmbedRequest {
            model: model.to_string(),
            input: texts.to_vec(),
        };

        let response = client
            .post(format!("{}/v1/embeddings", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingGenerationError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingGenerationError::Api(status.as_u16(), error_text));
        }

        let openai_response: OpenAiEmbedResponse = response.json().await.map_err(|e| {
            EmbeddingGenerationError::Json(format!("OpenAI embed JSON parse: {}", e))
        })?;

        let embeddings: Vec<Vec<f32>> = openai_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect();

        if embeddings.is_empty() {
            return Err(EmbeddingGenerationError::EmptyResponse);
        }

        Ok(embeddings)
    }
}

// ── Wire format types ────────────────────────────────────────────────────────

/// OpenAI-compatible embed request.
#[derive(Debug, Clone, Serialize)]
struct OpenAiEmbedRequest {
    model: String,
    input: Vec<String>,
}

/// OpenAI embed response wraps embeddings in a `data` array.
#[derive(Debug, Deserialize)]
struct OpenAiEmbedResponse {
    data: Vec<OpenAiEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingData {
    embedding: Vec<f32>,
}
