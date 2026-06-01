//! Bayesian confidence operations
//!
//! Free functions for the episodic and semantic memory subloops:
//! - `decay` — Loop 2a.3: Confidence decay (RECONCILE)
//! - `retract` — Loop 2a.4: Confidence retraction (RECONCILE)
//! - `combine` — Bayesian combination (used in semantic recall)
//! - `join` — Multi-source Bayesian combination
//! - `weighted_average` — Weighted average of confidences

/// Combine two confidence values using Bayesian rule
/// P(A,B) = P(A) * P(B) / (P(A) * P(B) + (1-P(A)) * (1-P(B)))
pub fn combine(conf1: f64, conf2: f64) -> f64 {
    let numerator = conf1 * conf2;
    let denominator = conf1 * conf2 + (1.0 - conf1) * (1.0 - conf2);

    if denominator == 0.0 {
        0.5
    } else {
        numerator / denominator
    }
}

/// Subtract confidence (retraction)
///
/// Reduces `conf1` proportionally by `conf2`.
/// Used in Loop 2a.4 (Confidence Retraction) to reduce episodic triple
/// confidence without deleting the triple.
pub fn retract(conf1: f64, conf2: f64) -> f64 {
    // Simplified retraction: reduce confidence proportionally
    (conf1 * (1.0 - conf2)).clamp(0.0, 1.0)
}

/// Join multiple confidence values
pub fn join(confidences: &[f64]) -> f64 {
    if confidences.is_empty() {
        return 0.5;
    }

    let mut result = confidences[0];
    for &conf in &confidences[1..] {
        result = combine(result, conf);
    }
    result
}

/// Decay confidence over time
///
/// Exponential decay: `confidence × e^(-rate × time_elapsed)`.
/// Used in Loop 2a.3 (Confidence Decay) to reduce episodic triple
/// confidence at recall time based on time since storage.
pub fn decay(confidence: f64, decay_rate: f64, time_elapsed: f64) -> f64 {
    // Exponential decay: conf * e^(-rate * time)
    confidence * (-decay_rate * time_elapsed).exp()
}
