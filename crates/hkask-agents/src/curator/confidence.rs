//! Confidence-Based Router for Okapi inference
//
//! Calculates confidence from token probabilities and escalates to larger models when confidence is below threshold.
//! Uses hexagonal architecture: depends on InferenceClient port, not concrete HTTP client.

use hkask_types::ports::TokenProbability;

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
