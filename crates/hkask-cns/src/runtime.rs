//! CNS Runtime — minimal observability
//!
//! CnsRuntime is the single entry point for all CNS operations:
//! - Variety counting (Ashby's Law)
//! - Algedonic alerts (deficit > threshold → escalate)
//! - Bot registration and energy budgets
//! - Sovereignty event tracking

use crate::algedonic::{
    AlgedonicManager, CnsHealth, DEFAULT_EXPECTED_VARIETY, DEFAULT_THRESHOLD, RuntimeAlert,
};
use crate::observers::sovereignty::SovereigntyEvent;
use crate::unified_tracker::UnifiedVarietyTracker;
use crate::variety::VarietyTracker;
use hkask_types::{InfrastructureError, WebID};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::warn;

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
    tracker: UnifiedVarietyTracker,
    subscribers: std::sync::Mutex<AlertSubscriberList>,
    next_subscriber_id: std::sync::atomic::AtomicU64,
}

impl CnsState {
    fn new(threshold: u64) -> Self {
        let algedonic = Arc::new(StdRwLock::new(AlgedonicManager::new(
            threshold,
            DEFAULT_EXPECTED_VARIETY,
        )));
        let tracker = UnifiedVarietyTracker::new(algedonic.clone());
        Self {
            algedonic,
            tracker,
            subscribers: std::sync::Mutex::new(Vec::new()),
            next_subscriber_id: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

/// CNS runtime — single entry point for observability
pub struct CnsRuntime {
    state: Arc<RwLock<CnsState>>,
}

impl CnsRuntime {
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CnsState::new(threshold))),
        }
    }

    fn read_algedonic(
        algedonic: &Arc<StdRwLock<AlgedonicManager>>,
    ) -> CnsResult<std::sync::RwLockReadGuard<'_, AlgedonicManager>> {
        algedonic
            .read()
            .map_err(|_| InfrastructureError::LockPoisoned.into())
    }

    fn write_algedonic(
        algedonic: &Arc<StdRwLock<AlgedonicManager>>,
    ) -> CnsResult<std::sync::RwLockWriteGuard<'_, AlgedonicManager>> {
        algedonic
            .write()
            .map_err(|_| InfrastructureError::LockPoisoned.into())
    }

    // ── Health & Alerts ──

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

    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        match Self::read_algedonic(&state.algedonic) {
            Ok(mgr) => mgr.alerts().to_vec(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn critical_alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        match Self::read_algedonic(&state.algedonic) {
            Ok(mgr) => mgr.critical_alerts().into_iter().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    // ── Variety ──

    pub async fn variety(&self) -> Vec<(String, u64)> {
        let state = self.state.read().await;
        let domains: Vec<String> = state
            .tracker
            .variety_domains()
            .iter()
            .map(|s| s.to_string())
            .collect();
        drop(state);

        let mut results = Vec::new();
        for domain in &domains {
            let state = self.state.read().await;
            let count = state.tracker.variety_for_domain(domain);
            drop(state);
            results.push((domain.clone(), count));
        }
        results
    }

    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        let state = self.state.read().await;
        state.tracker.variety_for_domain(domain)
    }

    /// Increment variety and check thresholds — the loop closes here.
    pub async fn increment_variety(&self, domain: &str, state_name: &str) {
        {
            let mut state = self.state.write().await;
            state.tracker.increment_variety(domain, state_name);
        }
        self.check_variety(domain).await;
    }

    pub async fn check_variety(&self, domain: &str) -> Option<RuntimeAlert> {
        let counter = {
            let state = self.state.read().await;
            state
                .tracker
                .variety_monitor()
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

    pub async fn check_all(&self) -> usize {
        let domains = {
            let state = self.state.read().await;
            state
                .tracker
                .variety_domains()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        };

        let mut count = 0;
        for domain in domains {
            let counter = {
                let state = self.state.read().await;
                state
                    .tracker
                    .variety_monitor()
                    .counters()
                    .get(&domain)
                    .cloned()
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

    pub async fn calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        let state = self.state.write().await;
        if let Ok(mut algedonic) = Self::write_algedonic(&state.algedonic) {
            algedonic.set_expected_variety(domain, new_threshold);
        }
    }

    pub async fn reset_alerts(&self) {
        let state = self.state.write().await;
        match Self::write_algedonic(&state.algedonic) {
            Ok(mut mgr) => mgr.reset(),
            Err(e) => warn!("CNS lock poisoned during reset: {}", e),
        }
    }

    pub async fn clear_old_alerts(&self, max_age: std::time::Duration) {
        let state = self.state.write().await;
        match Self::write_algedonic(&state.algedonic) {
            Ok(mut mgr) => mgr.clear_old(max_age),
            Err(e) => warn!("CNS lock poisoned during clear_old: {}", e),
        }
    }

    pub async fn total_deficit(&self) -> u64 {
        let state = self.state.read().await;
        state.tracker.total_variety_deficit(DEFAULT_THRESHOLD)
    }

    // ── Bot Metrics ──

    pub async fn register_bot(&self, bot_id: WebID, bot_name: String) {
        let mut state = self.state.write().await;
        state.tracker.register_bot(bot_id, bot_name);
    }

    pub async fn set_bot_energy_budget(&self, bot_id: &WebID, budget: u64) {
        let mut state = self.state.write().await;
        state.tracker.set_bot_energy_budget(bot_id, budget);
    }

    // ── Sovereignty ──

    pub async fn process_sovereignty_event(&self, event: SovereigntyEvent) {
        let mut state = self.state.write().await;
        state.tracker.process_sovereignty_event(event);
    }

    pub async fn sovereignty_state(
        &self,
    ) -> crate::observers::sovereignty::SovereigntyObserverState {
        let state = self.state.read().await;
        state.tracker.sovereignty_state().clone()
    }

    // ── Subscribers ──

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

/// Opaque handle — drop to unsubscribe.
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
        Self::with_threshold(DEFAULT_THRESHOLD)
    }
}
