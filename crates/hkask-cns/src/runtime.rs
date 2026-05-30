//! CNS Runtime Integration
//!
//! Runtime manager for CNS monitoring, algedonic alerts, and variety tracking.
//! Provides health status and alert querying for CLI and API integration.
//!
//! Uses shared state with RwLock for compatibility with sync and async contexts.
//! All lock operations return `Result` — CNS must not panic (CNS monitors panics).

use crate::algedonic::{
    AlgedonicManager, CnsHealth, DEFAULT_EXPECTED_VARIETY, DEFAULT_THRESHOLD, RuntimeAlert,
};
use crate::observers::sovereignty::SovereigntyObserver;
use crate::variety::{VarietyMonitor, VarietyTracker};
use hkask_types::InfrastructureError;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// A subscriber to algedonic alerts — an opaque callback.
pub type AlertSubscriber = Arc<dyn Fn(&RuntimeAlert) + Send + Sync>;

/// The list of alert subscribers, keyed by unique ID.
type AlertSubscriberList = Vec<(u64, AlertSubscriber)>;

/// CNS runtime errors
#[derive(Debug, Error)]
pub enum CnsError {
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
}

/// Result alias for CNS operations
pub type CnsResult<T> = Result<T, CnsError>;

/// CNS state shared between threads
struct CnsState {
    algedonic: Arc<StdRwLock<AlgedonicManager>>,
    variety: VarietyMonitor,
    sovereignty_observer: SovereigntyObserver,
    /// Subscribers for headless escalation delivery.
    /// Each subscriber is a callback invoked on Critical alerts.
    /// This is how "Escalate to Human" works in a headless system —
    /// the human connects via MCP/CLI and registers a subscriber,
    /// and the CnsRuntime delivers the alert to their interface.
    ///
    /// Uses std::sync::Mutex (not tokio) so subscribers can be
    /// unregistered from any thread, including Drop implementations.
    subscribers: std::sync::Mutex<AlertSubscriberList>,
    next_subscriber_id: std::sync::atomic::AtomicU64,
}

impl CnsState {
    fn new(threshold: u64) -> Self {
        let algedonic = Arc::new(StdRwLock::new(AlgedonicManager::new(
            threshold,
            DEFAULT_EXPECTED_VARIETY,
        )));
        let sovereignty_observer = SovereigntyObserver::with_manager(algedonic.clone());
        Self {
            algedonic,
            variety: VarietyMonitor::new(),
            sovereignty_observer,
            subscribers: std::sync::Mutex::new(Vec::new()),
            next_subscriber_id: std::sync::atomic::AtomicU64::new(1),
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

    /// Read-lock the algedonic manager, propagating poison errors
    fn read_algedonic(
        algedonic: &Arc<StdRwLock<AlgedonicManager>>,
    ) -> CnsResult<std::sync::RwLockReadGuard<'_, AlgedonicManager>> {
        algedonic
            .read()
            .map_err(|_| InfrastructureError::LockPoisoned.into())
    }

    /// Write-lock the algedonic manager, propagating poison errors
    fn write_algedonic(
        algedonic: &Arc<StdRwLock<AlgedonicManager>>,
    ) -> CnsResult<std::sync::RwLockWriteGuard<'_, AlgedonicManager>> {
        algedonic
            .write()
            .map_err(|_| InfrastructureError::LockPoisoned.into())
    }

    /// Get CNS health status
    pub async fn health(&self) -> CnsHealth {
        let state = self.state.read().await;
        match Self::read_algedonic(&state.algedonic) {
            Ok(mgr) => CnsHealth::check(&mgr),
            Err(e) => {
                warn!("CNS lock poisoned during health check: {}", e);
                CnsHealth {
                    overall_deficit: 0,
                    critical_count: 1,
                    warning_count: 0,
                    healthy: false,
                }
            }
        }
    }

    /// Get all algedonic alerts
    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        match Self::read_algedonic(&state.algedonic) {
            Ok(mgr) => mgr.alerts().to_vec(),
            Err(_) => Vec::new(),
        }
    }

    /// Get critical alerts only
    pub async fn critical_alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        match Self::read_algedonic(&state.algedonic) {
            Ok(mgr) => mgr.critical_alerts().into_iter().cloned().collect(),
            Err(_) => Vec::new(),
        }
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

        let alert = {
            let state = self.state.write().await;
            match Self::write_algedonic(&state.algedonic) {
                Ok(mut mgr) => mgr.check(&counter, domain).cloned(),
                Err(e) => {
                    warn!("CNS lock poisoned during variety check: {}", e);
                    None
                }
            }
        };

        // Headless escalation: if a Critical alert was produced,
        // deliver it to all registered subscribers immediately.
        // Uses std::sync::Mutex so the lock is held for a minimal
        // duration (no await point inside the lock scope).
        if let Some(ref alert) = alert
            && alert.is_critical()
        {
            let mut subs: Vec<AlertSubscriber> = Vec::new();
            {
                let state = self.state.read().await;
                if let Ok(locked) = state.subscribers.lock() {
                    for (_, f) in locked.iter() {
                        subs.push(f.clone());
                    }
                }
            }
            for subscriber in &subs {
                subscriber(alert);
            }
        }

        alert
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
                let state = self.state.write().await;
                match Self::write_algedonic(&state.algedonic) {
                    Ok(mut mgr) => {
                        if mgr.check(&counter, &domain).is_some() {
                            count += 1;
                        }
                    }
                    Err(e) => {
                        warn!("CNS lock poisoned during check_all: {}", e);
                    }
                }
            }
        }
        count
    }

    /// Reset all alerts
    pub async fn reset_alerts(&self) {
        let state = self.state.write().await;
        match Self::write_algedonic(&state.algedonic) {
            Ok(mut mgr) => mgr.reset(),
            Err(e) => warn!("CNS lock poisoned during reset: {}", e),
        }
    }

    /// Clear old alerts (older than specified duration)
    pub async fn clear_old_alerts(&self, max_age: std::time::Duration) {
        let state = self.state.write().await;
        match Self::write_algedonic(&state.algedonic) {
            Ok(mut mgr) => mgr.clear_old(max_age),
            Err(e) => warn!("CNS lock poisoned during clear_old: {}", e),
        }
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

    /// Subscribe to algedonic alert delivery.
    ///
    /// The subscriber is invoked on every Critical alert produced by
    /// [`check_variety`]. This is the headless equivalent of
    /// "Escalate to Human" — the human connects via MCP/CLI,
    /// registers a subscriber, and receives push notifications.
    ///
    /// Returns an opaque subscription handle. Drop it to unsubscribe.
    pub async fn subscribe(
        &self,
        f: impl Fn(&RuntimeAlert) + Send + Sync + 'static,
    ) -> AlertSubscription {
        let id = {
            let state = self.state.read().await;
            state
                .next_subscriber_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        };
        let arc: Arc<dyn Fn(&RuntimeAlert) + Send + Sync> = Arc::new(f);
        {
            let state = self.state.read().await;
            state
                .subscribers
                .lock()
                .expect("subscriber lock should not be poisoned")
                .push((id, arc.clone()));
        }
        AlertSubscription {
            id,
            state: self.state.clone(),
        }
    }
}

/// Opaque handle returned by [`CnsRuntime::subscribe`].
/// Dropping this handle unregisters the subscriber.
pub struct AlertSubscription {
    id: u64,
    state: Arc<RwLock<CnsState>>,
}

impl Drop for AlertSubscription {
    fn drop(&mut self) {
        if let Ok(state) = self.state.try_read()
            && let Ok(mut subs) = state.subscribers.lock()
        {
            subs.retain(|(sid, _)| *sid != self.id);
        }
    }
}

impl Default for CnsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
