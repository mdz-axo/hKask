//! Escalation domain types: thresholds, triggers, severity, alerts, and policy.
//! Pure-data module — no dependencies on external crates beyond std.

/// Default variety deficit threshold for escalation.
pub(crate) const DEFAULT_ESCALATION_VARIETY_DEFICIT: u64 = 100;

/// Default critical alert count threshold for escalation.
pub(crate) const DEFAULT_ESCALATION_CRITICAL_ALERTS: usize = 3;

/// Default bot failure count threshold for escalation.
pub(crate) const DEFAULT_ESCALATION_BOT_FAILURES: usize = 2;

/// Escalation trigger thresholds.
#[derive(Debug, Clone)]
pub(crate) struct EscalationThresholds {
    pub variety_deficit: u64,
    pub critical_alerts: usize,
    pub bot_failures: usize,
}

impl Default for EscalationThresholds {
    fn default() -> Self {
        Self {
            variety_deficit: DEFAULT_ESCALATION_VARIETY_DEFICIT,
            critical_alerts: DEFAULT_ESCALATION_CRITICAL_ALERTS,
            bot_failures: DEFAULT_ESCALATION_BOT_FAILURES,
        }
    }
}

/// The trigger that caused an escalation alert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EscalationTrigger {
    /// Variety deficit exceeded a threshold.
    VarietyDeficit,
    /// Critical alert count exceeded a threshold.
    CriticalAlerts,
    /// Bot failure count exceeded a threshold.
    BotFailures,
}

/// Algedonic signal model: Warning (threshold/2) or Critical (threshold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalationSeverity {
    Warning,
    Critical,
}

/// Alert produced when a threshold is breached.
#[derive(Debug, Clone)]
pub struct EscalationAlert {
    pub trigger: EscalationTrigger,
    pub value: f64,
    pub threshold: f64,
    pub severity: EscalationSeverity,
}

/// Encapsulates escalation threshold logic — independently testable.
/// Algedonic: Warning at threshold/2, Critical at threshold.
pub struct EscalationPolicy {
    thresholds: EscalationThresholds,
}

impl EscalationPolicy {
    pub(crate) fn new(thresholds: EscalationThresholds) -> Self {
        Self { thresholds }
    }

    /// Check all escalation conditions, return active alerts.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — escalation policy classifies variety deficit
    /// \[P4\] Constraining: Clear Boundaries — thresholds define explicit boundaries
    /// pre:  `variety_deficit`, `critical_alerts`, `bot_failures` are
    ///       non-negative numeric values.
    /// post: Returns a `Vec<EscalationAlert>` containing alerts for any
    ///       threshold exceeded: VarietyDeficit (Critical if > threshold,
    ///       Warning if > threshold/2), CriticalAlerts (Critical if ≥
    ///       threshold), BotFailures (Critical if ≥ threshold).
    pub fn check_conditions(
        &self,
        variety_deficit: f64,
        critical_alerts: u64,
        bot_failures: u64,
    ) -> Vec<EscalationAlert> {
        let mut alerts = Vec::new();

        let variety_threshold = self.thresholds.variety_deficit as f64;
        if variety_deficit > variety_threshold {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::VarietyDeficit,
                value: variety_deficit,
                threshold: variety_threshold,
                severity: EscalationSeverity::Critical,
            });
        } else if variety_deficit > variety_threshold / 2.0 {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::VarietyDeficit,
                value: variety_deficit,
                threshold: variety_threshold,
                severity: EscalationSeverity::Warning,
            });
        }

        let critical_alerts_threshold = self.thresholds.critical_alerts as f64;
        if critical_alerts >= self.thresholds.critical_alerts as u64 {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::CriticalAlerts,
                value: critical_alerts as f64,
                threshold: critical_alerts_threshold,
                severity: EscalationSeverity::Critical,
            });
        }

        let bot_failures_threshold = self.thresholds.bot_failures as f64;
        if bot_failures >= self.thresholds.bot_failures as u64 {
            alerts.push(EscalationAlert {
                trigger: EscalationTrigger::BotFailures,
                value: bot_failures as f64,
                threshold: bot_failures_threshold,
                severity: EscalationSeverity::Critical,
            });
        }

        alerts
    }
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self::new(EscalationThresholds::default())
    }
}
