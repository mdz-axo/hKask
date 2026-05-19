//! Variety counters — Ashby's Law monitoring

use std::collections::HashMap;

/// Variety counter for a domain
pub struct VarietyCounter {
    counts: HashMap<String, u64>,
}

impl VarietyCounter {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn get(&self, key: &str) -> u64 {
        *self.counts.get(key).unwrap_or(&0)
    }

    pub fn deficit(&self, expected: u64) -> u64 {
        let total: u64 = self.counts.values().sum();
        expected.saturating_sub(total)
    }
}

impl Default for VarietyCounter {
    fn default() -> Self {
        Self::new()
    }
}
