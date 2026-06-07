//! CNS (Cybernetic Nervous System) types for hKask
//
//! Namespace: cns.* (canonical observability namespace)
//! Key spans: cns.tool.*, cns.prompt.*, cns.inference.*, cns.agent_pod.*, cns.connector.*, cns.pipeline.*, cns.gas.*, cns.review.*, cns.template.*, cns.curation.*, cns.variety.*, cns.sovereignty.*, cns.goal.*, cns.spec.*

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Domain newtypes (P2.3) ──────────────────────────────────────────────────

/// Threshold for R̄ (confidence) in the curation gate's transition zone.
///
/// Newtype wrapper around `f64` that prevents accidental confusion with
/// other floating-point quantities (priority weight, usage ratio, etc.).
///
/// Defined in hkask-types (substrate crate) because it is shared across
/// hkask-cns (set-points, allosteric regulation) and hkask-agents
/// (curation confidence gate).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RBarThreshold(pub f64);

impl RBarThreshold {
    /// Create an R̄ threshold, clamped to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        RBarThreshold(value.clamp(0.0, 1.0))
    }

    /// Default upper threshold for the Proceed zone.
    pub const DEFAULT_UPPER: RBarThreshold = RBarThreshold(0.8);
    /// Default lower threshold for the Suppress zone.
    pub const DEFAULT_LOWER: RBarThreshold = RBarThreshold(0.3);

    /// Return the raw `f64` value.
    pub fn as_raw(self) -> f64 {
        self.0
    }
}

impl fmt::Display for RBarThreshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R̄={:.2}", self.0)
    }
}

/// Communication queue depth for backpressure regulation.
///
/// Newtype wrapper that prevents accidental confusion with other numeric
/// thresholds in `SetPoints` (gas, variety deficit, error rate).
///
/// Defined in hkask-types (substrate crate) because it is shared across
/// hkask-cns (SetPoints, cybernetics loop) and hkask-agents
/// (communication loop).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QueueDepth(pub f64);

impl QueueDepth {
    /// Create a queue depth threshold.
    pub fn new(value: f64) -> Self {
        QueueDepth(value.max(0.0))
    }

    /// Default backpressure threshold: 100 messages.
    pub const DEFAULT_BACKPRESSURE: QueueDepth = QueueDepth(100.0);

    /// Return the raw `f64` value.
    pub fn as_raw(self) -> f64 {
        self.0
    }
}

impl fmt::Display for QueueDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "depth={:.0}", self.0)
    }
}

// Circuit Breaker — States

/// Circuit breaker states
///
/// Defined here (not in hkask-cns) so the `CircuitBreakerPort` trait can
/// reference it without creating an upward dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

// CNS Health — Observability data struct

/// CNS health status
///
/// Pure data struct — construction logic (`cns_health_check`) lives in
/// hkask-cns where it has access to `AlgedonicManager`.
#[derive(Debug, Clone)]
pub struct CnsHealth {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

// CnsSpan has been collapsed into SpanNamespace (in event.rs).
// Use `SpanNamespace` for namespace validation and `Span` for
// path contexts. The canonical namespaces are in
// CANONICAL_NAMESPACES.

/// RetryConfig — Canonical retry configuration for all hKask subsystems
///
/// Combines exponential backoff with retryable status codes.
/// All delays are in milliseconds for serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,
    #[serde(default)]
    pub retryable_status: Vec<u16>,
}

fn default_multiplier() -> f64 {
    2.0
}

impl RetryConfig {
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms * (self.multiplier as u64).pow(attempt);
        delay.min(self.max_delay_ms)
    }

    /// Check if a status code is retryable
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_status.contains(&status)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            multiplier: 2.0,
            retryable_status: vec![408, 429, 500, 502, 503, 504],
        }
    }
}
