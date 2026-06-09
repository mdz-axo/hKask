//! Confidence-Based Router for Okapi inference
//!
//! Calculates confidence from token probabilities and escalates to larger models
//! when confidence is below threshold.
//!
//! `compute_confidence` is canonical in `hkask_types::ports`; this module
//! provides `ConfidenceConfig` for ensemble-specific escalation thresholds
//! and `check_and_escalate` for automatic model escalation.

use crate::ensemble::deliberation::AgentResponse;
use crate::ensemble::ports::{GenerateOptions, GenerateRequest, InferenceClient};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Confidence configuration (from template frontmatter or default)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl ConfidenceConfig {
    /// Parse confidence config from template frontmatter (serde_json::Value).
    ///
    /// Looks for `confidence_threshold`, `escalate_to_model`, and `n_probs` keys.
    /// Falls back to defaults for missing fields.
    pub fn from_template_frontmatter(frontmatter: &serde_json::Value) -> Self {
        Self {
            threshold: frontmatter
                .get("confidence_threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.75),
            escalate_to_model: frontmatter
                .get("escalate_to_model")
                .and_then(|v| v.as_str())
                .unwrap_or("qwen3:70b")
                .to_string(),
            n_probs: frontmatter
                .get("n_probs")
                .and_then(|v| v.as_i64())
                .unwrap_or(5) as i32,
        }
    }
}

/// Check confidence of a response and escalate to a larger model if below threshold.
///
/// Returns `Some(AgentResponse)` with the escalated response if confidence was
/// below threshold and escalation succeeded. Returns `None` if confidence is
/// acceptable or if escalation fails.
pub async fn check_and_escalate<C: InferenceClient>(
    config: &ConfidenceConfig,
    response: &AgentResponse,
    inference_client: &Arc<C>,
    original_prompt: &str,
) -> Option<AgentResponse> {
    if response.confidence < config.threshold {
        let request = GenerateRequest {
            model: config.escalate_to_model.clone(),
            prompt: original_prompt.to_string(),
            options: Some(GenerateOptions {
                n_probs: Some(config.n_probs),
                temperature: Some(0.5),
                max_tokens: Some(512),
            }),
        };
        match inference_client.generate(&request).await {
            Ok(escalated) => {
                let confidence = escalated
                    .completion_probabilities
                    .as_ref()
                    .map(|probs| compute_confidence(probs))
                    .unwrap_or(response.confidence);
                Some(AgentResponse::new(
                    response.agent_webid,
                    escalated.response.trim().to_string(),
                    confidence,
                ))
            }
            Err(_) => None,
        }
    } else {
        None
    }
}

/// Re-export canonical `compute_confidence` from hkask-types.
pub use hkask_types::ports::compute_confidence;
