//! Confidence-Based Router for Okapi inference
//!
//! Calculates confidence from token probabilities and escalates to larger models
//! when confidence is below threshold.
//!
//! `compute_confidence` is canonical in `hkask_types::ports`; this module
//! provides `ConfidenceConfig` for ensemble-specific escalation thresholds.

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

/// Re-export canonical `compute_confidence` from hkask-types.
pub use hkask_types::ports::compute_confidence;
