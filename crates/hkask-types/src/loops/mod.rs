//! hKask 6-Loop Architecture
//!
//! Six cybernetic feedback loops following Beer's Viable System Model.
//! Each loop implements sense → compare → compute → act.
//!
//! **Loop Numbering (VSM correspondence):**
//!
//! The numbering follows Stafford Beer's VSM. Loop 3 (Control) is absorbed
//! into Loop 6 (Cybernetics) — the homeostatic regulator IS the controller.
//! There is no Loop 3; this is intentional, not a gap.
//!
//! | Loop | Name | VSM Role | Category |
//! |------|------|----------|----------|
//! | 1 | Inference | Implementation | Domain |
//! | 2a | Episodic Memory | Coordination (private) | Domain |
//! | 2b | Semantic Memory | Coordination (shared) | Domain |
//! | 4 | Communication | Channel (dumb pipe) | Meta |
//! | 5 | Curation | Metasystem (observer) | Meta |
//! | 6 | Cybernetics | Homeostatic regulation | Meta |
//! | 6b | Snapshot | Scheduled CAS snapshots | Meta |
//!
//! **Bridge:**
//! - 2a→2b: Consolidation — episodic → strip perspective → store semantic (one-way)
//!
//! **Authority DAG:** Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}
//! No sideways edges. Authority flows downward.

pub mod curation;
pub mod dispatch;
pub mod episodic;

pub use curation::{CuratorDirective, CuratorHandle};
pub use dispatch::{
    DispatchTarget, LoopMessage, LoopPayload, MessagePriority, TraceId, WorkerKind,
};
pub use episodic::ExperienceClassification;

pub use self::Loop as HkaskLoop;

/// Loop identifiers for the 6-loop model.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LoopId {
    Inference,
    Episodic,
    Semantic,
    Communication,
    Curation,
    Cybernetics,
    /// Scheduled CAS snapshots (sub-function of Cybernetics Loop 6)
    Snapshot,
}

impl std::fmt::Display for LoopId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopId::Inference => write!(f, "inference"),
            LoopId::Episodic => write!(f, "episodic"),
            LoopId::Semantic => write!(f, "semantic"),
            LoopId::Communication => write!(f, "communication"),
            LoopId::Curation => write!(f, "curation"),
            LoopId::Cybernetics => write!(f, "cybernetics"),
            LoopId::Snapshot => write!(f, "snapshot"),
        }
    }
}

// Loop cycle — sense → compare → compute → act

/// Metric names for afferent signals from loop sensing.
///
/// Each variant identifies the kind of measurement a signal carries,
/// replacing magic strings with an exhaustive, type-safe enum
/// (Fowler H7: Replace Type Code with Strategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalMetric {
    /// Fraction of gas budget remaining (Cybernetics Loop 6)
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
    /// Confidence decay rate (Episodic Loop 2a)
    DecayRate,
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
            SignalMetric::DecayRate => "decay_rate",
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

/// Efferent action produced by a loop's compute phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopAction {
    pub target: crate::loops::dispatch::DispatchTarget,
    pub action_type: ActionType,
    pub parameters: serde_json::Value,
    pub priority: MessagePriority,
}

impl LoopAction {
    pub fn new(
        target: impl Into<crate::loops::dispatch::DispatchTarget>,
        action_type: ActionType,
        parameters: serde_json::Value,
    ) -> Self {
        let priority = match &action_type {
            ActionType::Throttle => MessagePriority::Warning,
            ActionType::Escalate => MessagePriority::Critical,
            ActionType::Calibrate => MessagePriority::Info,
            ActionType::CircuitBreak => MessagePriority::Critical,
            ActionType::AdjustGasBudget => MessagePriority::Warning,
            ActionType::OverrideGasBudget => MessagePriority::Critical,
            ActionType::ReplenishBudget => MessagePriority::Info,
        };
        Self {
            target: target.into(),
            action_type,
            parameters,
            priority,
        }
    }
}

/// Types of regulatory actions a loop can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    /// Reduce resource allocation to a target loop
    Throttle,
    /// Escalate an alert to the Curation loop
    Escalate,
    /// Adjust a threshold or set-point
    Calibrate,
    /// Open a circuit breaker on a target
    CircuitBreak,
    /// Adjust energy budget within set-point bounds (Cybernetics automatic regulation)
    ///
    /// This is a *weaker* capability than `OverrideGasBudget`.
    /// Cybernetics can adjust within its set-point range.
    /// Only Curation can override set-points themselves.
    AdjustGasBudget,
    /// Override energy budget beyond set-point bounds (Curation metacognitive override)
    ///
    /// This is a *stronger* capability than `AdjustGasBudget`.
    /// Only Curation can issue this — it can exceed Cybernetics' set-point range.
    OverrideGasBudget,
    /// Replenish an agent's gas budget (Curation directive)
    ///
    /// Used when an agent has exhausted its budget but should continue.
    /// This is the Curator's ability to inject gas into the system.
    ReplenishBudget,
}

/// The Loop trait — sense → compare → compute → act.
///
/// Every loop implements this cycle. Authority flows downward
/// through the DAG: Curation → Cybernetics → domain loops.
///
/// Native async (Rust 2024 edition). Implementations that need
/// async I/O (e.g., reading from `CnsRuntime`) can do so directly
/// without `async_trait` boxing.
///
/// All async methods return `Send` futures so loops can run in
/// async tasks without `static bounds issues.
#[async_trait::async_trait]
pub trait Loop: Send + Sync {
    fn id(&self) -> LoopId;

    /// Worker kind — `Some` for non-governing workers, `None` for governing loops.
    ///
    /// Workers (Metacognition, ToolDispatch) operate within a parent loop
    /// and have no authority rank. Governing loops return `None`.
    fn worker_kind(&self) -> Option<crate::loops::dispatch::WorkerKind> {
        None
    }

    /// Sense: observe current state and produce afferent signals.
    async fn sense(&self) -> Vec<Signal>;

    /// Compare: detect deviations from set-points.
    async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        signals.iter().filter_map(Deviation::from_signal).collect()
    }

    /// Compute: produce regulatory actions for detected deviations.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction>;

    /// Act: execute regulatory actions (route through Communication Loop).
    async fn act(&self, actions: &[LoopAction]);

    /// Full regulation cycle: sense → compare → compute → act.
    async fn tick(&self) {
        let signals = self.sense().await;
        let deviations = self.compare(&signals).await;
        let actions = self.compute(&deviations).await;
        self.act(&actions).await;
    }
}
