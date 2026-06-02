//! hKask 6-Loop Architecture
//!
//! Six cybernetic feedback loops following Beer's Viable System Model.
//! Each loop implements sense → compare → compute → act.
//!
//! **Domain Loops:**
//! - Loop 1: Inference — prompt → context → model → response → parse → act
//! - Loop 2a: Episodic Memory — experience → encode → store (private) → recall → temporal weight → context
//! - Loop 2b: Semantic Memory — knowledge → store (public) → index → recall → dedup → combine → context
//!
//! **Meta Loops:**
//! - Loop 4: Communication — send → observe delivery → confirm (dumb transport pipe)
//! - Loop 5: Curation — observe → evaluate → compose → regulate (metacognitive observer)
//! - Loop 6: Cybernetics — sense → regulate → adapt (homeostatic self-regulation)
//!
//! **Bridge:**
//! - 2a→2b: Consolidation — episodic → strip perspective → dedup → store semantic (one-way)
//!
//! **Authority DAG:**
//! - Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}
//! - Episodic → Semantic (consolidation bridge, one-way)
//! - No sideways edges. Authority flows downward.

pub mod curation;
pub mod cybernetics;
pub mod dispatch;
pub mod episodic;
pub mod inference;
pub mod semantic;

pub use curation::{CuratorDirective, CuratorHandle};
pub use cybernetics::{CyberneticsHandle, GovernanceDenial};
pub use dispatch::{LoopMessage, LoopOrigin, LoopPayload, MessagePriority, TraceId};
pub use episodic::{
    EpisodicBudgetExceeded, EpisodicReadHandle, EpisodicWriteHandle, ExperienceClassification,
};
pub use inference::InferenceHandle;
pub use semantic::{SemanticReadHandle, SemanticWriteHandle};

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
        }
    }
}

// =============================================================================
// Loop cycle — sense → compare → compute → act
// =============================================================================

/// Afferent signal from a loop's sensing phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Signal {
    pub source: LoopId,
    pub metric: String,
    pub value: f64,
    pub set_point: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Signal {
    pub fn new(source: LoopId, metric: &str, value: f64, set_point: f64) -> Self {
        Self {
            source,
            metric: metric.to_string(),
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
    pub target: LoopId,
    pub action_type: ActionType,
    pub parameters: serde_json::Value,
    pub priority: MessagePriority,
}

impl LoopAction {
    pub fn new(target: LoopId, action_type: ActionType, parameters: serde_json::Value) -> Self {
        let priority = match &action_type {
            ActionType::Throttle => MessagePriority::Warning,
            ActionType::Escalate => MessagePriority::Critical,
            ActionType::Calibrate => MessagePriority::Info,
            ActionType::CircuitBreak => MessagePriority::Critical,
        };
        Self {
            target,
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
