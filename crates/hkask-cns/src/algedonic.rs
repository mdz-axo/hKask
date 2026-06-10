//! Algedonic alerts — Variety deficit escalation
//!
//! Implements algedonic (pain/pleasure) feedback for cybernetic control.
//! When variety deficit exceeds threshold, alerts are escalated to the Curator/human.
//!
//! Per architecture v0.22.0: Variety deficit >50 → Warning escalation to Curator;
//! deficit >100 → Critical escalation to human. Binary threshold only — the
//! allosteric MWC sigmoid was deleted (essentialist review: added zero
//! runtime-observable behavior; CurationConfidenceGate always created with
//! empty ports; binary threshold is the backward-compatible limit case).

use crate::runtime::VarietyTracker;
use chrono::{DateTime, Utc};
use hkask_types::cns::CnsHealth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, warn};

/// Default DateTime for serde deserialization
fn default_datetime() -> DateTime<Utc> {
    Utc::now()
}

/// Default algedonic alert threshold (variety deficit)
pub const DEFAULT_THRESHOLD: u64 = 100;

/// Default expected variety per domain
pub(crate) const DEFAULT_EXPECTED_VARIETY: u64 = 10;

/// Alert severity levels — simple binary threshold classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational - deficit detected but below threshold
    Info,
    /// Warning - deficit approaching threshold
    Warning,
    /// Critical - deficit exceeds threshold, escalation required
    Critical,
}

/// Algedonic alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeAlert {
    pub domain: String,
    pub deficit: u64,
    pub threshold: u64,
    pub severity: AlertSeverity,
    pub escalated: bool,
    #[serde(default = "default_datetime")]
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

impl RuntimeAlert {
    /// Create an alert using binary thresholds.
    pub fn new(domain: &str, deficit: u64, threshold: u64) -> Self {
        let severity = if deficit > threshold {
            AlertSeverity::Critical
        } else if deficit > threshold / 2 {
            AlertSeverity::Warning
        } else {
            AlertSeverity::Info
        };

        Self {
            domain: domain.to_string(),
            deficit,
            threshold,
            severity,
            escalated: severity == AlertSeverity::Critical,
            timestamp: Utc::now(),
            message: format!(
                "Variety deficit {} in domain '{}' (threshold: {})",
                deficit, domain, threshold
            ),
        }
    }

    pub fn should_escalate(&self) -> bool {
        self.escalated
    }

    pub fn is_critical(&self) -> bool {
        self.severity == AlertSeverity::Critical
    }

    pub fn is_warning(&self) -> bool {
        self.severity == AlertSeverity::Warning
    }
}

/// Algedonic alert manager
pub(crate) struct AlgedonicManager {
    threshold: u64,
    default_expected_variety: u64,
    expected_variety: HashMap<String, u64>,
    alerts: Vec<RuntimeAlert>,
}

impl AlgedonicManager {
    pub(crate) fn new(threshold: u64, default_expected_variety: u64) -> Self {
        Self {
            threshold,
            default_expected_variety,
            expected_variety: HashMap::new(),
            alerts: Vec::new(),
        }
    }

    /// Set expected variety for a specific domain
    pub(crate) fn set_expected_variety(&mut self, domain: &str, expected: u64) {
        self.expected_variety.insert(domain.to_string(), expected);
    }

    /// Check variety counter and generate alert using binary thresholds.
    pub(crate) fn check(
        &mut self,
        counter: &VarietyTracker,
        domain: &str,
    ) -> Option<&RuntimeAlert> {
        let expected = self
            .expected_variety
            .get(domain)
            .copied()
            .unwrap_or(self.default_expected_variety);
        let deficit = counter.deficit(expected);

        let alert = RuntimeAlert::new(domain, deficit, self.threshold);

        if alert.should_escalate() {
            error!(
                target: "cns.algedonic",
                domain = %alert.domain,
                deficit = alert.deficit,
                threshold = alert.threshold,
                "ALGEDONIC ALERT - Escalation required"
            );
        } else if alert.is_warning() {
            warn!(
                target: "cns.algedonic",
                domain = %alert.domain,
                deficit = alert.deficit,
                "Variety deficit approaching threshold"
            );
        }

        self.alerts.push(alert);
        self.alerts.last()
    }

    /// Get the configured default threshold
    pub(crate) fn default_threshold(&self) -> u64 {
        self.threshold
    }

    /// Get all alerts
    pub(crate) fn alerts(&self) -> &[RuntimeAlert] {
        &self.alerts
    }

    /// Get critical alerts only
    pub(crate) fn critical_alerts(&self) -> Vec<&RuntimeAlert> {
        self.alerts.iter().filter(|a| a.is_critical()).collect()
    }

    /// Get total deficit across all alerts
    pub(crate) fn total_deficit(&self) -> u64 {
        self.alerts.iter().map(|a| a.deficit).sum()
    }
}

/// Construct CnsHealth from the algedonic manager's current state.
pub(crate) fn cns_health_check(manager: &AlgedonicManager) -> CnsHealth {
    CnsHealth {
        overall_deficit: manager.total_deficit(),
        critical_count: manager.critical_alerts().len(),
        warning_count: manager.alerts().iter().filter(|a| a.is_warning()).count(),
        healthy: manager.critical_alerts().is_empty(),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::VarietyTracker;

    // REQ: svc-cns-algedonic-001 — binary_threshold_classifies_critical_and_warning
    //
    // TASK 1 cybernetic property: when deficit exceeds threshold, severity
    // must be Critical. When deficit > threshold/2 but ≤ threshold, severity
    // must be Warning. When deficit ≤ threshold/2, severity must be Info.
    #[test]
    fn binary_threshold_classifies_critical_and_warning() {
        let threshold = 100;

        // deficit = 150 → > threshold → Critical
        let critical = RuntimeAlert::new("test", 150, threshold);
        assert_eq!(critical.severity, AlertSeverity::Critical);
        assert!(critical.escalated);

        // deficit = 75 → > threshold/2 but ≤ threshold → Warning
        let warning = RuntimeAlert::new("test", 75, threshold);
        assert_eq!(warning.severity, AlertSeverity::Warning);
        assert!(!warning.escalated);

        // deficit = 25 → ≤ threshold/2 → Info
        let info = RuntimeAlert::new("test", 25, threshold);
        assert_eq!(info.severity, AlertSeverity::Info);
        assert!(!info.escalated);
    }

    // REQ: svc-cns-algedonic-005 — algedonic_manager_accumulates_alerts_across_domains
    //
    // TASK 1 cybernetic property: AlgedonicManager must track variety per domain
    // independently, so a deficit in one domain does not suppress alerts in another.
    #[test]
    fn algedonic_manager_accumulates_alerts_across_domains() {
        let mut mgr = AlgedonicManager::new(100, 10);

        // Domain A: low variety (5 distinct states, expected 10 → deficit 5)
        let mut tracker_a = VarietyTracker::new();
        for i in 0..5 {
            tracker_a.increment(&format!("state_{}", i));
        }

        // Domain B: very low variety (1 distinct state, expected 10 → deficit 9)
        let mut tracker_b = VarietyTracker::new();
        tracker_b.increment("only_state");

        mgr.check(&tracker_a, "domain_a");
        mgr.check(&tracker_b, "domain_b");

        // Both domains should have alerts
        assert!(
            !mgr.alerts().is_empty(),
            "Should accumulate alerts per domain"
        );
        // Domain B should be more severe (higher deficit)
        let total = mgr.total_deficit();
        assert!(total >= 5 + 9, "Total deficit should reflect both domains");
    }
}
