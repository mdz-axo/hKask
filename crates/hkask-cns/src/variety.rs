//! Variety counters — Ashby's Law monitoring
//!
//! Implements variety tracking for cybernetic control per Ashby's Law of Requisite Variety.
//! The system must have at least as many states as the environment it controls.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default variety counter window duration (1 minute).
pub(crate) const DEFAULT_VARIETY_WINDOW_SECS: u64 = 60;

/// Variety counter for tracking state diversity in a domain
#[derive(Debug, Clone)]
pub(crate) struct VarietyTracker {
    counts: HashMap<String, u64>,
    window_start: Instant,
    window_duration: Duration,
}

impl VarietyTracker {
    /// Create a new variety counter with default 1-minute window
    pub(crate) fn new() -> Self {
        Self {
            counts: HashMap::new(),
            window_start: Instant::now(),
            window_duration: Duration::from_secs(DEFAULT_VARIETY_WINDOW_SECS),
        }
    }

    /// Increment count for a key
    pub(crate) fn increment(&mut self, key: &str) {
        self.check_window();
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Get total variety (number of distinct states observed)
    pub(crate) fn variety(&self) -> u64 {
        self.counts.len() as u64
    }

    /// Calculate variety deficit against expected variety
    pub(crate) fn deficit(&self, expected_variety: u64) -> u64 {
        expected_variety.saturating_sub(self.variety())
    }

    /// Check if window has expired and reset if needed
    fn check_window(&mut self) {
        if self.window_start.elapsed() > self.window_duration {
            self.reset();
        }
    }

    /// Reset the counter and window
    pub(crate) fn reset(&mut self) {
        self.counts.clear();
        self.window_start = Instant::now();
    }
}

impl Default for VarietyTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Variety monitor for multiple domains
#[derive(Debug)]
pub struct VarietyMonitor {
    counters: HashMap<String, VarietyTracker>,
}

impl VarietyMonitor {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Get or create a counter for a domain
    pub(crate) fn counter(&mut self, domain: &str) -> &mut VarietyTracker {
        self.counters.entry(domain.to_string()).or_default()
    }

    /// Get variety count for a specific domain
    pub fn variety_for_domain(&self, domain: &str) -> u64 {
        self.counters.get(domain).map(|c| c.variety()).unwrap_or(0)
    }

    /// Get all domain names
    pub fn domains(&self) -> Vec<&str> {
        self.counters.keys().map(|s| s.as_str()).collect()
    }

    /// Get all counters (public accessor)
    pub(crate) fn counters(&self) -> &HashMap<String, VarietyTracker> {
        &self.counters
    }
}

impl Default for VarietyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── VarietyTracker ─────────────────────────────────────────────────

    #[test]
    fn tracker_starts_empty() {
        let tracker = VarietyTracker::new();
        assert_eq!(tracker.variety(), 0);
    }

    #[test]
    fn increment_creates_new_key() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("inference");
        assert_eq!(tracker.variety(), 1);
    }

    #[test]
    fn increment_same_key_doesnt_increase_variety() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("inference");
        tracker.increment("inference");
        assert_eq!(tracker.variety(), 1); // Same key, still 1
    }

    #[test]
    fn increment_different_keys_increases_variety() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("inference");
        tracker.increment("memory");
        tracker.increment("governance");
        assert_eq!(tracker.variety(), 3);
    }

    #[test]
    fn deficit_saturating_sub() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("a");
        tracker.increment("b");
        // variety = 2, expected = 10 → deficit = 8
        assert_eq!(tracker.deficit(10), 8);
    }

    #[test]
    fn deficit_zero_when_variety_meets_expected() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("a");
        tracker.increment("b");
        tracker.increment("c");
        assert_eq!(tracker.deficit(3), 0);
    }

    #[test]
    fn deficit_saturates_at_zero_when_surplus() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("a");
        tracker.increment("b");
        tracker.increment("c");
        tracker.increment("d");
        tracker.increment("e");
        // variety = 5, expected = 3 → deficit should not go negative
        assert_eq!(tracker.deficit(3), 0);
    }

    #[test]
    fn reset_clears_all_counts() {
        let mut tracker = VarietyTracker::new();
        tracker.increment("a");
        tracker.increment("b");
        tracker.reset();
        assert_eq!(tracker.variety(), 0);
    }

    #[test]
    fn default_matches_new() {
        let a = VarietyTracker::new();
        let b = VarietyTracker::default();
        assert_eq!(a.variety(), b.variety());
    }

    // ── VarietyMonitor ─────────────────────────────────────────────────

    #[test]
    fn monitor_starts_empty() {
        let monitor = VarietyMonitor::new();
        assert!(monitor.domains().is_empty());
    }

    #[test]
    fn monitor_variety_for_untracked_domain_is_zero() {
        let monitor = VarietyMonitor::new();
        assert_eq!(monitor.variety_for_domain("unknown"), 0);
    }

    #[test]
    fn monitor_tracks_domains_independently() {
        let mut monitor = VarietyMonitor::new();
        monitor.counter("inference").increment("chat");
        monitor.counter("inference").increment("embed");
        monitor.counter("memory").increment("store");
        assert_eq!(monitor.variety_for_domain("inference"), 2);
        assert_eq!(monitor.variety_for_domain("memory"), 1);
    }

    #[test]
    fn monitor_domains_lists_tracked() {
        let mut monitor = VarietyMonitor::new();
        monitor.counter("a").increment("x");
        monitor.counter("b").increment("x");
        let mut domains = monitor.domains();
        domains.sort();
        assert_eq!(domains, vec!["a", "b"]);
    }
}
