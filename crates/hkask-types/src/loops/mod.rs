//! hKask 8-Loop Architecture — Loop module structure
//!
//! hKask has 8 loops: 4 domain loops + 3 master loops, plus an inter-loop bridge.
//!
//! **Domain Loops:**
//! - Loop 1: Inference — prompt → context → model → response → parse → act
//! - Loop 2a: Episodic Memory — experience → encode → store (private) → recall → temporal attention → context
//! - Loop 2b: Semantic Memory — knowledge → store (public) → index → recall → dedup → combine → context
//! - Loop 3: Governance — request → authorize → dispatch → observe → adapt policy
//! - Loop 4: Observability — emit span → aggregate → detect anomaly → escalate
//!
//! **Master Loops:**
//! - Loop 5: Curation — observe → evaluate → compose → regulate (regulator — reads all, writes policy)
//! - Loop 6: Communication — send → observe delivery → detect congestion → dampen → confirm (connector)
//! - Loop 7: Cybernetics — sense (Observability) → compare → decide → act (Governance) → sense again (manages Loops 3+4)
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
pub mod governance;
pub mod inference;
pub mod observability;
pub mod semantic;

pub use curation::CuratorHandle;
pub use cybernetics::CyberneticHandle;
pub use dispatch::{LoopMessage, LoopOrigin, LoopPayload, MessagePriority, TraceId};
pub use episodic::{
    EpisodicBudgetExceeded, EpisodicReadHandle, EpisodicWriteHandle, ExperienceClassification,
};
pub use governance::GovernanceHandle;
pub use inference::{EnergyBudgetHandle, InferenceBudgetExceeded, InferenceHandle};
pub use semantic::{SemanticReadHandle, SemanticWriteHandle};

/// 9 Control Primitives
///
/// Every subloop is a domain-specific instance of one of these 9 abstract patterns.
/// The primitive is the pattern; the subloop is the instantiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlPrimitive {
    /// request → check condition → allow or deny
    Guard,
    /// stream → remove undesired → pass through
    Filter,
    /// request → hit? → return / miss → compute + store
    Cache,
    /// call → fail → count → threshold → open → half-open → probe → close
    Circuit,
    /// conflict A, conflict B → combine → resolved
    Reconcile,
    /// state → measure → signal
    Sense,
    /// signal → classify → deliver to consumer
    Route,
    /// grant → revoke → persist → deny future
    Withdraw,
    /// outcome → compare to desired → adjust parameter
    Adapt,
}

impl std::fmt::Display for ControlPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlPrimitive::Guard => write!(f, "GUARD"),
            ControlPrimitive::Filter => write!(f, "FILTER"),
            ControlPrimitive::Cache => write!(f, "CACHE"),
            ControlPrimitive::Circuit => write!(f, "CIRCUIT"),
            ControlPrimitive::Reconcile => write!(f, "RECONCILE"),
            ControlPrimitive::Sense => write!(f, "SENSE"),
            ControlPrimitive::Route => write!(f, "ROUTE"),
            ControlPrimitive::Withdraw => write!(f, "WITHDRAW"),
            ControlPrimitive::Adapt => write!(f, "ADAPT"),
        }
    }
}

/// Loop identifiers for the 8-loop model.
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
    /// Loop 3: Governance
    Governance,
    /// Loop 4: Observability (CNS)
    Observability,
    /// Loop 5: Curation (regulator)
    Curation,
    /// Loop 6: Communication (connector)
    Communication,
    /// Loop 7: Cybernetics (manages Observability→Governance feedback cycle)
    Cybernetics,
}

impl std::fmt::Display for LoopId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopId::Inference => write!(f, "inference"),
            LoopId::Episodic => write!(f, "episodic"),
            LoopId::Semantic => write!(f, "semantic"),
            LoopId::Governance => write!(f, "governance"),
            LoopId::Observability => write!(f, "observability"),
            LoopId::Curation => write!(f, "curation"),
            LoopId::Communication => write!(f, "communication"),
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
