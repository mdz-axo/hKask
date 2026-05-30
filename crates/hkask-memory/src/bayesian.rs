//! Bayesian confidence operations

#[allow(dead_code)]
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

#[allow(dead_code)]
/// Subtract confidence (retraction)
pub fn retract(conf1: f64, conf2: f64) -> f64 {
    // Simplified retraction: reduce confidence proportionally
    (conf1 * (1.0 - conf2)).clamp(0.0, 1.0)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
/// Decay confidence over time
pub fn decay(confidence: f64, decay_rate: f64, time_elapsed: f64) -> f64 {
    // Exponential decay: conf * e^(-rate * time)
    confidence * (-decay_rate * time_elapsed).exp()
}

#[allow(dead_code)]
/// Weighted average of confidences
pub fn weighted_average(confidences: &[(f64, f64)]) -> f64 {
    // confidences: Vec<(confidence, weight)>
    if confidences.is_empty() {
        return 0.5;
    }

    let total_weight: f64 = confidences.iter().map(|(_, w)| w).sum();
    if total_weight == 0.0 {
        return 0.5;
    }

    let weighted_sum: f64 = confidences.iter().map(|(c, w)| c * w).sum();

    weighted_sum / total_weight
}
