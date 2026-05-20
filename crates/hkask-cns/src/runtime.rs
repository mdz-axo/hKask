//! CNS Runtime Integration
//!
//! Runtime manager for CNS monitoring, algedonic alerts, and variety tracking.
//! Provides health status and alert querying for CLI and API integration.

use crate::algedonic::{AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD};
use crate::variety::{VarietyCounter, VarietyMonitor};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// CNS runtime manager
pub struct CnsRuntime {
    /// Algedonic alert manager
    algedonic: Arc<RwLock<AlgedonicManager>>,
    /// Variety monitor
    variety: Arc<RwLock<VarietyMonitor>>,
}

impl CnsRuntime {
    /// Create new CNS runtime with default threshold
    pub fn new() -> Self {
        Self {
            algedonic: Arc::new(RwLock::new(AlgedonicManager::new(DEFAULT_THRESHOLD))),
            variety: Arc::new(RwLock::new(VarietyMonitor::new())),
        }
    }

    /// Create CNS runtime with custom threshold
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            algedonic: Arc::new(RwLock::new(AlgedonicManager::new(threshold))),
            variety: Arc::new(RwLock::new(VarietyMonitor::new())),
        }
    }

    /// Get CNS health status
    pub async fn health(&self) -> CnsHealth {
        let algedonic = self.algedonic.read().await;
        CnsHealth::check(&algedonic)
    }

    /// Get all algedonic alerts
    pub async fn alerts(&self) -> Vec<crate::algedonic::AlgedonicAlert> {
        let algedonic = self.algedonic.read().await;
        algedonic.alerts().to_vec()
    }

    /// Get critical alerts only
    pub async fn critical_alerts(&self) -> Vec<crate::algedonic::AlgedonicAlert> {
        let algedonic = self.algedonic.read().await;
        algedonic.critical_alerts().into_iter().cloned().collect()
    }

    /// Get variety counters for all domains
    pub async fn variety(&self) -> Vec<(String, u64)> {
        let variety = self.variety.read().await;
        let domains: Vec<String> = variety.domains().iter().map(|s| s.to_string()).collect();
        drop(variety);

        let mut results = Vec::new();
        for domain in &domains {
            let variety = self.variety.read().await;
            let count = variety.variety_for_domain(domain);
            results.push((domain.clone(), count));
        }
        results
    }

    /// Get variety counter for specific domain
    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        let variety = self.variety.read().await;
        variety.variety_for_domain(domain)
    }

    /// Increment variety counter for domain
    pub async fn increment_variety(&self, domain: &str, state: &str) {
        let mut variety = self.variety.write().await;
        variety.counter(domain).increment(state);
        info!(target: "cns.variety", domain = %domain, state = %state, "Variety incremented");
    }

    /// Check variety and generate algedonic alert if needed
    pub async fn check_variety(&self, domain: &str) -> Option<crate::algedonic::AlgedonicAlert> {
        let counter = {
            let variety = self.variety.read().await;
            variety
                .counters
                .get(domain)
                .cloned()
                .unwrap_or_else(VarietyCounter::new)
        };

        let mut algedonic = self.algedonic.write().await;
        algedonic.check(&counter, domain).cloned()
    }

    /// Check all domains and return count of alerts generated
    pub async fn check_all(&self) -> usize {
        let mut variety = self.variety.write().await;
        let mut algedonic = self.algedonic.write().await;
        algedonic.check_all(&mut variety)
    }

    /// Reset all alerts
    pub async fn reset_alerts(&self) {
        let mut algedonic = self.algedonic.write().await;
        algedonic.reset();
    }

    /// Clear old alerts (older than specified duration)
    pub async fn clear_old_alerts(&self, max_age: std::time::Duration) {
        let mut algedonic = self.algedonic.write().await;
        algedonic.clear_old(max_age);
    }

    /// Get total variety deficit across all domains
    pub async fn total_deficit(&self) -> u64 {
        let variety = self.variety.read().await;
        variety.total_deficit(DEFAULT_THRESHOLD)
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
        assert_eq!(variety, 2); // Two distinct states

        let all_variety = runtime.variety().await;
        assert_eq!(all_variety.len(), 1);
        assert_eq!(all_variety[0].1, 2);
    }

    #[tokio::test]
    async fn test_cns_runtime_check_variety() {
        let runtime = CnsRuntime::with_threshold(1); // Low threshold for testing

        runtime.increment_variety("test", "state_a").await;
        runtime.increment_variety("test", "state_b").await;

        // Variety of 2 should exceed threshold of 1
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
        // Each domain has variety 1, expected DEFAULT_THRESHOLD
        // Deficit per domain = DEFAULT_THRESHOLD - 1
        assert!(deficit > 0);
    }
}
