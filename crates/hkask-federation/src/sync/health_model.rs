//! FederationHealthModel — the Curator's model of healthy federation behavior.
//!
//! Updated by CNS federation spans. Used by Curator metacognition to detect
//! anomalies. Implements the Good Regulator theorem: every good regulator
//! must be a model of the system it regulates.

use chrono::{DateTime, Utc};
use hkask_types::Confidence;

/// Default decay rate: ln(2) / (90 × 86400) ≈ 8.913 × 10⁻⁸.
/// Same as hkask_memory::episodic::DEFAULT_DECAY_RATE (pub(crate)).
const DEFAULT_DECAY_RATE: f64 = 8.913e-8;

/// The Curator's model of what "healthy federation" looks like.
#[derive(Debug, Clone)]
pub struct FederationHealthModel {
    /// Rolling window of sync latency samples (milliseconds).
    latency_window: Vec<u64>,
    /// Expected merge frequency per interval.
    expected_merge_frequency: f64,
    /// Expected number of federation members.
    expected_member_count: usize,
    /// Model confidence — decays with staleness.
    confidence: Confidence,
    /// Last time the model was updated from CNS observations.
    last_updated: DateTime<Utc>,
}

impl FederationHealthModel {
    /// Create a new model with no observations.
    pub fn new() -> Self {
        Self {
            latency_window: Vec::new(),
            expected_merge_frequency: 0.0,
            expected_member_count: 0,
            confidence: Confidence::full(),
            last_updated: Utc::now(),
        }
    }

    /// Update the model with a new latency observation.
    pub fn observe_latency(&mut self, latency_ms: u64) {
        self.latency_window.push(latency_ms);
        // Keep a rolling window of the last 100 observations
        if self.latency_window.len() > 100 {
            self.latency_window.remove(0);
        }
        self.last_updated = Utc::now();
    }

    /// Update the expected merge frequency.
    pub fn observe_merge(&mut self, count: u64) {
        // Exponential moving average with α = 0.1
        self.expected_merge_frequency = 0.9 * self.expected_merge_frequency + 0.1 * count as f64;
        self.last_updated = Utc::now();
    }

    /// Update the expected member count.
    pub fn observe_member_count(&mut self, count: usize) {
        self.expected_member_count = count;
        self.last_updated = Utc::now();
    }

    /// Average sync latency from the rolling window.
    pub fn average_latency(&self) -> Option<u64> {
        if self.latency_window.is_empty() {
            return None;
        }
        let sum: u64 = self.latency_window.iter().sum();
        Some(sum / self.latency_window.len() as u64)
    }

    /// Model confidence — decays with time since last update.
    pub fn confidence(&self) -> Confidence {
        let elapsed = (Utc::now() - self.last_updated).num_seconds() as f64;
        // Confidence decays at the same rate as episodic memory (90-day half-life)
        self.confidence.decay(DEFAULT_DECAY_RATE, elapsed)
    }

    /// Expected merge frequency.
    pub fn expected_merge_frequency(&self) -> f64 {
        self.expected_merge_frequency
    }

    /// Expected member count.
    pub fn expected_member_count(&self) -> usize {
        self.expected_member_count
    }

    /// Compute anomaly score for a current observation.
    /// Higher score = more anomalous. 0.0 = perfectly normal.
    pub fn anomaly_score(
        &self,
        current_latency_ms: u64,
        current_merge_count: u64,
        current_member_count: usize,
    ) -> f64 {
        let mut score = 0.0;

        // Latency anomaly: how many multiples of the average?
        if let Some(avg) = self.average_latency() {
            if avg > 0 {
                let ratio = current_latency_ms as f64 / avg as f64;
                if ratio > 2.0 {
                    score += (ratio - 1.0).min(5.0);
                }
            }
        }

        // Merge frequency anomaly: sudden drops
        if self.expected_merge_frequency > 0.0 && current_merge_count == 0 {
            score += 2.0;
        }

        // Member count anomaly: unexpected changes
        if self.expected_member_count > 0 && current_member_count != self.expected_member_count {
            let delta =
                (self.expected_member_count as i64 - current_member_count as i64).unsigned_abs();
            score += delta as f64;
        }

        score
    }
}

impl Default for FederationHealthModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_model_has_no_latency() {
        let model = FederationHealthModel::new();
        assert!(model.average_latency().is_none());
    }

    #[test]
    fn latency_window_averages_correctly() {
        let mut model = FederationHealthModel::new();
        model.observe_latency(100);
        model.observe_latency(200);
        assert_eq!(model.average_latency(), Some(150));
    }

    #[test]
    fn anomaly_score_zero_when_normal() {
        let mut model = FederationHealthModel::new();
        model.observe_latency(100);
        model.observe_latency(100);
        model.observe_merge(5);
        model.observe_member_count(3);

        // Current observation matches expectations
        let score = model.anomaly_score(100, 5, 3);
        assert!(score < 1.0, "expected low anomaly score, got {score}");
    }

    #[test]
    fn anomaly_score_high_when_latency_spikes() {
        let mut model = FederationHealthModel::new();
        model.observe_latency(100);
        model.observe_latency(100);

        let score = model.anomaly_score(5000, 5, 3);
        assert!(score > 1.0, "expected high anomaly score, got {score}");
    }

    #[test]
    fn confidence_decays_with_time() {
        let model = FederationHealthModel::new();
        let initial = model.confidence();
        // Confidence should be 1.0 initially
        assert!((initial.value() - 1.0).abs() < 0.001);
    }
}
