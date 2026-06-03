//! CNS Runtime — minimal observability
//!
//! CnsRuntime is the single entry point for all CNS operations:
//! - Variety counting (Ashby's Law)
//! - Algedonic alerts (deficit > threshold → escalate)

use crate::algedonic::{
    AlgedonicManager, DEFAULT_EXPECTED_VARIETY, DEFAULT_THRESHOLD, RuntimeAlert, cns_health_check,
};
use crate::kill_zone::KillZoneDetector;
use crate::unified_tracker::UnifiedVarietyTracker;
use crate::variety::VarietyTracker;
use hkask_types::InfrastructureError;
use hkask_types::cns::CnsHealth;
use hkask_types::sovereignty::KillZoneConfig;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock;
use tracing::warn;

/// CNS state shared between threads
struct CnsState {
    algedonic: Arc<StdRwLock<AlgedonicManager>>,
    tracker: UnifiedVarietyTracker,
    kill_zone: Arc<tokio::sync::Mutex<KillZoneDetector>>,
}

impl CnsState {
    fn new(threshold: u64) -> Self {
        let algedonic = Arc::new(StdRwLock::new(
            AlgedonicManager::new(threshold, DEFAULT_EXPECTED_VARIETY).with_default_allosteric(),
        ));
        let tracker = UnifiedVarietyTracker::new();
        let kill_zone = Arc::new(tokio::sync::Mutex::new(KillZoneDetector::new(
            hkask_types::sovereignty::KillZoneThresholds::default(),
        )));
        Self {
            algedonic,
            tracker,
            kill_zone,
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
    ) -> Result<std::sync::RwLockReadGuard<'_, AlgedonicManager>, InfrastructureError> {
        algedonic
            .read()
            .map_err(|_| InfrastructureError::LockPoisoned)
    }

    fn write_algedonic(
        algedonic: &Arc<StdRwLock<AlgedonicManager>>,
    ) -> Result<std::sync::RwLockWriteGuard<'_, AlgedonicManager>, InfrastructureError> {
        algedonic
            .write()
            .map_err(|_| InfrastructureError::LockPoisoned)
    }

    // ── Health & Alerts ──

    pub async fn health(&self) -> CnsHealth {
        let state = self.state.read().await;
        match Self::read_algedonic(&state.algedonic) {
            Ok(mgr) => cns_health_check(&mgr),
            Err(_) => CnsHealth {
                overall_deficit: 0,
                critical_count: 1,
                warning_count: 0,
                healthy: false,
            },
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

        let state = self.state.write().await;
        match Self::write_algedonic(&state.algedonic) {
            Ok(mut mgr) => mgr.check(&counter, domain).cloned(),
            Err(e) => {
                warn!("CNS lock poisoned during variety check: {}", e);
                None
            }
        }
    }

    pub async fn calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        let state = self.state.write().await;
        if let Ok(mut algedonic) = Self::write_algedonic(&state.algedonic) {
            algedonic.set_expected_variety(domain, new_threshold);
        }
    }

    // ── Kill Zone ──

    /// Get the current kill zone configuration/state.
    pub async fn kill_zone_state(&self) -> KillZoneConfig {
        let state = self.state.read().await;
        state.kill_zone.lock().await.state().clone()
    }

    /// Update VC investment and check if kill zone is triggered.
    ///
    /// Returns `true` if the kill zone alert should be fired.
    pub async fn check_kill_zone(&self, vc_investment: f32, acquisition_attempt: bool) -> bool {
        let state = self.state.read().await;
        let mut detector = state.kill_zone.lock().await;
        detector.update_vc_investment(vc_investment);
        if acquisition_attempt {
            detector.mark_acquisition_attempt();
        }
        detector.needs_alert()
    }
}

impl Default for CnsRuntime {
    fn default() -> Self {
        Self::with_threshold(DEFAULT_THRESHOLD)
    }
}

impl hkask_types::ports::CnsPort for CnsRuntime {
    async fn health(&self) -> CnsHealth {
        CnsRuntime::health(self).await
    }

    async fn variety(&self) -> Vec<(String, u64)> {
        CnsRuntime::variety(self).await
    }

    async fn increment_variety(&self, domain: &str, state_name: &str) {
        CnsRuntime::increment_variety(self, domain, state_name).await
    }
}
