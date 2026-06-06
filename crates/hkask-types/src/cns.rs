//! CNS (Cybernetic Nervous System) types for hKask
//
//! Namespace: cns.* (canonical observability namespace)
//! Key spans: cns.tool.*, cns.prompt.*, cns.inference.*, cns.agent_pod.*, cns.connector.*, cns.pipeline.*, cns.gas.*, cns.review.*, cns.template.*, cns.curation.*, cns.variety.*, cns.killzone.*, cns.sovereignty.*, cns.goal.*, cns.spec.*

use serde::{Deserialize, Serialize};

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
// namespace + path contexts. The 15 canonical namespaces are in
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
