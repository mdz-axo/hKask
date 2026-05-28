//! CNS Runtime Integration
//!
//! Runtime manager for CNS monitoring, algedonic alerts, and variety tracking.
//! Provides health status and alert querying for CLI and API integration.
//!
//! Uses shared state with RwLock for compatibility with sync and async contexts.

use crate::algedonic::{AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD, RuntimeAlert};
use crate::observers::sovereignty::SovereigntyObserver;
use crate::variety::{VarietyMonitor, VarietyTracker};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock;
use tracing::info;

/// CNS state shared between threads
struct CnsState {
    algedonic: Arc<StdRwLock<AlgedonicManager>>,
    variety: VarietyMonitor,
    sovereignty_observer: SovereigntyObserver,
}

impl CnsState {
    fn new(threshold: u64) -> Self {
        let algedonic = Arc::new(StdRwLock::new(AlgedonicManager::new(threshold)));
        let sovereignty_observer = SovereigntyObserver::with_manager(algedonic.clone());
        Self {
            algedonic,
            variety: VarietyMonitor::new(),
            sovereignty_observer,
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
        CnsHealth::check(&state.algedonic.read().unwrap())
    }

    /// Get all algedonic alerts
    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        state.algedonic.read().unwrap().alerts().to_vec()
    }

    /// Get critical alerts only
    pub async fn critical_alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        state
            .algedonic
            .read()
            .unwrap()
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
    pub async fn check_variety(&self, domain: &str) -> Option<RuntimeAlert> {
        let counter = {
            let state = self.state.read().await;
            state
                .variety
                .counters()
                .get(domain)
                .cloned()
                .unwrap_or_else(VarietyTracker::new)
        };

        let mut state = self.state.write().await;
        state
            .algedonic
            .write()
            .unwrap()
            .check(&counter, domain)
            .cloned()
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
                state.variety.counters().get(&domain).cloned()
            };

            if let Some(counter) = counter {
                let mut state = self.state.write().await;
                if state
                    .algedonic
                    .write()
                    .unwrap()
                    .check(&counter, &domain)
                    .is_some()
                {
                    count += 1;
                }
            }
        }
        count
    }

    /// Reset all alerts
    pub async fn reset_alerts(&self) {
        let mut state = self.state.write().await;
        state.algedonic.write().unwrap().reset();
    }

    /// Clear old alerts (older than specified duration)
    pub async fn clear_old_alerts(&self, max_age: std::time::Duration) {
        let mut state = self.state.write().await;
        state.algedonic.write().unwrap().clear_old(max_age);
    }

    /// Get total variety deficit across all domains
    pub async fn total_deficit(&self) -> u64 {
        let state = self.state.read().await;
        state.variety.total_deficit(DEFAULT_THRESHOLD)
    }

    /// Process a sovereignty event through the SovereigntyObserver
    pub async fn process_sovereignty_event(
        &self,
        event: crate::observers::sovereignty::SovereigntyEvent,
    ) {
        let state = self.state.read().await;
        state.sovereignty_observer.process_event(event);
    }

    /// Get current sovereignty observer state
    pub async fn sovereignty_state(
        &self,
    ) -> crate::observers::sovereignty::SovereigntyObserverState {
        let state = self.state.read().await;
        state.sovereignty_observer.get_state()
    }
}

impl Default for CnsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
