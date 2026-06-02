//! Unified variety tracker — single SENSE point for all CNS observation domains
//!
//! Consolidates domain variety (4.1), bot metrics (4.3), sovereignty events (4.4),
//! and goal variety into a single variety accounting structure. All SENSE subloops
//! feed into this one tracker, ensuring consistent variety accounting per Ashby's Law.
//!
//! # Design Rationale
//!
//! The previous design used independent variety-tracking structures:
//! - `VarietyMonitor` (Loop 4.1 — domain-based variety)
//! - `SovereigntyObserver` (Loop 4.4 — sovereignty event variety) — removed in Phase 11a
//! - `GoalVarietyMonitor` (Loop 4.4 — goal variety) — removed in Phase 11a
//! - `BotMetricsCollector` (Loop 4.3 — bot health variety) — removed in Phase 11b
//!
//! Each tracked variety independently, which meant:
//! 1. Inconsistent variety accounting (different windows, different reset policies)
//! 2. Duplicate state management (multiple HashMaps where one suffices)
//! 3. Complex CnsState (multiple fields where one suffices)
//!
//! The unified tracker uses domain-prefixed keys so all variety counting
//! goes through a single `VarietyMonitor`, while preserving the domain-specific
//! methods each subloop needs.

use crate::algedonic::AlgedonicManager;
use crate::observers::sovereignty::{
    SovereigntyEvent, SovereigntyEventType, SovereigntyObserverState,
};
use crate::variety::VarietyMonitor;
use hkask_types::WebID;
use std::sync::{Arc, RwLock};
use tracing::{error, warn};

/// Domain prefixes for unified variety counting.
///
/// All variety counters use domain-prefixed keys to avoid collisions
/// between different observation domains while sharing a single tracker.
pub mod domains {
    /// Sovereignty acquisition attempts: `sovereignty:acq:{webid}`
    pub(crate) const SOVEREIGNTY_ACQ: &str = "sovereignty:acq";
    /// Sovereignty kill zone alerts: `sovereignty:kz:{webid}`
    pub(crate) const SOVEREIGNTY_KZ: &str = "sovereignty:kz";
    /// Sovereignty boundary violations: `sovereignty:bv:{webid}`
    pub(crate) const SOVEREIGNTY_BV: &str = "sovereignty:bv";
    /// Bot variety tracking: `bot:{webid}:{category}`
    pub const BOT: &str = "bot";
}

/// Acquisition attempt threshold before algedonic alert
const DEFAULT_ACQUISITION_THRESHOLD: u64 = 5;
/// Boundary violation threshold before algedonic alert
const DEFAULT_VIOLATION_THRESHOLD: u64 = 3;

/// Unified variety tracker for all CNS observation domains.
///
/// A single structure that tracks variety across all SENSE subloops:
/// - Loop 4.1: Domain-based variety (inference, memory, governance, etc.)
/// - Loop 4.3: Bot health metrics (per-WebID evaluation)
/// - Loop 4.4: Sovereignty events (acquisition, violations, kill zone)
/// - Goal variety (per-WebID goal counting)
///
/// All variety counting goes through a single `VarietyMonitor`, ensuring
/// consistent windowing and reset behavior. The algedonic manager is shared
/// so all domains can trigger alerts through the same escalation path.
pub struct UnifiedVarietyTracker {
    /// Single variety monitor for all domains
    variety: VarietyMonitor,
    /// Shared algedonic manager for alert escalation
    algedonic: Arc<RwLock<AlgedonicManager>>,
    /// Sovereignty event state (per-WebID counters)
    sovereignty_state: SovereigntyObserverState,
    /// Acquisition attempt threshold for algedonic alert
    acquisition_threshold: u64,
    /// Boundary violation threshold for algedonic alert
    violation_threshold: u64,
}

impl UnifiedVarietyTracker {
    /// Create a new unified tracker with default thresholds.
    pub fn new(algedonic: Arc<RwLock<AlgedonicManager>>) -> Self {
        Self {
            variety: VarietyMonitor::new(),
            algedonic,
            sovereignty_state: SovereigntyObserverState::default(),
            acquisition_threshold: DEFAULT_ACQUISITION_THRESHOLD,
            violation_threshold: DEFAULT_VIOLATION_THRESHOLD,
        }
    }

    // =========================================================================
    // Loop 4.1 — Domain-based variety (Ashby's Law)
    // =========================================================================

    /// Increment variety counter for a domain.
    pub fn increment_variety(&mut self, domain: &str, state_name: &str) {
        self.variety.counter(domain).increment(state_name);
    }

    /// Get variety count for a specific domain.
    pub fn variety_for_domain(&self, domain: &str) -> u64 {
        self.variety.variety_for_domain(domain)
    }

    /// Get all domain names with variety counters.
    pub fn variety_domains(&self) -> Vec<&str> {
        self.variety.domains()
    }

    /// Get total variety deficit across all domains.
    pub fn total_variety_deficit(&self, expected_per_domain: u64) -> u64 {
        self.variety.total_deficit(expected_per_domain)
    }

    /// Get a reference to the underlying variety monitor.
    pub fn variety_monitor(&self) -> &VarietyMonitor {
        &self.variety
    }

    // =========================================================================
    // Loop 4.4 — Sovereignty observation
    // =========================================================================

    /// Process a sovereignty event.
    ///
    /// Updates sovereignty state counters and triggers algedonic alerts
    /// when thresholds are exceeded. This replaces the need for a separate
    /// `SovereigntyObserver` struct.
    pub fn process_sovereignty_event(&mut self, event: SovereigntyEvent) {
        self.sovereignty_state.total_events += 1;

        match event.event_type {
            SovereigntyEventType::AcquisitionAttempt => {
                *self
                    .sovereignty_state
                    .acquisition_attempts
                    .entry(event.webid)
                    .or_insert(0) += 1;
                let count = self.sovereignty_state.acquisition_attempts[&event.webid];

                // Also track as variety
                self.variety
                    .counter(&format!("{}:{}", domains::SOVEREIGNTY_ACQ, event.webid))
                    .increment("attempt");

                if count >= self.acquisition_threshold {
                    self.trigger_algedonic_alert(
                        &event.webid,
                        domains::SOVEREIGNTY_ACQ,
                        count,
                        &format!(
                            "WebID {} exceeded acquisition threshold ({} attempts)",
                            event.webid, count
                        ),
                    );
                }
            }
            SovereigntyEventType::KillZoneAlert => {
                *self
                    .sovereignty_state
                    .kill_zone_alerts
                    .entry(event.webid)
                    .or_insert(0) += 1;
                let count = self.sovereignty_state.kill_zone_alerts[&event.webid];

                // Also track as variety
                self.variety
                    .counter(&format!("{}:{}", domains::SOVEREIGNTY_KZ, event.webid))
                    .increment("alert");

                // Kill zone alerts immediately trigger algedonic escalation
                self.trigger_algedonic_alert(
                    &event.webid,
                    domains::SOVEREIGNTY_KZ,
                    count,
                    &format!(
                        "WebID {} entered kill zone (VC investment < threshold)",
                        event.webid
                    ),
                );
            }
            SovereigntyEventType::BoundaryViolation => {
                *self
                    .sovereignty_state
                    .boundary_violations
                    .entry(event.webid)
                    .or_insert(0) += 1;
                let count = self.sovereignty_state.boundary_violations[&event.webid];

                // Also track as variety
                self.variety
                    .counter(&format!("{}:{}", domains::SOVEREIGNTY_BV, event.webid))
                    .increment("violation");

                if count >= self.violation_threshold {
                    self.trigger_algedonic_alert(
                        &event.webid,
                        domains::SOVEREIGNTY_BV,
                        count,
                        &format!(
                            "WebID {} exceeded boundary violation threshold ({} violations)",
                            event.webid, count
                        ),
                    );
                }
            }
            SovereigntyEventType::ConsentGranted | SovereigntyEventType::ConsentRevoked => {
                warn!(
                    target: "cns.tracker.sovereignty",
                    webid = %event.webid,
                    event_type = ?event.event_type,
                    "Consent state changed"
                );
            }
        }
    }

    /// Get current sovereignty state.
    pub fn sovereignty_state(&self) -> &SovereigntyObserverState {
        &self.sovereignty_state
    }

    /// Trigger an algedonic alert through the shared algedonic manager.
    fn trigger_algedonic_alert(&self, webid: &WebID, domain: &str, deficit: u64, message: &str) {
        let mut manager = match self.algedonic.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!(target: "cns.tracker", error = %e, "AlgedonicManager lock poisoned during trigger_algedonic_alert");
                return;
            }
        };

        error!(
            target: "cns.tracker.sovereignty",
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
}
