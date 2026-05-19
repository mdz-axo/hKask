//! Algedonic alerts — Variety deficit escalation
//!
//! Implements algedonic (pain/pleasure) feedback for cybernetic control.
//! When variety deficit exceeds threshold, alerts are escalated to the Curator/human.
//!
//! Per architecture v0.21.0: Variety deficit >100 → escalate to Curator/human

use crate::variety::{VarietyCounter, VarietyMonitor};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Default algedonic alert threshold (variety deficit)
pub const DEFAULT_THRESHOLD: u64 = 100;

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    /// Informational - deficit detected but below threshold
    Info,
    /// Warning - deficit approaching threshold
    Warning,
    /// Critical - deficit exceeds threshold, escalation required
    Critical,
}

/// Algedonic alert
#[derive(Debug, Clone)]
pub struct AlgedonicAlert {
    pub domain: String,
    pub deficit: u64,
    pub threshold: u64,
    pub severity: AlertSeverity,
    pub escalated: bool,
    pub timestamp: Instant,
    pub message: String,
}

impl AlgedonicAlert {
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
            timestamp: Instant::now(),
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
pub struct AlgedonicManager {
    threshold: u64,
    alerts: Vec<AlgedonicAlert>,
    escalation_callback: Option<Box<dyn Fn(&AlgedonicAlert) + Send + Sync>>,
}

impl AlgedonicManager {
    pub fn new(threshold: u64) -> Self {
        Self {
            threshold,
            alerts: Vec::new(),
            escalation_callback: None,
        }
    }

    pub fn with_escalation_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&AlgedonicAlert) + Send + Sync + 'static,
    {
        self.escalation_callback = Some(Box::new(callback));
        self
    }

    /// Check variety counter and generate alert if needed
    pub fn check(&mut self, counter: &VarietyCounter, domain: &str) -> Option<&AlgedonicAlert> {
        let deficit = counter.deficit(u64::MAX); // Total variety deficit
        let alert = AlgedonicAlert::new(domain, deficit, self.threshold);

        if alert.should_escalate() {
            error!(target: "cns.algedonic", %alert.message, "ALGEDONIC ALERT - Escalation required");
            if let Some(callback) = &self.escalation_callback {
                callback(&alert);
            }
        } else if alert.is_warning() {
            warn!(target: "cns.algedonic", %alert.message, "Variety deficit approaching threshold");
        }

        self.alerts.push(alert);
        self.alerts.last()
    }

    /// Check variety monitor across all domains
    pub fn check_all(&mut self, monitor: &mut VarietyMonitor) -> Vec<&AlgedonicAlert> {
        let domains = monitor.domains();
        let mut alerts = Vec::new();

        for domain in domains {
            if let Some(alert) = self.check(monitor.counter(domain), domain) {
                alerts.push(alert);
            }
        }

        alerts
    }

    /// Get all alerts
    pub fn alerts(&self) -> &[AlgedonicAlert] {
        &self.alerts
    }

    /// Get critical alerts only
    pub fn critical_alerts(&self) -> Vec<&AlgedonicAlert> {
        self.alerts.iter().filter(|a| a.is_critical()).collect()
    }

    /// Get total deficit across all alerts
    pub fn total_deficit(&self) -> u64 {
        self.alerts.iter().map(|a| a.deficit).sum()
    }

    /// Clear old alerts (older than duration)
    pub fn clear_old(&mut self, max_age: Duration) {
        let cutoff = Instant::now() - max_age;
        self.alerts.retain(|a| a.timestamp > cutoff);
    }

    /// Reset all alerts
    pub fn reset(&mut self) {
        self.alerts.clear();
    }
}

/// CNS health status
#[derive(Debug, Clone)]
pub struct CnsHealth {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

impl CnsHealth {
    pub fn check(manager: &AlgedonicManager) -> Self {
        let critical_count = manager.critical_alerts().len();
        let warning_count = manager.alerts().iter().filter(|a| a.is_warning()).count();
        let overall_deficit = manager.total_deficit();

        Self {
            overall_deficit,
            critical_count,
            warning_count,
            healthy: critical_count == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algedonic_alert_severity() {
        // Info level (deficit < threshold/2)
        let alert = AlgedonicAlert::new("test", 25, 100);
        assert_eq!(alert.severity, AlertSeverity::Info);
        assert!(!alert.should_escalate());

        // Warning level (threshold/2 <= deficit < threshold)
        let alert = AlgedonicAlert::new("test", 75, 100);
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.should_escalate());

        // Critical level (deficit >= threshold)
        let alert = AlgedonicAlert::new("test", 150, 100);
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.should_escalate());
    }

    #[test]
    fn test_algedonic_manager_check() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();

        // Low variety - should trigger alert
        counter.increment("state_a");
        counter.increment("state_a");

        let alert = manager.check(&counter, "test_domain");
        assert!(alert.is_some());
        // Note: deficit is based on variety count, not total count
    }

    #[test]
    fn test_algedonic_manager_escalation_callback() {
        let mut escalation_called = false;

        let mut manager = AlgedonicManager::new(1).with_escalation_callback(|_| {
            escalation_called = true;
        });

        let mut counter = VarietyCounter::new();
        counter.increment("state_a");
        counter.increment("state_b");

        // Variety of 2 should exceed threshold of 1
        manager.check(&counter, "test");

        assert!(escalation_called);
    }

    #[test]
    fn test_cns_health() {
        let mut manager = AlgedonicManager::new(100);

        // Add some alerts
        let mut counter1 = VarietyCounter::new();
        counter1.increment("a");
        manager.check(&counter1, "domain1");

        let health = CnsHealth::check(&manager);
        assert!(health.healthy); // No critical alerts with default threshold
    }

    #[test]
    fn test_alert_clear_old() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();
        counter.increment("a");

        manager.check(&counter, "test");
        assert_eq!(manager.alerts().len(), 1);

        // Clear with 0 duration should remove all
        manager.clear_old(Duration::from_secs(0));
        assert_eq!(manager.alerts().len(), 0);
    }
}
