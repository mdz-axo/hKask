//! Variety counters — Ashby's Law monitoring
//!
//! Implements variety tracking for cybernetic control per Ashby's Law of Requisite Variety.
//! The system must have at least as many states as the environment it controls.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Variety counter for tracking state diversity in a domain
#[derive(Debug, Clone)]
pub(crate) struct VarietyTracker {
    counts: HashMap<String, u64>,
    window_start: Instant,
    window_duration: Duration,
}

impl VarietyTracker {
    /// Create a new variety counter with default 1-minute window
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            window_start: Instant::now(),
            window_duration: Duration::from_secs(60),
        }
    }

    /// Increment count for a key
    pub fn increment(&mut self, key: &str) {
        self.check_window();
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Get total variety (number of distinct states observed)
    pub fn variety(&self) -> u64 {
        self.counts.len() as u64
    }

    /// Get total count across all states
    pub fn total(&self) -> u64 {
        self.counts.values().sum()
    }

    /// Calculate variety deficit against expected variety
    pub fn deficit(&self, expected_variety: u64) -> u64 {
        expected_variety.saturating_sub(self.variety())
    }

    /// Get entropy of the distribution (measure of variety quality)
    pub fn entropy(&self) -> f64 {
        let total = self.total() as f64;
        if total == 0.0 {
            return 0.0;
        }

        let mut entropy = 0.0;
        for &count in self.counts.values() {
            let p = count as f64 / total;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    /// Check if window has expired and reset if needed
    fn check_window(&mut self) {
        if self.window_start.elapsed() > self.window_duration {
            self.reset();
        }
    }

    /// Reset the counter and window
    pub fn reset(&mut self) {
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
    pub fn counter(&mut self, domain: &str) -> &mut VarietyTracker {
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
    pub fn counters(&self) -> &HashMap<String, VarietyTracker> {
        &self.counters
    }

    /// Get total variety deficit across all domains
    pub fn total_deficit(&self, expected_per_domain: u64) -> u64 {
        self.counters
            .values()
            .map(|c| c.deficit(expected_per_domain))
            .sum()
    }
}

impl Default for VarietyMonitor {
    fn default() -> Self {
        Self::new()
    }
}
