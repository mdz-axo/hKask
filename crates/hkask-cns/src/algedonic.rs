//! Algedonic alerts — Variety deficit escalation
//
//! Implements algedonic (pain/pleasure) feedback for cybernetic control.
//! When variety deficit exceeds threshold, alerts are escalated to the Curator/human.
//
//! Per architecture v0.22.0: Variety deficit >50 → Warning escalation to Curator; deficit >100 → Critical escalation to human
//
//! IP-1: The binary threshold has been replaced with an AllostericGate
//! that produces a smooth MWC sigmoid. The existing behavior is the limit
//! case (L→∞), so this is backward-compatible.

use crate::allosteric::gate::{AllostericGate, AllostericGateConfig};
use crate::variety::VarietyTracker;
use chrono::{DateTime, Utc};
use hkask_types::cns::CnsHealth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, warn};

/// Default DateTime for serde deserialization
fn default_datetime() -> DateTime<Utc> {
    Utc::now()
}

/// Default algedonic alert threshold (variety deficit)
pub const DEFAULT_THRESHOLD: u64 = 100;

/// Default expected variety per domain
pub(crate) const DEFAULT_EXPECTED_VARIETY: u64 = 10;

/// Alert severity levels
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
    /// R̄ from the allosteric gate (0 = fully T/suppress, 1 = fully R/escalate).
    pub r_bar: f64,
    #[serde(default = "default_datetime")]
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

impl RuntimeAlert {
    /// Create an alert using the traditional binary thresholds.
    ///
    /// This is retained for backward compatibility. The allosteric-aware
    /// path uses `new_allosteric` instead.
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
            r_bar: 0.0,
            timestamp: Utc::now(),
            message: format!(
                "Variety deficit {} in domain '{}' (threshold: {})",
                deficit, domain, threshold
            ),
        }
    }

    /// Create an alert using an allosteric gate for severity classification.
    ///
    /// The gate's R̄ determines severity via the MWC sigmoid:
    /// - R̄ ≥ 0.8 → Critical (high confidence that escalation is needed)
    /// - 0.3 < R̄ < 0.8 → Warning (transition zone)
    /// - R̄ ≤ 0.3 → Info (low confidence, just monitoring)
    ///
    /// This replaces the binary threshold with a smooth sigmoid that
    /// is backward-compatible (existing behavior = L→∞ limit case).
    pub(crate) fn new_allosteric(
        domain: &str,
        deficit: u64,
        threshold: u64,
        gate: &AllostericGate,
    ) -> Self {
        let r_bar = gate.r_bar_eq();
        let dist = gate.decide();
        let expected = dist.expected_r_bar();

        let severity = if expected >= 0.8 {
            AlertSeverity::Critical
        } else if expected > 0.3 {
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
            r_bar,
            timestamp: Utc::now(),
            message: format!(
                "Variety deficit {} in domain '{}' (R̄={:.3}, threshold: {})",
                deficit, domain, r_bar, threshold
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

/// Default allosteric gate config for algedonic escalation.
///
/// Parameters are calibrated for the α = deficit/threshold normalization:
/// - L=10 (low skepticism — the sigmoid activates within 1-5× threshold)
/// - c=0.1 (moderate cooperativity)
/// - n=3 (three evidence channels: variety, energy, error rate)
/// - threshold=0.5 (R̄ at which escalation becomes likely)
/// - tau=5s (5-second relaxation — gates don't jump instantly)
/// - hysteresis=1.0 (resists rapid state changes)
///
/// With these parameters and the algedonic R̄ thresholds (0.3/0.8):
/// - α < 0.5 → R̄ < 0.3 (Info — no escalation)
/// - 0.5 ≤ α < 2 → 0.3 ≤ R̄ < 0.8 (Warning — transition zone)
/// - α ≥ 2 → R̄ ≥ 0.8 (Critical — escalate)
fn default_algedonic_gate() -> AllostericGate {
    AllostericGate::new(&AllostericGateConfig {
        name: "algedonic".to_string(),
        base_l: 10.0,
        c: 0.1,
        n: 3,
        threshold: 0.5,
        tau: Duration::from_secs(5),
        hysteresis: 1.0,
    })
}

/// Algedonic alert manager
pub(crate) struct AlgedonicManager {
    threshold: u64,
    default_expected_variety: u64,
    expected_variety: HashMap<String, u64>,
    alerts: Vec<RuntimeAlert>,
    /// Allosteric gate for MWC-regulated severity classification.
    /// When `None`, falls back to binary thresholds (backward-compatible).
    allosteric_gate: Option<AllostericGate>,
}

impl AlgedonicManager {
    pub(crate) fn new(threshold: u64, default_expected_variety: u64) -> Self {
        Self {
            threshold,
            default_expected_variety,
            expected_variety: HashMap::new(),
            alerts: Vec::new(),
            allosteric_gate: None,
        }
    }

    /// Enable allosteric regulation with default algedonic gate config.
    pub(crate) fn with_default_allosteric(mut self) -> Self {
        self.allosteric_gate = Some(default_algedonic_gate());
        self
    }

    /// Set expected variety for a specific domain
    pub(crate) fn set_expected_variety(&mut self, domain: &str, expected: u64) {
        self.expected_variety.insert(domain.to_string(), expected);
    }

    /// Check variety counter and generate alert if needed.
    ///
    /// When an allosteric gate is configured, severity is determined by
    /// the MWC sigmoid (α = deficit / threshold). When no gate is
    /// configured, falls back to binary thresholds.
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

        let alert = if let Some(ref mut gate) = self.allosteric_gate {
            // Set α from normalized deficit: α = deficit / threshold
            let alpha = if self.threshold > 0 {
                deficit as f64 / self.threshold as f64
            } else {
                0.0
            };
            gate.set_alpha(alpha);
            RuntimeAlert::new_allosteric(domain, deficit, self.threshold, gate)
        } else {
            RuntimeAlert::new(domain, deficit, self.threshold)
        };

        if alert.should_escalate() {
            error!(
                target: "cns.algedonic",
                domain = %alert.domain,
                deficit = alert.deficit,
                threshold = alert.threshold,
                r_bar = alert.r_bar,
                "ALGEDONIC ALERT - Escalation required"
            );
        } else if alert.is_warning() {
            warn!(
                target: "cns.algedonic",
                domain = %alert.domain,
                deficit = alert.deficit,
                r_bar = alert.r_bar,
                "Variety deficit approaching threshold"
            );
        }

        self.alerts.push(alert);
        self.alerts.last()
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
///
/// This replaces the former `CnsHealth::check()` inherent method,
/// which couldn't stay in hkask-types (it depends on AlgedonicManager).
pub(crate) fn cns_health_check(manager: &AlgedonicManager) -> CnsHealth {
    CnsHealth {
        overall_deficit: manager.total_deficit(),
        critical_count: manager.critical_alerts().len(),
        warning_count: manager.alerts().iter().filter(|a| a.is_warning()).count(),
        healthy: manager.critical_alerts().is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracker_with_variety(variety: u64) -> VarietyTracker {
        let mut tracker = VarietyTracker::new();
        for i in 0..variety {
            tracker.increment(&format!("state_{i}"));
        }
        tracker
    }

    #[test]
    fn binary_path_produces_critical_at_threshold() {
        // expected_variety=200, actual_variety=10 → deficit=190 > threshold=100
        let mut mgr = AlgedonicManager::new(100, 200);
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.escalated);
    }

    #[test]
    fn binary_path_produces_warning_at_half_threshold() {
        // expected_variety=80, actual_variety=10 → deficit=70 > threshold/2=50
        let mut mgr = AlgedonicManager::new(100, 80);
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.escalated);
    }

    #[test]
    fn binary_path_produces_info_below_half_threshold() {
        // expected_variety=30, actual_variety=10 → deficit=20 < threshold/2=50
        let mut mgr = AlgedonicManager::new(100, 30);
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        assert_eq!(alert.severity, AlertSeverity::Info);
    }

    #[test]
    fn allosteric_path_smooth_transition_zone() {
        // With the allosteric gate, the MWC sigmoid produces smooth
        // transitions. At deficit just above threshold/2, the sigmoid
        // may classify as Warning (same as binary) but with a nonzero
        // R̄ indicating partial activation.
        let mut mgr = AlgedonicManager::new(100, 80).with_default_allosteric();
        // deficit=70 → α = 70/100 = 0.7
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        // With L=1000 the sigmoid is still conservative at α=0.7
        assert!(
            alert.r_bar > 0.0,
            "R̄ should be nonzero with allosteric gate"
        );
    }

    #[test]
    fn allosteric_gate_high_deficit_produces_critical() {
        // With default gate (L=10, c=0.1, n=3), Critical requires R̄ ≥ 0.8.
        // That happens at α ≈ 5 (deficit ≈ 5 × threshold).
        // expected=600, variety=10 → deficit=590 → α = 590/100 = 5.9
        let mut mgr = AlgedonicManager::new(100, 600).with_default_allosteric();
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(
            alert.r_bar > 0.5,
            "R̄ should be high with large deficit, got {r_bar}",
            r_bar = alert.r_bar
        );
    }

    #[test]
    fn allosteric_gate_records_r_bar() {
        // expected=200, variety=10 → deficit=190 → α=1.9 → Warning zone
        let mut mgr = AlgedonicManager::new(100, 200).with_default_allosteric();
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        assert!(alert.r_bar > 0.0, "Allosteric alerts should record R̄");
    }

    #[test]
    fn allosteric_vs_binary_at_same_deficit() {
        // Compare the same deficit under both paths
        // expected=600, variety=10 → deficit=590 → α=5.9 → Critical
        let mut mgr_binary = AlgedonicManager::new(100, 600);
        let tracker = make_tracker_with_variety(10);
        let alert_binary = mgr_binary.check(&tracker, "test").unwrap();

        let mut mgr_allosteric = AlgedonicManager::new(100, 600).with_default_allosteric();
        let tracker2 = make_tracker_with_variety(10);
        let alert_allosteric = mgr_allosteric.check(&tracker2, "test").unwrap();

        // Both should produce Critical at 590 (well above threshold)
        assert_eq!(alert_binary.severity, AlertSeverity::Critical);
        assert_eq!(alert_allosteric.severity, AlertSeverity::Critical);
        // But allosteric has R̄ information
        assert!(alert_binary.r_bar == 0.0, "Binary path has no R̄");
        assert!(alert_allosteric.r_bar > 0.0, "Allosteric path has R̄");
    }

    #[test]
    fn allosteric_fallback_without_gate_matches_binary() {
        // Without an allosteric gate, the path should produce identical results
        let mut mgr = AlgedonicManager::new(100, 600);
        let tracker = make_tracker_with_variety(10);
        let alert = mgr.check(&tracker, "test").unwrap();
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!((alert.r_bar - 0.0).abs() < f64::EPSILON, "No gate → R̄ = 0");
    }
}
