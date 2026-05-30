//! CNS (Cybernetic Nervous System) types for hKask
//!
//! Namespace: cns.* (canonical observability namespace)
//! Key spans: cns.tool.*, cns.prompt.*, cns.agent_pod.*, cns.connector.*, cns.template.*, cns.curation.*

use crate::event::SpanCategory;
use serde::{Deserialize, Serialize};

/// VarietyCounter — Tracks diversity in system behavior
///
/// Algedonic Alert: Variety deficit >100 → escalate to Curator/human
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VarietyCounter(pub u64);

impl VarietyCounter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn decrement(&mut self) {
        if self.0 > 0 {
            self.0 -= 1;
        }
    }

    pub fn deficit(&self, target: u64) -> u64 {
        target.saturating_sub(self.0)
    }

    /// Default target variety level
    pub fn target() -> u64 {
        100
    }

    /// Check if variety deficit exceeds algedonic threshold
    /// Alert triggers when deficit > 100 (i.e., counter < 0 when target is 100)
    pub fn needs_alert(&self) -> bool {
        self.deficit(Self::target()) >= 100
    }
}

impl Default for VarietyCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VarietyCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// AlgedonicAlert — Cybernetic alert when variety deficit exceeds threshold
///
/// Named after algedonic meter in Beer's viable system model.
/// Signals pain/pleasure balance in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgedonicAlert {
    /// Unique alert identifier
    pub id: u64,
    /// Current variety counter value
    pub current: u64,
    /// Threshold that triggered alert
    pub threshold: u64,
    /// Deficit amount
    pub deficit: u64,
    /// Whether alert has been escalated to Curator/human
    pub escalated: bool,
    /// Timestamp of alert
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Span category where deficit was detected
    pub span: SpanCategory,
}

impl AlgedonicAlert {
    pub fn new(current: u64, threshold: u64, span: SpanCategory) -> Self {
        let deficit = threshold.saturating_sub(current);

        Self {
            id: Self::generate_id(),
            current,
            threshold,
            deficit,
            escalated: false,
            timestamp: chrono::Utc::now(),
            span,
        }
    }

    pub fn escalate(&mut self) {
        self.escalated = true;
    }

    fn generate_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after 1970")
            .as_nanos() as u64
    }
}

impl std::fmt::Display for AlgedonicAlert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AlgedonicAlert[deficit={}, span={}, escalated={}]",
            self.deficit, self.span, self.escalated
        )
    }
}

// CnsSpan has been collapsed into SpanCategory (in event.rs).
// Use `SpanCategory` for category-only contexts and `Span` for
// category + path contexts. The 14 variants are identical.

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
    pub fn new(max_retries: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            initial_delay_ms,
            max_delay_ms,
            multiplier: 2.0,
            retryable_status: Vec::new(),
        }
    }

    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    pub fn with_retryable_status(mut self, status: Vec<u16>) -> Self {
        self.retryable_status = status;
        self
    }

    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms * (self.multiplier as u64).pow(attempt);
        delay.min(self.max_delay_ms)
    }

    /// Check if retry should continue (attempt < max_retries)
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
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
