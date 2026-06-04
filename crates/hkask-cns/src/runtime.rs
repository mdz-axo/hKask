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
use hkask_types::WebID;
use hkask_types::cns::CnsHealth;
use hkask_types::event::SpanNamespace;
use hkask_types::ports::{BackpressureSignal, CnsObserver, DepletionSignal};
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

/// CNS runtime — single entry point for observability and regulation
pub struct CnsRuntime {
    state: Arc<RwLock<CnsState>>,
    subscribers: Arc<RwLock<Vec<Arc<dyn CnsObserver>>>>,
}

impl CnsRuntime {
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CnsState::new(threshold))),
            subscribers: Arc::new(RwLock::new(Vec::new())),
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
    /// After persisting variety, notifies subscribers whose interest mask
    /// includes the relevant span namespace.
    pub async fn increment_variety(&self, domain: &str, state_name: &str) {
        {
            let mut state = self.state.write().await;
            state.tracker.increment_variety(domain, state_name);
        }
        self.check_variety(domain).await;

        // Notify subscribers interested in this domain's span namespace
        if let Some(span_ns) = SpanNamespace::parse(domain) {
            let subscribers = self.subscribers.read().await;
            for observer in subscribers.iter() {
                if observer.interest_mask().iter().any(|ns| ns == &span_ns) {
                    // on_event requires a NuEvent; we create a minimal one
                    // from the domain/state change. The observer decides what
                    // to do with it.
                    let event = hkask_types::event::NuEvent::new(
                        WebID::default(),
                        hkask_types::event::Span::new(span_ns.clone(), "variety_incremented"),
                        hkask_types::event::Phase::Act,
                        serde_json::json!({"domain": domain, "state": state_name}),
                        0,
                    );
                    observer.on_event(&event).await;
                }
            }
        }
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
        let alert = match Self::write_algedonic(&state.algedonic) {
            Ok(mut mgr) => mgr.check(&counter, domain).cloned(),
            Err(e) => {
                warn!("CNS lock poisoned during variety check: {}", e);
                None
            }
        };
        drop(state);

        // If alert is critical, emit depletion signals to subscribers
        if let Some(ref alert) = alert
            && alert.severity == crate::algedonic::AlertSeverity::Critical
        {
            let subscribers = self.subscribers.read().await;
            let signal = DepletionSignal {
                agent: WebID::default(),
                remaining: alert.threshold.saturating_sub(alert.deficit),
                cap: alert.threshold,
                usage_ratio: if alert.threshold > 0 {
                    alert.deficit as f64 / alert.threshold as f64
                } else {
                    1.0
                },
            };
            for observer in subscribers.iter() {
                observer.on_depletion(&signal).await;
            }
        }

        alert
    }

    pub async fn calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        let state = self.state.write().await;
        if let Ok(mut algedonic) = Self::write_algedonic(&state.algedonic) {
            algedonic.set_expected_variety(domain, new_threshold);
        }
    }

    // ── Bot Observation (CNS Observer) ──

    /// Register a CnsObserver to receive events matching its interest mask.
    ///
    /// Observers are notified asynchronously when:
    /// - A variety increment matches their interest mask (on_event)
    /// - A depletion signal fires for their agent (on_depletion)
    /// - A backpressure signal fires (on_backpressure)
    ///
    /// Use `subscribe_async` when calling from an async context.
    pub fn subscribe(&self, observer: Arc<dyn CnsObserver>) {
        let mut subscribers = self.subscribers.blocking_write();
        subscribers.push(observer);
    }

    /// Register a CnsObserver to receive events matching its interest mask.
    ///
    /// This is the async version of subscribe, preferred when called from
    /// an async context (e.g., during bootstrap or from the API).
    pub async fn subscribe_async(&self, observer: Arc<dyn CnsObserver>) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(observer);
    }

    /// Emit a backpressure signal to all subscribers.
    pub async fn emit_backpressure(&self, signal: BackpressureSignal) {
        let subscribers = self.subscribers.read().await;
        for observer in subscribers.iter() {
            observer.on_backpressure(&signal).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::SpanNamespace;
    use hkask_types::loops::LoopId;
    use hkask_types::ports::{BackpressureSignal, CnsObserver, DepletionSignal};
    use std::sync::{Arc, Mutex};

    /// Test observer that records events, depletion, and backpressure signals.
    struct TestObserver {
        events: Mutex<Vec<String>>,
        depletions: Mutex<Vec<DepletionSignal>>,
        backpressures: Mutex<Vec<BackpressureSignal>>,
        mask: Vec<SpanNamespace>,
    }

    impl TestObserver {
        fn new(mask: Vec<SpanNamespace>) -> Self {
            Self {
                events: Mutex::new(Vec::new()),
                depletions: Mutex::new(Vec::new()),
                backpressures: Mutex::new(Vec::new()),
                mask,
            }
        }
    }

    #[async_trait::async_trait]
    impl CnsObserver for TestObserver {
        fn interest_mask(&self) -> Vec<SpanNamespace> {
            self.mask.clone()
        }

        async fn on_event(&self, event: &hkask_types::event::NuEvent) {
            self.events.lock().unwrap().push(event.span.path.clone());
        }

        async fn on_depletion(&self, signal: &DepletionSignal) {
            self.depletions.lock().unwrap().push(signal.clone());
        }

        async fn on_backpressure(&self, signal: &BackpressureSignal) {
            self.backpressures.lock().unwrap().push(signal.clone());
        }
    }

    #[tokio::test]
    async fn cns_runtime_delivers_events_to_observer() {
        let runtime = CnsRuntime::default();
        let observer = Arc::new(TestObserver::new(vec![SpanNamespace::new("cns.variety")]));
        runtime
            .subscribe_async(observer.clone() as Arc<dyn CnsObserver>)
            .await;

        // "variety" maps to "cns.variety" via SpanNamespace::from_str — observer should be notified
        runtime.increment_variety("variety", "test_state").await;

        let events = observer.events.lock().unwrap();
        assert!(
            !events.is_empty(),
            "Observer should have received at least one event"
        );
    }

    #[tokio::test]
    async fn cns_runtime_delivers_backpressure_to_observer() {
        let runtime = CnsRuntime::default();
        let observer = Arc::new(TestObserver::new(vec![SpanNamespace::new("cns.variety")]));
        runtime
            .subscribe_async(observer.clone() as Arc<dyn CnsObserver>)
            .await;

        let signal = BackpressureSignal {
            source: LoopId::Cybernetics,
            reason: "test backpressure".to_string(),
            severity: 0.8,
        };
        runtime.emit_backpressure(signal).await;

        let backpressures = observer.backpressures.lock().unwrap();
        assert_eq!(backpressures.len(), 1);
        assert_eq!(backpressures[0].reason, "test backpressure");
    }
}
