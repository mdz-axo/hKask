//! CNS Sovereignty Observer
//!
//! Monitors CNS events for sovereignty violations and triggers algedonic alerts.
//! Integrates sovereignty checking with CNS algedonic escalation.

use crate::algedonic::AlgedonicManager;
use hkask_types::{DataCategory, SovereigntyId, WebID};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{error, warn};

/// Sovereignty event types monitored by CNS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SovereigntyEventType {
    /// Acquisition attempt detected
    AcquisitionAttempt,
    /// Kill zone alert triggered
    KillZoneAlert,
    /// Consent granted
    ConsentGranted,
    /// Consent revoked
    ConsentRevoked,
    /// Sovereignty boundary violation
    BoundaryViolation,
}

/// Sovereignty event record
#[derive(Debug, Clone)]
pub struct SovereigntyEvent {
    pub event_type: SovereigntyEventType,
    pub timestamp: std::time::Instant,
    pub webid: WebID,
    pub sovereignty_id: SovereigntyId,
    pub data_category: Option<DataCategory>,
    pub details: Value,
}

/// Sovereignty observer state
#[derive(Debug, Default, Clone)]
pub struct SovereigntyObserverState {
    /// Count of acquisition attempts per WebID
    pub acquisition_attempts: HashMap<WebID, u64>,
    /// Count of kill zone alerts per WebID
    pub kill_zone_alerts: HashMap<WebID, u64>,
    /// Count of boundary violations per WebID
    pub boundary_violations: HashMap<WebID, u64>,
    /// Total sovereignty events processed
    pub total_events: u64,
}

/// CNS Sovereignty Observer
///
/// Monitors CNS spans for sovereignty-related events and triggers
/// algedonic alerts when violations exceed thresholds.
pub struct SovereigntyObserver {
    state: Arc<RwLock<SovereigntyObserverState>>,
    algedonic_manager: Arc<RwLock<AlgedonicManager>>,
    /// Threshold for acquisition attempts before alert
    pub acquisition_threshold: u64,
    /// Threshold for boundary violations before alert
    pub violation_threshold: u64,
}

impl SovereigntyObserver {
    /// Create new sovereignty observer with its own AlgedonicManager
    pub fn new(algedonic_manager: AlgedonicManager) -> Self {
        Self {
            state: Arc::new(RwLock::new(SovereigntyObserverState::default())),
            algedonic_manager: Arc::new(RwLock::new(algedonic_manager)),
            acquisition_threshold: 5,
            violation_threshold: 3,
        }
    }

    /// Create new sovereignty observer sharing an existing AlgedonicManager
    pub fn with_manager(algedonic_manager: Arc<RwLock<AlgedonicManager>>) -> Self {
        Self {
            state: Arc::new(RwLock::new(SovereigntyObserverState::default())),
            algedonic_manager,
            acquisition_threshold: 5,
            violation_threshold: 3,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(
        algedonic_manager: AlgedonicManager,
        acquisition_threshold: u64,
        violation_threshold: u64,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(SovereigntyObserverState::default())),
            algedonic_manager: Arc::new(RwLock::new(algedonic_manager)),
            acquisition_threshold,
            violation_threshold,
        }
    }

    /// Process a CNS sovereignty event
    ///
    /// # Arguments
    /// * `event` — Sovereignty event to process
    pub fn process_event(&self, event: SovereigntyEvent) {
        let mut state = self
            .state
            .write()
            .expect("SovereigntyObserver state lock poisoned");

        state.total_events += 1;

        match event.event_type {
            SovereigntyEventType::AcquisitionAttempt => {
                *state.acquisition_attempts.entry(event.webid).or_insert(0) += 1;
                let count = state.acquisition_attempts[&event.webid];

                if count >= self.acquisition_threshold {
                    self.trigger_algedonic_alert(
                        &event.webid,
                        "acquisition_pattern",
                        count,
                        &format!(
                            "WebID {} exceeded acquisition threshold ({} attempts)",
                            event.webid, count
                        ),
                    );
                }
            }
            SovereigntyEventType::KillZoneAlert => {
                *state.kill_zone_alerts.entry(event.webid).or_insert(0) += 1;
                let count = state.kill_zone_alerts[&event.webid];

                // Kill zone alerts immediately trigger algedonic escalation
                self.trigger_algedonic_alert(
                    &event.webid,
                    "killzone",
                    count,
                    &format!(
                        "WebID {} entered kill zone (VC investment < threshold)",
                        event.webid
                    ),
                );
            }
            SovereigntyEventType::BoundaryViolation => {
                *state.boundary_violations.entry(event.webid).or_insert(0) += 1;
                let count = state.boundary_violations[&event.webid];

                if count >= self.violation_threshold {
                    self.trigger_algedonic_alert(
                        &event.webid,
                        "boundary_violation",
                        count,
                        &format!(
                            "WebID {} exceeded boundary violation threshold ({} violations)",
                            event.webid, count
                        ),
                    );
                }
            }
            SovereigntyEventType::ConsentGranted | SovereigntyEventType::ConsentRevoked => {
                // Log consent changes but don't trigger alerts
                warn!(
                    target: "cns.observer.sovereignty",
                    webid = %event.webid,
                    event_type = ?event.event_type,
                    "Consent state changed"
                );
            }
        }
    }

    /// Trigger an algedonic alert
    fn trigger_algedonic_alert(&self, webid: &WebID, domain: &str, deficit: u64, message: &str) {
        let mut manager = self
            .algedonic_manager
            .write()
            .expect("AlgedonicManager lock poisoned");

        error!(
            target: "cns.observer.sovereignty",
            webid = %webid,
            domain = %domain,
            deficit = deficit,
            "ALGEDONIC ALERT - Sovereignty violation"
        );

        // Create variety counter for this domain
        let mut counter = crate::variety::VarietyTracker::new();
        for _ in 0..deficit {
            counter.increment(domain);
        }

        // Check and generate alert
        if let Some(alert) = manager.check(&counter, domain)
            && alert.should_escalate()
        {
            error!(
                target: "cns.algedonic",
                webid = %webid,
                message = %message,
                "Escalating sovereignty violation to Curator/human"
            );
        }
    }

    /// Get current observer state
    pub fn get_state(&self) -> SovereigntyObserverState {
        let state = self
            .state
            .read()
            .expect("SovereigntyObserver state lock poisoned");
        (*state).clone()
    }

    /// Get acquisition attempt count for a WebID
    pub fn get_acquisition_count(&self, webid: &WebID) -> u64 {
        self.state
            .read()
            .expect("SovereigntyObserver state lock poisoned")
            .acquisition_attempts
            .get(webid)
            .copied()
            .unwrap_or(0)
    }

    /// Get boundary violation count for a WebID
    pub fn get_violation_count(&self, webid: &WebID) -> u64 {
        self.state
            .read()
            .expect("SovereigntyObserver state lock poisoned")
            .boundary_violations
            .get(webid)
            .copied()
            .unwrap_or(0)
    }

    /// Reset observer state
    pub fn reset(&self) {
        let mut state = self
            .state
            .write()
            .expect("SovereigntyObserver state lock poisoned");
        *state = SovereigntyObserverState::default();
    }
}
