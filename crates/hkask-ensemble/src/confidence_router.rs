//! Confidence-Based Router for Okapi inference
//!
//! Calculates confidence from token probabilities and escalates to larger models when confidence is below threshold.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Token probability from Okapi response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenProbability {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TokenProb>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
}

/// Okapi generate/chat response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OkapiResponse {
    pub response: String,
    pub completion_probabilities: Option<Vec<TokenProbability>>,
}

/// Generate request for Okapi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub options: Option<GenerateOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    pub n_probs: Option<i32>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
}

/// Confidence configuration (from template frontmatter or default)
#[derive(Debug, Clone)]
pub struct ConfidenceConfig {
    pub threshold: f64,
    pub escalate_to_model: String,
    pub n_probs: i32,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            threshold: 0.75,
            escalate_to_model: "qwen3:70b".to_string(),
            n_probs: 5,
        }
    }
}

/// Okapi client trait for mocking
#[async_trait::async_trait]
pub trait OkapiClientTrait {
    async fn generate(&self, request: &GenerateRequest) -> Result<OkapiResponse, RouterError>;
}

/// HTTP-based Okapi client
pub struct OkapiClient {
    base_url: String,
}

impl OkapiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl OkapiClientTrait for OkapiClient {
    async fn generate(&self, request: &GenerateRequest) -> Result<OkapiResponse, RouterError> {
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/api/generate", self.base_url))
            .json(request)
            .send()
            .await
            .map_err(|e| RouterError::OkapiError(e.to_string()))?;

        let result: OkapiResponse = response
            .json()
            .await
            .map_err(|e| RouterError::OkapiError(e.to_string()))?;

        Ok(result)
    }
}

/// Confidence-based router with escalation
pub struct ConfidenceRouter<C: OkapiClientTrait> {
    config: ConfidenceConfig,
    okapi_client: C,
}

impl<C: OkapiClientTrait> ConfidenceRouter<C> {
    pub fn new(config: ConfidenceConfig, okapi_client: C) -> Self {
        Self { config, okapi_client }
    }

    /// Generate response with confidence-based escalation
    pub async fn generate_with_escalation(
        &self,
        request: &GenerateRequest,
    ) -> Result<OkapiResponse, RouterError> {
        tracing::debug!(
            "Generating with confidence threshold: {:.2}, escalate to: {}",
            self.config.threshold,
            self.config.escalate_to_model
        );

        let mut current_request = request.clone();
        if current_request.options.is_none() {
            current_request.options = Some(GenerateOptions {
                n_probs: Some(self.config.n_probs),
                temperature: None,
                max_tokens: None,
            });
        } else if let Some(ref mut opts) = current_request.options {
            if opts.n_probs.is_none() {
                opts.n_probs = Some(self.config.n_probs);
            }
        }

        let response = self.okapi_client.generate(&current_request).await?;

        if let Some(probs) = &response.completion_probabilities {
            let confidence = compute_confidence(probs);

            tracing::debug!(
                "Calculated confidence: {:.3} (threshold: {:.3})",
                confidence,
                self.config.threshold
            );

            if confidence < self.config.threshold {
                tracing::info!(
                    "Low confidence ({:.3} < {:.3}), escalating to {}",
                    confidence,
                    self.config.threshold,
                    self.config.escalate_to_model
                );

                let mut escalate_request = current_request.clone();
                escalate_request.model = self.config.escalate_to_model.clone();

                return self.okapi_client.generate(&escalate_request).await;
            }
        }

        Ok(response)
    }
}

/// Compute confidence score from token probabilities
/// Formula: avg(prob) × (1 - sqrt(variance))
pub fn compute_confidence(probs: &[TokenProbability]) -> f64 {
    if probs.is_empty() {
        return 0.0;
    }

    let avg_prob: f64 = probs.iter()
        .map(|p| p.prob)
        .sum::<f64>() / probs.len() as f64;

    let variance: f64 = probs.iter()
        .map(|p| (p.prob - avg_prob).powi(2))
        .sum::<f64>() / probs.len() as f64;

    avg_prob * (1.0 - variance.sqrt())
}

#[derive(Debug, Error)]
pub enum RouterError {
    #[error("Okapi client error: {0}")]
    OkapiError(String),

    #[error("Escalation failed: {0}")]
    EscalationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_confidence_formula_high_confidence() {
        let probs = vec![
            TokenProbability { token: "a".to_string(), prob: 0.95, top_k: vec![] },
            TokenProbability { token: "b".to_string(), prob: 0.92, top_k: vec![] },
            TokenProbability { token: "c".to_string(), prob: 0.94, top_k: vec![] },
        ];
        let confidence = compute_confidence(&probs);
        assert!(confidence > 0.85);
    }

    #[test]
    fn test_confidence_formula_low_confidence() {
        let probs = vec![
            TokenProbability { token: "a".to_string(), prob: 0.60, top_k: vec![] },
            TokenProbability { token: "b".to_string(), prob: 0.35, top_k: vec![] },
            TokenProbability { token: "c".to_string(), prob: 0.50, top_k: vec![] },
        ];
        let confidence = compute_confidence(&probs);
        assert!(confidence < 0.60);
    }

    #[test]
    fn test_confidence_formula_empty() {
        let probs: Vec<TokenProbability> = vec![];
        let confidence = compute_confidence(&probs);
        assert_eq!(confidence, 0.0);
    }

    #[test]
    fn test_confidence_config_default() {
        let config = ConfidenceConfig::default();
        assert_eq!(config.threshold, 0.75);
        assert_eq!(config.escalate_to_model, "qwen3:70b");
        assert_eq!(config.n_probs, 5);
    }

    struct MockClient {
        responses: Arc<Mutex<Vec<Result<OkapiResponse, RouterError>>>>,
    }

    impl MockClient {
        fn new(responses: Vec<Result<OkapiResponse, RouterError>>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    #[async_trait::async_trait]
    impl OkapiClientTrait for MockClient {
        async fn generate(&self, _request: &GenerateRequest) -> Result<OkapiResponse, RouterError> {
            let mut responses = self.responses.lock().await;
            responses.remove(0)
        }
    }

    #[tokio::test]
    async fn test_high_confidence_no_escalation() {
        let config = ConfidenceConfig::default();
        let client = MockClient::new(vec![
            Ok(OkapiResponse {
                response: "Paris".to_string(),
                completion_probabilities: Some(vec![
                    TokenProbability { token: "Paris".to_string(), prob: 0.95, top_k: vec![] },
                    TokenProbability { token: " is".to_string(), prob: 0.92, top_k: vec![] },
                    TokenProbability { token: " the".to_string(), prob: 0.94, top_k: vec![] },
                ]),
            }),
        ]);

        let router = ConfidenceRouter::new(config, client);
        let request = GenerateRequest {
            model: "qwen3:8b".to_string(),
            prompt: "What is the capital of France?".to_string(),
            options: None,
        };

        let response = router.generate_with_escalation(&request).await.unwrap();
        assert_eq!(response.response, "Paris");
    }

    #[tokio::test]
    async fn test_low_confidence_escalation() {
        let config = ConfidenceConfig::default();
        let client = MockClient::new(vec![
            Ok(OkapiResponse {
                response: "Maybe Paris".to_string(),
                completion_probabilities: Some(vec![
                    TokenProbability { token: "Maybe".to_string(), prob: 0.60, top_k: vec![] },
                    TokenProbability { token: " Paris".to_string(), prob: 0.50, top_k: vec![] },
                ]),
            }),
            Ok(OkapiResponse {
                response: "Paris".to_string(),
                completion_probabilities: Some(vec![
                    TokenProbability { token: "Paris".to_string(), prob: 0.98, top_k: vec![] },
                ]),
            }),
        ]);

        let router = ConfidenceRouter::new(config, client);
        let request = GenerateRequest {
            model: "qwen3:8b".to_string(),
            prompt: "What is the capital of France?".to_string(),
            options: None,
        };

        let response = router.generate_with_escalation(&request).await.unwrap();
        assert_eq!(response.response, "Paris");
    }
}
