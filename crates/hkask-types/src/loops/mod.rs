//! hKask 6-Loop Architecture — Loop module structure
//!
//! hKask has 6 loops: 3 domain loops + 3 meta loops, plus an inter-loop bridge.
//!
//! **Domain Loops:**
//! - Loop 1: Inference — prompt → context → model → response → parse → act
//! - Loop 2a: Episodic Memory — experience → encode → store (private) → recall → temporal attention → context
//! - Loop 2b: Semantic Memory — knowledge → store (public) → index → recall → dedup → combine → context
//!
//! **Meta Loops:**
//! - Loop 4: Communication — send → observe delivery → detect congestion → dampen → confirm (connector)
//! - Loop 5: Curation/Metacognition — observe → evaluate → compose → regulate (regulator)
//! - Loop 6: Cybernetics — sense → regulate → adapt (homeostatic self-regulation)
//!
//! **Bridge:**
//! - 2a→2b: Consolidation — episodic → strip perspective → dedup → store semantic (one-way transformation)
//!
//! Each loop enforces capability discipline through typed handles. A handle's type determines
//! what operations are available: `EpisodicReadHandle` cannot call `store_episodic()` because
//! the method doesn't exist on that type. This is the strongest possible enforcement.

pub mod curation;
pub mod cybernetics;
pub mod dispatch;
pub mod episodic;
pub mod inference;
pub mod semantic;

pub use curation::{CurationAlertSignal, CurationRegulation, CuratorDirective, CuratorHandle};
pub use cybernetics::{CyberneticsHandle, CyberneticsRegulation, GovernanceDenial};
pub use dispatch::{
    CommunicationRegulation, LoopMessage, LoopOrigin, LoopPayload, MessagePriority, TraceId,
};
pub use episodic::{
    EpisodicBudgetExceeded, EpisodicReadHandle, EpisodicRegulation, EpisodicWriteHandle,
    ExperienceClassification,
};
pub use inference::{InferenceBudgetExceeded, InferenceHandle, InferenceRegulation};
pub use semantic::{SemanticReadHandle, SemanticRegulation, SemanticWriteHandle};

pub use self::Loop as HkaskLoop;

/// Loop identifiers for the 6-loop model.
///
/// Used in message routing, span tagging, and subloop mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopId {
    /// Loop 1: Inference
    Inference,
    /// Loop 2a: Episodic Memory
    Episodic,
    /// Loop 2b: Semantic Memory
    Semantic,
    /// Loop 4: Communication (meta)
    Communication,
    /// Loop 5: Curation/Metacognition (meta)
    Curation,
    /// Loop 6: Cybernetics (meta)
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

/// Data visibility tier for HKDF key derivation mapping.
///
/// Each `DataCategory` maps to a visibility tier, which determines
/// the HKDF derivation context for encryption key derivation.
/// The tier also governs which handles can access the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataVisibilityTier {
    /// Universal access, no encryption key needed
    Public,
    /// Capability-gated access, shared encryption key per capability group
    Shared,
    /// Owner-only access, per-agent HKDF-derived encryption key
    Private,
}

impl DataVisibilityTier {
    /// Map a DataCategory to its visibility tier for HKDF key derivation.
    ///
    /// This mapping drives encryption key selection:
    /// - `Public` → no encryption, plaintext storage
    /// - `Shared` → HKDF context `hkask:shared:<category>`, group key
    /// - `Private` → HKDF context `hkask:private:<category>:<agent_webid>`, per-agent key
    pub fn from_data_category(category: &crate::sovereignty::DataCategory) -> Self {
        if category.is_typically_public() {
            DataVisibilityTier::Public
        } else if category.is_typically_shared() {
            DataVisibilityTier::Shared
        } else {
            DataVisibilityTier::Private
        }
    }

    /// HKDF derivation context for this visibility tier and data category.
    ///
    /// Used by `hkask-keystore` to derive per-tier encryption keys.
    /// Append `:<agent_webid>` for Private tier to get per-agent keys.
    pub fn derivation_context(&self, category: &crate::sovereignty::DataCategory) -> String {
        match self {
            DataVisibilityTier::Public => format!("hkask:public:{}", category.as_str()),
            DataVisibilityTier::Shared => format!("hkask:shared:{}", category.as_str()),
            DataVisibilityTier::Private => format!("hkask:private:{}", category.as_str()),
        }
    }

    /// Whether data at this tier requires encryption at rest.
    pub fn requires_encryption(&self) -> bool {
        !matches!(self, DataVisibilityTier::Public)
    }
}

impl std::fmt::Display for DataVisibilityTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataVisibilityTier::Public => write!(f, "public"),
            DataVisibilityTier::Shared => write!(f, "shared"),
            DataVisibilityTier::Private => write!(f, "private"),
        }
    }
}

// =============================================================================
// Loop Trait — sense → compare → compute → act
// =============================================================================

/// Afferent signal from a loop's sensing phase.
///
/// Each signal carries the loop's identity, the metric being observed,
/// the current value, and the reference set-point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Signal {
    /// Which loop produced this signal
    pub source: LoopId,
    /// Metric name (e.g., "energy_remaining", "variety_deficit", "error_rate")
    pub metric: String,
    /// Current observed value
    pub value: f64,
    /// Reference set-point (desired value)
    pub set_point: f64,
    /// Timestamp of observation
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Signal {
    /// Create a new afferent signal.
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
    /// The signal that triggered this deviation
    pub signal: Signal,
    /// Magnitude of deviation (always positive)
    pub magnitude: f64,
    /// Direction of deviation (above or below set-point)
    pub direction: DeviationDirection,
}

impl Deviation {
    /// Create a new deviation from a signal.
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
    /// Observed value exceeds the set-point
    AboveSetPoint,
    /// Observed value is below the set-point
    BelowSetPoint,
}

/// Efferent action produced by a loop's compute phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopAction {
    /// Which loop this action targets
    pub target: LoopId,
    /// Type of regulatory action
    pub action_type: ActionType,
    /// Parameters for the action
    pub parameters: serde_json::Value,
    /// Priority of this action
    pub priority: MessagePriority,
}

impl LoopAction {
    /// Create a new loop action.
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
/// Every loop in the 6-loop architecture implements this trait.
/// The cycle is recursive: after `act`, the loop returns to `sense`.
/// The Cybernetics Loop's `act` calls `regulate` on other loops
/// through capability-restricted references — no loop holds a
/// reference to a loop it does not govern (Mark Miller's principle:
/// authority flows downward, never sideways).
pub trait Loop: Send + Sync {
    /// The loop's identity.
    fn id(&self) -> LoopId;

    /// Sense — collect afferent signals from the loop's domain.
    fn sense(&self) -> Vec<Signal>;

    /// Compare — evaluate signals against homeostatic set-points.
    fn compare(&self, signals: &[Signal]) -> Vec<Deviation>;

    /// Compute — produce efferent actions from deviations.
    fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction>;

    /// Act — dispatch efferent actions to target loops.
    fn act(&self, actions: &[LoopAction]);

    /// Execute one full sense→compare→compute→act cycle.
    fn tick(&self) {
        let signals = self.sense();
        let deviations = self.compare(&signals);
        let actions = self.compute(&deviations);
        self.act(&actions);
    }
}

/// Regulation interface — entry point for meta-loop governance.
///
/// Each loop exposes `regulate` so that its governing meta loop can
/// dispatch efferent signals without violating capability boundaries.
/// The `LoopAction` carries the authority granted by the regulator.
pub trait Regulatable: Loop {
    /// Receive a regulatory signal from a governing meta loop.
    ///
    /// The implementor MUST verify that the signal's `target` matches
    /// its own `LoopId` and that the `action_type` is within the
    /// regulator's authority.
    fn regulate(&self, action: &LoopAction);
}
