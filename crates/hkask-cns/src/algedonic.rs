//! Algedonic alerts — Variety deficit escalation
//!
//! Implements algedonic (pain/pleasure) feedback for cybernetic control.
//! When variety deficit exceeds threshold, alerts are escalated to the Curator/human.
//!
//! Per architecture v0.21.0: Variety deficit >100 → escalate to Curator/human

use crate::variety::{VarietyCounter, VarietyMonitor};
use std::time::{Duration, Instant};
use tracing::{error, warn};

/// Callback type for escalation notifications
pub type EscalationCallback = dyn Fn(&AlgedonicAlert) + Send + Sync;

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

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
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
    escalation_callback: Option<Box<EscalationCallback>>,
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
            error!(
                target: "cns.algedonic",
                domain = %alert.domain,
                deficit = alert.deficit,
                threshold = alert.threshold,
                "ALGEDONIC ALERT - Escalation required"
            );
            if let Some(callback) = &self.escalation_callback {
                callback(&alert);
            }
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

    /// Check variety monitor across all domains
    /// Returns count of alerts generated
    pub fn check_all(&mut self, monitor: &mut VarietyMonitor) -> usize {
        let domains: Vec<String> = monitor.domains().iter().map(|s| s.to_string()).collect();
        let mut count = 0;

        for domain in domains {
            if self.check(monitor.counter(&domain), &domain).is_some() {
                count += 1;
            }
        }

        count
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

