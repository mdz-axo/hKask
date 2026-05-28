//! Confidence-Based Router for Okapi inference
//!
//! Calculates confidence from token probabilities and escalates to larger models when confidence is below threshold.
//! Uses hexagonal architecture: depends on InferenceClient port, not concrete HTTP client.

use crate::ports::{GenerateRequest, GenerateResponse, InferenceClient, TokenProbability};
use thiserror::Error;

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

/// Confidence-based router with escalation
///
/// Uses dependency injection: receives InferenceClient trait object,
/// allowing test mocks and different infrastructure implementations.
pub struct ConfidenceRouter<C: InferenceClient> {
    config: ConfidenceConfig,
    inference_client: C,
}

impl<C: InferenceClient> ConfidenceRouter<C> {
    pub fn new(config: ConfidenceConfig, inference_client: C) -> Self {
        Self {
            config,
            inference_client,
        }
    }

    /// Generate response with confidence-based escalation
    #[tracing::instrument(
        skip(self),
        fields(
            model = %request.model,
            threshold = self.config.threshold,
            escalate_to = %self.config.escalate_to_model
        )
    )]
    pub async fn generate_with_escalation(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, RouterError<C::Error>> {
        tracing::debug!(
            target: "hkask.ensemble.confidence",
            confidence_threshold = self.config.threshold,
            escalate_to_model = %self.config.escalate_to_model,
            "Starting confidence-based generation"
        );

        let mut current_request = request.clone();
        if current_request.options.is_none() {
            current_request.options = Some(crate::ports::GenerateOptions {
                n_probs: Some(self.config.n_probs),
                temperature: None,
                max_tokens: None,
            });
        } else if let Some(ref mut opts) = current_request.options
            && opts.n_probs.is_none()
        {
            opts.n_probs = Some(self.config.n_probs);
        }

        let response = self
            .inference_client
            .generate(&current_request)
            .await
            .map_err(RouterError::InferenceError)?;

        if let Some(probs) = &response.completion_probabilities {
            let confidence = compute_confidence(probs);

            tracing::debug!(
                target: "hkask.ensemble.confidence",
                confidence = %confidence,
                threshold = %self.config.threshold,
                "Confidence calculated"
            );

            if confidence < self.config.threshold {
                tracing::info!(
                    target: "hkask.ensemble.confidence.escalation",
                    confidence = %confidence,
                    threshold = %self.config.threshold,
                    primary_model = %current_request.model,
                    escalated_model = %self.config.escalate_to_model,
                    "Low confidence detected, escalating to larger model"
                );

                let mut escalate_request = current_request.clone();
                escalate_request.model = self.config.escalate_to_model.clone();

                return self
                    .inference_client
                    .generate(&escalate_request)
                    .await
                    .map_err(RouterError::InferenceError);
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

    let avg_prob: f64 = probs.iter().map(|p| p.prob).sum::<f64>() / probs.len() as f64;

    let variance: f64 = probs
        .iter()
        .map(|p| (p.prob - avg_prob).powi(2))
        .sum::<f64>()
        / probs.len() as f64;

    avg_prob * (1.0 - variance.sqrt())
}

#[derive(Debug, Error)]
pub enum RouterError<E: std::error::Error + Send + Sync> {
    #[error("Inference error: {0}")]
    InferenceError(E),
}
