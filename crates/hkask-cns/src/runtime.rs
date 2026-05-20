//! CNS Runtime Integration
//!
//! Runtime manager for CNS monitoring, algedonic alerts, and variety tracking.
//! Provides health status and alert querying for CLI and API integration.
//!
//! Uses shared state with RwLock for compatibility with sync and async contexts.

use crate::algedonic::{AlgedonicAlert, AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD};
use crate::variety::{VarietyCounter, VarietyMonitor};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// CNS state shared between threads
struct CnsState {
    algedonic: AlgedonicManager,
    variety: VarietyMonitor,
}

impl CnsState {
    fn new(threshold: u64) -> Self {
        Self {
            algedonic: AlgedonicManager::new(threshold),
            variety: VarietyMonitor::new(),
        }
    }
}

/// CNS runtime manager
pub struct CnsRuntime {
    state: Arc<RwLock<CnsState>>,
}

impl CnsRuntime {
    /// Create new CNS runtime with default threshold
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_THRESHOLD)
    }

    /// Create CNS runtime with custom threshold
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CnsState::new(threshold))),
        }
    }

    /// Get CNS health status
    pub async fn health(&self) -> CnsHealth {
        let state = self.state.read().await;
        CnsHealth::check(&state.algedonic)
    }

    /// Get all algedonic alerts
    pub async fn alerts(&self) -> Vec<AlgedonicAlert> {
        let state = self.state.read().await;
        state.algedonic.alerts().to_vec()
    }

    /// Get critical alerts only
    pub async fn critical_alerts(&self) -> Vec<AlgedonicAlert> {
        let state = self.state.read().await;
        state
            .algedonic
            .critical_alerts()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Get variety counters for all domains
    pub async fn variety(&self) -> Vec<(String, u64)> {
        let state = self.state.read().await;
        let domains: Vec<String> = state
            .variety
            .domains()
            .iter()
            .map(|s| s.to_string())
            .collect();
        drop(state);

        let mut results = Vec::new();
        for domain in &domains {
            let state = self.state.read().await;
            let count = state.variety.variety_for_domain(domain);
            drop(state);
            results.push((domain.clone(), count));
        }
        results
    }

    /// Get variety counter for specific domain
    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        let state = self.state.read().await;
        state.variety.variety_for_domain(domain)
    }

    /// Increment variety counter for domain
    pub async fn increment_variety(&self, domain: &str, state_name: &str) {
        let mut state = self.state.write().await;
        state.variety.counter(domain).increment(state_name);
        info!(target: "cns.variety", domain = %domain, state = %state_name, "Variety incremented");
    }

    /// Check variety and generate algedonic alert if needed
    pub async fn check_variety(&self, domain: &str) -> Option<AlgedonicAlert> {
        let counter = {
            let state = self.state.read().await;
            state
                .variety
                .counters
                .get(domain)
                .cloned()
                .unwrap_or_else(VarietyCounter::new)
        };

        let mut state = self.state.write().await;
        state.algedonic.check(&counter, domain).cloned()
    }

    /// Check all domains and return count of alerts generated
    pub async fn check_all(&self) -> usize {
        let domains = {
            let state = self.state.read().await;
            state
                .variety
                .domains()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        };

        let mut count = 0;
        for domain in domains {
            let counter = {
                let state = self.state.read().await;
                state.variety.counters.get(&domain).cloned()
            };

            if let Some(counter) = counter {
                let mut state = self.state.write().await;
                if state.algedonic.check(&counter, &domain).is_some() {
                    count += 1;
                }
            }
        }
        count
    }

    /// Reset all alerts
    pub async fn reset_alerts(&self) {
        let mut state = self.state.write().await;
        state.algedonic.reset();
    }

    /// Clear old alerts (older than specified duration)
    pub async fn clear_old_alerts(&self, max_age: std::time::Duration) {
        let mut state = self.state.write().await;
        state.algedonic.clear_old(max_age);
    }

    /// Get total variety deficit across all domains
    pub async fn total_deficit(&self) -> u64 {
        let state = self.state.read().await;
        state.variety.total_deficit(DEFAULT_THRESHOLD)
    }
}

impl Default for CnsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cns_runtime_new() {
        let runtime = CnsRuntime::new();
        let health = runtime.health().await;
        assert!(health.healthy);
        assert_eq!(health.critical_count, 0);
    }

    #[tokio::test]
    async fn test_cns_runtime_variety() {
        let runtime = CnsRuntime::new();

        runtime.increment_variety("test_domain", "state_a").await;
        runtime.increment_variety("test_domain", "state_b").await;
        runtime.increment_variety("test_domain", "state_a").await;

        let variety = runtime.variety_for_domain("test_domain").await;
        assert_eq!(variety, 2);

        let all_variety = runtime.variety().await;
        assert_eq!(all_variety.len(), 1);
        assert_eq!(all_variety[0].1, 2);
    }

    #[tokio::test]
    async fn test_cns_runtime_check_variety() {
        let runtime = CnsRuntime::with_threshold(1);

        runtime.increment_variety("test", "state_a").await;
        runtime.increment_variety("test", "state_b").await;

        let alert = runtime.check_variety("test").await;
        assert!(alert.is_some());
        assert!(alert.unwrap().is_critical());
    }

    #[tokio::test]
    async fn test_cns_runtime_alerts() {
        let runtime = CnsRuntime::with_threshold(1);

        runtime.increment_variety("domain1", "a").await;
        runtime.increment_variety("domain1", "b").await;
        runtime.check_variety("domain1").await;

        let alerts = runtime.alerts().await;
        assert!(!alerts.is_empty());

        let critical = runtime.critical_alerts().await;
        assert!(!critical.is_empty());
    }

    #[tokio::test]
    async fn test_cns_runtime_reset() {
        let runtime = CnsRuntime::with_threshold(1);

        runtime.increment_variety("test", "a").await;
        runtime.increment_variety("test", "b").await;
        runtime.check_variety("test").await;

        assert!(!runtime.alerts().await.is_empty());

        runtime.reset_alerts().await;
        assert!(runtime.alerts().await.is_empty());
    }

    #[tokio::test]
    async fn test_cns_runtime_total_deficit() {
        let runtime = CnsRuntime::new();

        runtime.increment_variety("domain1", "a").await;
        runtime.increment_variety("domain2", "b").await;

        let deficit = runtime.total_deficit().await;
        assert!(deficit > 0);
    }
}
