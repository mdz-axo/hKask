//! Bayesian confidence operations

/// Bayesian confidence combination
pub struct BayesianOps;

impl BayesianOps {
    pub fn new() -> Self {
        Self
    }

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
    pub fn retract(conf1: f64, conf2: f64) -> f64 {
        // Simplified retraction: reduce confidence proportionally
        (conf1 * (1.0 - conf2)).max(0.0).min(1.0)
    }

    /// Join multiple confidence values
    pub fn join(confidences: &[f64]) -> f64 {
        if confidences.is_empty() {
            return 0.5;
        }

        let mut result = confidences[0];
        for &conf in &confidences[1..] {
            result = Self::combine(result, conf);
        }
        result
    }

    /// Decay confidence over time
    pub fn decay(confidence: f64, decay_rate: f64, time_elapsed: f64) -> f64 {
        // Exponential decay: conf * e^(-rate * time)
        confidence * (-decay_rate * time_elapsed).exp()
    }

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
}

impl Default for BayesianOps {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_high_confidence() {
        let result = BayesianOps::combine(0.9, 0.9);
        assert!(result > 0.9); // Combined should be higher
        assert!(result < 1.0);
    }

    #[test]
    fn test_combine_low_confidence() {
        let result = BayesianOps::combine(0.1, 0.1);
        assert!(result < 0.1); // Combined should be lower
    }

    #[test]
    fn test_combine_opposite() {
        let result = BayesianOps::combine(0.9, 0.1);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_retract() {
        let result = BayesianOps::retract(0.9, 0.5);
        assert!(result < 0.9);
    }

    #[test]
    fn test_join() {
        let confidences = vec![0.8, 0.7, 0.9];
        let result = BayesianOps::join(&confidences);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_decay() {
        let result = BayesianOps::decay(1.0, 0.1, 1.0);
        assert!(result < 1.0);
        assert!(result > 0.0);
    }

    #[test]
    fn test_weighted_average() {
        let confidences = vec![(0.5, 1.0), (1.0, 2.0)];
        let result = BayesianOps::weighted_average(&confidences);
        assert!((result - 0.833).abs() < 0.01);
    }

    #[test]
    fn test_join_empty() {
        let result = BayesianOps::join(&[]);
        assert_eq!(result, 0.5);
    }
}
