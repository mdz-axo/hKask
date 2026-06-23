//! FederationHealthModel — the Curator's model of healthy federation behavior.
//!
//! Updated by FederationSync each tick.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct FederationHealthModel {
    latency_window: Vec<u64>,
    expected_merge_frequency: f64,
    expected_member_count: usize,
    last_updated: DateTime<Utc>,
}

impl FederationHealthModel {
    pub fn new() -> Self {
        Self {
            latency_window: Vec::new(),
            expected_merge_frequency: 0.0,
            expected_member_count: 0,
            last_updated: Utc::now(),
        }
    }

    pub fn observe_latency(&mut self, latency_ms: u64) {
        self.latency_window.push(latency_ms);
        if self.latency_window.len() > 100 {
            self.latency_window.remove(0);
        }
        self.last_updated = Utc::now();
    }

    pub fn observe_merge(&mut self, count: u64) {
        self.expected_merge_frequency = 0.9 * self.expected_merge_frequency + 0.1 * count as f64;
        self.last_updated = Utc::now();
    }

    pub fn observe_member_count(&mut self, count: usize) {
        self.expected_member_count = count;
        self.last_updated = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_model_starts_empty() {
        let model = FederationHealthModel::new();
        assert_eq!(model.latency_window.len(), 0);
        assert_eq!(model.expected_merge_frequency, 0.0);
        assert_eq!(model.expected_member_count, 0);
    }

    #[test]
    fn observe_latency_rolls_window() {
        let mut model = FederationHealthModel::new();
        for i in 0..150 {
            model.observe_latency(i);
        }
        assert_eq!(model.latency_window.len(), 100);
    }

    #[test]
    fn observe_merge_ema() {
        let mut model = FederationHealthModel::new();
        model.observe_merge(10);
        assert!((model.expected_merge_frequency - 1.0).abs() < 0.01);
        model.observe_merge(10);
        assert!((model.expected_merge_frequency - 1.9).abs() < 0.01);
    }

    #[test]
    fn observe_member_count_updates() {
        let mut model = FederationHealthModel::new();
        model.observe_member_count(3);
        assert_eq!(model.expected_member_count, 3);
    }
}
