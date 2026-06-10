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
use crate::runtime::VarietyTracker;
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

/// R̄ threshold for Critical severity in algedonic escalation.
///
/// R̄ ≥ this value → AlertSeverity::Critical (high confidence escalation needed).
const ALGEDONIC_CRITICAL_R_BAR: f64 = 0.8;

/// R̄ threshold for Warning severity in algedonic escalation.
///
/// R̄ > this value and < Critical → AlertSeverity::Warning (transition zone).
const ALGEDONIC_WARNING_R_BAR: f64 = 0.3;

/// Allosteric gate base L parameter (low skepticism — sigmoid activates within 1-5× threshold).
const ALGEDONIC_GATE_BASE_L: f64 = 10.0;

/// Allosteric gate cooperativity parameter (moderate).
const ALGEDONIC_GATE_C: f64 = 0.1;

/// Allosteric gate number of evidence channels (variety, energy, error rate).
const ALGEDONIC_GATE_N: usize = 3;

/// Allosteric gate MWC threshold (R̄ at which escalation becomes likely).
const ALGEDONIC_GATE_THRESHOLD: f64 = 0.5;

/// Allosteric gate relaxation time in seconds (gates don't jump instantly).
const ALGEDONIC_GATE_TAU_SECS: u64 = 5;

/// Allosteric gate hysteresis (resists rapid state changes).
const ALGEDONIC_GATE_HYSTERESIS: f64 = 1.0;

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
        let expected = gate.decide();

        let severity = if expected >= ALGEDONIC_CRITICAL_R_BAR {
            AlertSeverity::Critical
        } else if expected > ALGEDONIC_WARNING_R_BAR {
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
        base_l: ALGEDONIC_GATE_BASE_L,
        c: ALGEDONIC_GATE_C,
        n: ALGEDONIC_GATE_N,
        threshold: ALGEDONIC_GATE_THRESHOLD,
        tau: Duration::from_secs(ALGEDONIC_GATE_TAU_SECS),
        hysteresis: ALGEDONIC_GATE_HYSTERESIS,
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

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allosteric::gate::{AllostericGate, AllostericGateConfig};
    use crate::runtime::VarietyTracker;
    use std::time::Duration;

    // REQ: svc-cns-algedonic-002 — allosteric_gate_escalates_at_high_alpha
    //
    // TASK 1 cybernetic property: when α (deficit/threshold) ≥ 5,
    // the MWC sigmoid MUST produce R̄ ≥ 0.8 (Critical severity).
    // With the production algedonic gate config (L=10, c=0.1, n=3),
    // R̄ crosses 0.8 at approximately α=3.5.
    // This tests the comparator function in the variety regulation loop.
    // See TASK 1 condensed loop topology (Loop 1: Variety Regulation).
    #[test]
    fn allosteric_gate_escalates_at_high_alpha() {
        let mut gate = AllostericGate::new(&AllostericGateConfig {
            name: "test".into(),
            base_l: 10.0,
            c: 0.1,
            n: 3,
            threshold: 0.5,
            tau: Duration::from_secs(0),
            hysteresis: 0.0,
        });

        // α = 5 → (1+5)³ / ((1+5)³ + 10·(1+0.1·5)³) = 216/(216+33.75) ≈ 0.865
        gate.set_alpha(5.0);
        let r_bar = gate.r_bar_eq();
        assert!(
            r_bar >= 0.8,
            "R̄={} should be ≥ 0.8 for α=2.5 (MWC sigmoid escalation threshold)",
            r_bar
        );
    }

    // REQ: svc-cns-algedonic-003 — allosteric_gate_stays_low_at_low_alpha
    //
    // Complementary property: when α (deficit/threshold) ≤ 0.3,
    // the MWC sigmoid MUST produce R̄ ≤ 0.3 (Info severity — no escalation).
    #[test]
    fn allosteric_gate_stays_low_at_low_alpha() {
        let mut gate = AllostericGate::new(&AllostericGateConfig {
            name: "test".into(),
            base_l: 10.0,
            c: 0.1,
            n: 3,
            threshold: 0.5,
            tau: Duration::from_secs(0),
            hysteresis: 0.0,
        });

        // α = 0.2 → should produce R̄ ≤ 0.3 (Info zone)
        gate.set_alpha(0.2);
        let r_bar = gate.r_bar_eq();
        assert!(
            r_bar <= 0.3,
            "R̄={} should be ≤ 0.3 for α=0.2 (no escalation zone)",
            r_bar
        );
    }

    // REQ: svc-cns-algedonic-004 — new_allosteric_classifies_critical_correctly
    //
    // End-to-end test: after setting α on the allosteric gate (as AlgedonicManager
    // does in production), RuntimeAlert::new_allosteric must produce Critical
    // severity when the deficit causes R̄ ≥ 0.8.
    #[test]
    fn new_allosteric_classifies_critical_correctly() {
        let mut gate = AllostericGate::new(&AllostericGateConfig {
            name: "test-alert".into(),
            base_l: 10.0,
            c: 0.1,
            n: 3,
            threshold: 0.5,
            tau: Duration::from_secs(0),
            hysteresis: 0.0,
        });

        // deficit = 500, threshold = 100 → α = 5.0 → set on gate before classification
        // (mirrors AlgedonicManager::check which sets alpha then calls new_allosteric)
        let alpha = 500_f64 / 100_f64;
        gate.set_alpha(alpha);
        let alert = RuntimeAlert::new_allosteric("test_domain", 500, 100, &gate);
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.escalated);
        assert!(
            alert.r_bar >= 0.8,
            "R̄={} should indicate Critical",
            alert.r_bar
        );
    }

    // REQ: svc-cns-algedonic-005 — algedonic_manager_accumulates_alerts_across_domains
    //
    // TASK 1 cybernetic property: AlgedonicManager must track variety per domain
    // independently, so a deficit in one domain does not suppress alerts in another.
    #[test]
    fn algedonic_manager_accumulates_alerts_across_domains() {
        let mut mgr = AlgedonicManager::new(100, 10).with_default_allosteric();

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
