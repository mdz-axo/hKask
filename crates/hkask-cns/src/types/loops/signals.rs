//! Signal types — metrics, afferent signals, deviations, and deviation direction.

use super::core::LoopId;

/// Metric names for afferent signals from loop sensing.
///
/// Each variant identifies the kind of measurement a signal carries,
/// replacing magic strings with an exhaustive, type-safe enum
/// (Fowler H7: Replace Type Code with Strategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalMetric {
    /// Fraction of energy budget remaining (Cybernetics Loop 6)
    EnergyRemaining,
    /// Raw variety deficit count (Cybernetics Loop 6)
    VarietyDeficit,
    /// Error rate as a fraction (Cybernetics Loop 6)
    ErrorRate,
    /// Connector latency in milliseconds (Cybernetics Loop 6)
    ConnectorLatency,
    /// Communication queue depth (backpressure signal)
    CommunicationQueueDepth,
    /// Episodic storage usage fraction (Episodic Loop 2a)
    StorageUsage,
    /// Episodic memory life S in days (Episodic Loop 2a).
    /// Wozniak-Gorzelanczyk (1995) forgetting curve: R(t) = exp(-t/S).
    /// Default 180 days. Configurable via HKASK_MEMORY_LIFE_DAYS.
    MemoryLife,
    /// Semantic triple count (Semantic Loop 2b)
    TripleCount,
    /// Low-confidence triple count (Semantic Loop 2b)
    LowConfidenceCount,
    /// Dispatch queue depth (Communication Loop 4)
    QueueDepth,
    /// Registered loop count (Communication Loop 4)
    RegisteredLoops,
    /// Tool dispatch queue depth (Communication Loop 4)
    ToolDispatchQueueDepth,
    /// Circuit breaker state 0.0/1.0 (Inference Loop 1)
    CircuitBreakerState,
    /// Inference availability 0.0/1.0 (Inference Loop 1)
    InferenceAvailable,
    /// Inference gas remaining fraction (Inference Loop 1)
    InferenceGasRemaining,
    /// Model availability 0.0/1.0 (Inference Loop 1)
    InferenceModelAvailable,
    /// Algedonic event count (Cybernetics Loop 6)
    AlgedonicEvents,
    /// Pending escalation count (Curation Loop 5)
    PendingEscalations,
    /// Consolidation candidate count (Episodic → Semantic bridge)
    ConsolidationCandidates,
    /// Stale goal count (Curation Loop 5)
    GoalStaleCount,
    /// Expired goal count (Curation Loop 5)
    GoalExpiredCount,
    /// Spec drift alert count (Cybernetics Loop 6)
    SpecDriftAlertCount,
    /// Metacognition variety deficit (Curation Loop 5)
    MetacognitionVarietyDeficit,
    /// Metacognition critical alert count (Curation Loop 5)
    MetacognitionCriticalAlerts,
    /// Metacognition bot failure count (Curation Loop 5)
    MetacognitionBotFailures,
    /// Seconds since last CAS snapshot vs. policy interval (Cybernetics Loop 6)
    SnapshotInterval,
    /// Wallet rJoule balance ratio (0.0 = empty, 1.0 = full relative to 30-day avg)
    WalletBalanceRatio,
    /// Wallet treasury reserve ratio (0.0 = below min_reserve, 1.0 = healthy)
    WalletTreasuryRatio,
    /// Wallet API key health (1.0 = exhausted/expired, 0.0 = healthy)
    WalletKeyHealth,
    /// Public seam coverage ratio per crate (R7.3 watcher, 0.0–100.0)
    SeamCoverage,
}

impl std::fmt::Display for SignalMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self));
        write!(f, "{}", s)
    }
}

impl SignalMetric {
    /// Returns the snake_case string representation for comparison.
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalMetric::EnergyRemaining => "energy_remaining",
            SignalMetric::VarietyDeficit => "variety_deficit",
            SignalMetric::ErrorRate => "error_rate",
            SignalMetric::ConnectorLatency => "connector_latency",
            SignalMetric::CommunicationQueueDepth => "communication_queue_depth",
            SignalMetric::StorageUsage => "storage_usage",
            SignalMetric::MemoryLife => "memory_life",
            SignalMetric::TripleCount => "triple_count",
            SignalMetric::LowConfidenceCount => "low_confidence_count",
            SignalMetric::QueueDepth => "queue_depth",
            SignalMetric::RegisteredLoops => "registered_loops",
            SignalMetric::ToolDispatchQueueDepth => "tool_dispatch_queue_depth",
            SignalMetric::CircuitBreakerState => "circuit_breaker_state",
            SignalMetric::InferenceAvailable => "inference_available",
            SignalMetric::InferenceGasRemaining => "inference_gas_remaining",
            SignalMetric::InferenceModelAvailable => "inference_model_available",
            SignalMetric::AlgedonicEvents => "algedonic_events",
            SignalMetric::PendingEscalations => "pending_escalations",
            SignalMetric::ConsolidationCandidates => "consolidation_candidates",
            SignalMetric::GoalStaleCount => "goal_stale_count",
            SignalMetric::GoalExpiredCount => "goal_expired_count",
            SignalMetric::SpecDriftAlertCount => "spec_drift_alert_count",
            SignalMetric::MetacognitionVarietyDeficit => "metacognition_variety_deficit",
            SignalMetric::MetacognitionCriticalAlerts => "metacognition_critical_alerts",
            SignalMetric::MetacognitionBotFailures => "metacognition_bot_failures",
            SignalMetric::SnapshotInterval => "snapshot_interval",
            SignalMetric::WalletBalanceRatio => "wallet_balance_ratio",
            SignalMetric::WalletTreasuryRatio => "wallet_treasury_ratio",
            SignalMetric::WalletKeyHealth => "wallet_key_health",
            SignalMetric::SeamCoverage => "seam_coverage",
        }
    }
}

/// Afferent signal from a loop's sensing phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Signal {
    pub source: LoopId,
    pub metric: SignalMetric,
    pub value: f64,
    pub set_point: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Signal {
    pub fn new(source: LoopId, metric: SignalMetric, value: f64, set_point: f64) -> Self {
        Self {
            source,
            metric,
            value,
            set_point,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Deviation detected when comparing a signal against its set-point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Deviation {
    pub signal: Signal,
    pub magnitude: f64,
    pub direction: DeviationDirection,
}

impl Deviation {
    pub fn from_signal(signal: &Signal) -> Option<Self> {
        let diff = signal.value - signal.set_point;
        if diff.abs() < f64::EPSILON {
            return None;
        }
        Some(Self {
            signal: signal.clone(),
            magnitude: diff.abs(),
            direction: if diff > 0.0 {
                DeviationDirection::AboveSetPoint
            } else {
                DeviationDirection::BelowSetPoint
            },
        })
    }
}

/// Direction of a deviation relative to the set-point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeviationDirection {
    AboveSetPoint,
    BelowSetPoint,
}
