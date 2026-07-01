//! Metacognition configuration types and constants.

use std::collections::HashMap;
use std::time::Duration;

use hkask_types::event::SpanNamespace;

pub(crate) const MC_TARGET: &str = "curator.metacognition";

/// Default interval between metacognition cycles (1 hour).
pub(crate) const DEFAULT_METACOGNITION_INTERVAL_SECS: u64 = 3600;

/// Default expected variety per domain for deficit calculation.
pub(crate) const DEFAULT_EXPECTED_VARIETY_PER_DOMAIN: u64 = 50;

/// Default maximum concurrent escalations (VSM algedonic paradox — fewer signals = higher fidelity).
pub(crate) const DEFAULT_MAX_CONCURRENT_ESCALATIONS: usize = 3;

/// Health snapshot — unified system health state.
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cns_health: String,
    pub variety_counters: HashMap<SpanNamespace, u64>,
    pub variety_deficit: u64,
    pub critical_alerts: usize,
    pub total_alerts: usize,
}

/// Metacognition loop configuration.
#[derive(Debug, Clone)]
pub struct MetacognitionConfig {
    /// Interval between metacognition cycles (default: 1 hour)
    pub interval: Duration,
    /// Escalation thresholds
    pub(crate) thresholds: super::escalation::EscalationThresholds,
    /// Expected variety per domain (for deficit calculation)
    pub expected_variety_per_domain: u64,
    /// Max concurrent escalations before batching (VSM algedonic paradox). Default: 3.
    pub max_concurrent_escalations: usize,
}

impl Default for MetacognitionConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_METACOGNITION_INTERVAL_SECS),
            thresholds: super::escalation::EscalationThresholds::default(),
            expected_variety_per_domain: DEFAULT_EXPECTED_VARIETY_PER_DOMAIN,
            max_concurrent_escalations: DEFAULT_MAX_CONCURRENT_ESCALATIONS,
        }
    }
}
