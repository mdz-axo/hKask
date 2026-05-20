//! Variety counters — Ashby's Law monitoring
//!
//! Implements variety tracking for cybernetic control per Ashby's Law of Requisite Variety.
//! The system must have at least as many states as the environment it controls.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Variety counter for tracking state diversity in a domain
#[derive(Debug)]
pub struct VarietyCounter {
    counts: HashMap<String, u64>,
    window_start: Instant,
    window_duration: Duration,
}

impl VarietyCounter {
    /// Create a new variety counter with default 1-minute window
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            window_start: Instant::now(),
            window_duration: Duration::from_secs(60),
        }
    }

    /// Create a new variety counter with custom window
    pub fn with_window(duration: Duration) -> Self {
        Self {
            counts: HashMap::new(),
            window_start: Instant::now(),
            window_duration: duration,
        }
    }

    /// Increment count for a key
    pub fn increment(&mut self, key: &str) {
        self.check_window();
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Get count for a key
    pub fn get(&self, key: &str) -> u64 {
        *self.counts.get(key).unwrap_or(&0)
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

    /// Calculate count deficit against expected count
    pub fn count_deficit(&self, expected_count: u64) -> u64 {
        expected_count.saturating_sub(self.total())
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

    /// Get the most frequently observed states (top N)
    pub fn top(&self, n: usize) -> Vec<(String, u64)> {
        let mut items: Vec<_> = self.counts.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1));
        items
            .into_iter()
            .take(n)
            .map(|(k, v)| (k.clone(), *v))
            .collect()
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
}

impl Default for VarietyCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Variety monitor for multiple domains
#[derive(Debug)]
pub struct VarietyMonitor {
    counters: HashMap<String, VarietyCounter>,
}

impl VarietyMonitor {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Get or create a counter for a domain
    pub fn counter(&mut self, domain: &str) -> &mut VarietyCounter {
        self.counters.entry(domain.to_string()).or_default()
    }

    /// Get all domain names
    pub fn domains(&self) -> Vec<&str> {
        self.counters.keys().map(|s| s.as_str()).collect()
    }

    /// Get total variety deficit across all domains
    pub fn total_deficit(&self, expected_per_domain: u64) -> u64 {
        self.counters
            .values()
            .map(|c| c.deficit(expected_per_domain))
            .sum()
    }

    /// Check if any domain exceeds the deficit threshold
    pub fn exceeds_threshold(&self, threshold: u64) -> bool {
        self.counters
            .values()
            .any(|c| c.deficit(u64::MAX) > threshold)
    }
}

impl Default for VarietyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

