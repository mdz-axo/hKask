//! CNS (Cybernetic Nervous System) types for hKask
//!
//! Namespace: cns.* (canonical observability namespace)
//! Key spans: cns.tool.*, cns.prompt.*, cns.agent_pod.*, cns.connector.*, cns.template.*, cns.curation.*

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
    /// Span where deficit was detected
    pub span: CnsSpan,
}

impl AlgedonicAlert {
    pub fn new(current: u64, threshold: u64, span: CnsSpan) -> Self {
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
            .unwrap()
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

/// CnsSpan — Namespace for CNS monitoring spans
///
/// All CNS spans use cns.* prefix for observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CnsSpan {
    /// Tool governance, invocation (cns.tool.*)
    Tool,
    /// Prompt render, validate, outcome (cns.prompt.*)
    Prompt,
    /// Agent pod lifecycle, delegation (cns.agent_pod.*)
    AgentPod,
    /// External I/O: LLM, embeddings (cns.connector.*)
    Connector,
    /// Template invocation, registry (cns.template.*)
    Template,
    /// Curation decisions, OCAP boundaries (cns.curation.*)
    Curation,
    /// Variety monitoring, algedonic alerts (cns.variety.*)
    Variety,
    /// Kill zone detection (cns.killzone.*)
    KillZone,
    /// User sovereignty, acquisition resistance (cns.sovereignty.*)
    Sovereignty,
    /// Goal primitive (cns.goal.*)
    Goal,
    /// Specification operations: capture, compose, validate, sign, curate (cns.spec.*)
    Spec,
}

impl CnsSpan {
    /// Full span name with cns. prefix
    pub fn full_name(&self) -> String {
        match self {
            CnsSpan::Tool => "cns.tool".to_string(),
            CnsSpan::Prompt => "cns.prompt".to_string(),
            CnsSpan::AgentPod => "cns.agent_pod".to_string(),
            CnsSpan::Connector => "cns.connector".to_string(),
            CnsSpan::Template => "cns.template".to_string(),
            CnsSpan::Curation => "cns.curation".to_string(),
            CnsSpan::Variety => "cns.variety".to_string(),
            CnsSpan::KillZone => "cns.killzone".to_string(),
            CnsSpan::Sovereignty => "cns.sovereignty".to_string(),
            CnsSpan::Goal => "cns.goal".to_string(),
            CnsSpan::Spec => "cns.spec".to_string(),
        }
    }
}

impl std::fmt::Display for CnsSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

/// CnsEvent — Cybernetic audit trail event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnsEvent {
    pub id: u64,
    pub span: CnsSpan,
    pub action: String,
    pub outcome: String,
    pub variety_before: Option<VarietyCounter>,
    pub variety_after: Option<VarietyCounter>,
    pub alert: Option<AlgedonicAlert>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl CnsEvent {
    pub fn new(span: CnsSpan, action: String, outcome: String) -> Self {
        Self {
            id: Self::generate_id(),
            span,
            action,
            outcome,
            variety_before: None,
            variety_after: None,
            alert: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_variety(mut self, before: VarietyCounter, after: VarietyCounter) -> Self {
        self.variety_before = Some(before);
        self.variety_after = Some(after);
        self
    }

    pub fn with_alert(mut self, alert: AlgedonicAlert) -> Self {
        self.alert = Some(alert);
        self
    }

    fn generate_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// KillZoneState — Tracking state for catch-and-kill detection
///
/// Monitors VC investment patterns that indicate kill zone formation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillZoneState {
    /// Space/technology being monitored
    pub space_id: String,
    /// VC investment level (normalized 0.0-1.0)
    pub vc_investment: f32,
    /// Acquisition count in last N days
    pub acquisition_count: u32,
    /// Whether kill zone is detected
    pub kill_zone_detected: bool,
    /// Timestamp of last update
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl KillZoneState {
    pub fn new(space_id: String) -> Self {
        Self {
            space_id,
            vc_investment: 1.0,
            acquisition_count: 0,
            kill_zone_detected: false,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Update VC investment level
    pub fn update_vc_investment(&mut self, level: f32) {
        self.vc_investment = level.clamp(0.0, 1.0);
        self.last_updated = chrono::Utc::now();

        // Kill zone detected if VC investment drops below 0.5 after major acquisition
        if self.vc_investment < 0.5 && self.acquisition_count > 0 {
            self.kill_zone_detected = true;
        }
    }

    /// Record acquisition event
    pub fn record_acquisition(&mut self) {
        self.acquisition_count += 1;
        self.last_updated = chrono::Utc::now();
    }

    /// Check if kill zone is active
    pub fn is_kill_zone(&self) -> bool {
        self.kill_zone_detected
    }
}

/// TokenBucket — General-purpose token bucket rate limiter
///
/// Uses f64 for fractional token accumulation. Suitable for
/// rate limiting across all hKask subsystems.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: std::time::Instant,
}

impl TokenBucket {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: std::time::Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    pub fn consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    pub fn available(&self) -> f64 {
        self.tokens
    }
}

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

/// ObservabilityPort — Canonical observability trait for hKask
///
/// Provides CNS span emission and health checking across all subsystems.
/// Implementations emit structured telemetry via `NuEventSink`.
pub trait ObservabilityPort: Send + Sync {
    /// Record a counter metric
    fn record_counter(&self, name: &str, value: u64, labels: &[(&str, &str)]);

    /// Record a histogram observation
    fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);

    /// Check system health
    fn health_check(&self) -> HealthStatus;
}

/// Health status for observability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Observability errors
#[derive(Debug, thiserror::Error)]
pub enum ObservabilityError {
    #[error("Emission failed: {0}")]
    EmissionFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Channel closed")]
    ChannelClosed,
}
