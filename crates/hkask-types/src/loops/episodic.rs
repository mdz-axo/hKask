//! Loop 2a: Episodic Memory — Capability handles
//
//! The Episodic Memory loop governs private, agent-scoped experience:
//! experience → encode → store (private) → recall → temporal attention → context
//!
//! Subloops:
//! - 2a.1 Experience Encoding (FILTER) — filter and classify incoming experience
//! - 2a.2 Temporal Attention (ADAPT) — weight by recency: weight = e^(-λ × time_since_storage)
//! - 2a.3 Confidence Decay (RECONCILE) — confidence decreases over time via Bayesian decay
//! - 2a.4 Confidence Retraction (RECONCILE) — reduce confidence without deleting the triple
//! - 2a.5 Episodic Storage Budget (GUARD) — per-agent storage limit, mark oldest for consolidation
//! - 2a.6 Episodic Context Assembly (FILTER+ADAPT) — temporal-ordered, recency-weighted, budget-constrained
//!
//! # Capability Discipline
//
//! Episodic memory is PRIVATE to the agent. Only the owning agent can store or read
//! their own episodic triples. This is enforced by the type system:
//!
//! - `EpisodicReadHandle` can query visible episodic triples for own perspective and
//!   assemble episodic context. It CANNOT store triples, access other agents' episodic
//!   memories, or query by similarity (use `SemanticReadHandle` for that).
//!
//! - `EpisodicWriteHandle` can store episodic triples for own WebID only.
//!   It CANNOT delete triples, write on behalf of other agents, or write semantic triples.

// =============================================================================
// Experience Classification (Loop 2a.1 — Experience Encoding)
// =============================================================================

/// Classification of an episodic experience for encoding (Loop 2a.1).
///
/// Each classification carries a default confidence that informs the initial
/// confidence of the stored triple. These defaults can be overridden by the
/// caller via `store_episodic_experience()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceClassification {
    /// A successful action or outcome. Default confidence: 0.9
    Success,
    /// A failed action or negative outcome. Default confidence: 0.3
    Failure,
    /// An observed fact or state. Default confidence: 0.7
    Observation,
    /// An inferred conclusion. Default confidence: 0.5
    Inference,
    /// A user-provided instruction or correction. Default confidence: 0.8
    Instruction,
}

impl ExperienceClassification {
    /// Default confidence for this experience classification.
    ///
    /// These values are used when no explicit confidence is provided
    /// in `store_episodic_experience()`.
    pub fn default_confidence(&self) -> f64 {
        match self {
            ExperienceClassification::Success => 0.9,
            ExperienceClassification::Failure => 0.3,
            ExperienceClassification::Observation => 0.7,
            ExperienceClassification::Inference => 0.5,
            ExperienceClassification::Instruction => 0.8,
        }
    }
}

impl std::fmt::Display for ExperienceClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExperienceClassification::Success => write!(f, "success"),
            ExperienceClassification::Failure => write!(f, "failure"),
            ExperienceClassification::Observation => write!(f, "observation"),
            ExperienceClassification::Inference => write!(f, "inference"),
            ExperienceClassification::Instruction => write!(f, "instruction"),
        }
    }
}

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// EpisodicReadHandle — Loop 2a read access
// =============================================================================

/// Episodic memory read handle.
///
/// Provides read-only access to an agent's own episodic memory. Enforces
/// agent-scoped visibility: the handle is bound to a single `WebID` and
/// can only read triples owned by that agent.
///
/// # OCAP Boundaries (Hoare triples: requires → ensures)
///
/// - **CAN** query visible episodic triples for own perspective
/// - **CAN** assemble episodic context (temporal-ordered, recency-weighted)
/// - **CANNOT** store triples (use `EpisodicWriteHandle`)
/// - **CANNOT** access other agents' episodic memories
/// - **CANNOT** query by similarity (use `SemanticReadHandle`)
/// - **CANNOT** delete triples (use `GovernanceHandle` with explicit revocation)
pub struct EpisodicReadHandle {
    /// Agent whose episodic memory this handle can read
    owner: WebID,
    /// Maximum number of triples to return in a single query (budget enforcement)
    query_budget: u32,
}

impl EpisodicReadHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            owner: WebID::new(),
            query_budget: 100,
        }
    }

    /// Create an episodic read handle for a specific agent.
    pub fn new(owner: WebID, query_budget: u32) -> Self {
        Self {
            owner,
            query_budget,
        }
    }

    /// The agent whose episodic memory this handle can read.
    pub fn owner(&self) -> &WebID {
        &self.owner
    }

    /// Maximum number of triples returnable in a single query.
    pub fn query_budget(&self) -> u32 {
        self.query_budget
    }

    /// Check if this handle can read data from the given category.
    ///
    /// Episodic read handles can only access `EpisodicMemory` data.
    pub fn can_access(&self, category: &DataCategory) -> bool {
        matches!(category, DataCategory::EpisodicMemory)
    }
}

// =============================================================================
// EpisodicWriteHandle — Loop 2a write access
// =============================================================================

/// Episodic memory write handle.
///
/// Provides write access to an agent's own episodic memory. Enforces
/// agent-scoped write authority: the handle is bound to a single `WebID`
/// and can only store triples owned by that agent.
///
/// # OCAP Boundaries
///
/// - **CAN** store episodic triples (own WebID only)
/// - **CAN** retract episodic triples (reduce confidence, not delete)
/// - **CANNOT** delete triples (retraction reduces confidence to 0, but the triple persists)
/// - **CANNOT** write on behalf of other agents
/// - **CANNOT** write semantic triples (use `SemanticWriteHandle`)
/// - **CANNOT** read triples (use `EpisodicReadHandle`)
pub struct EpisodicWriteHandle {
    /// Agent whose episodic memory this handle can write to
    owner: WebID,
    /// Per-agent storage budget (max triples)
    storage_budget: u32,
    /// Current storage usage
    storage_used: u32,
}

impl EpisodicWriteHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            owner: WebID::new(),
            storage_budget: 10000,
            storage_used: 0,
        }
    }

    /// Create an episodic write handle for a specific agent.
    pub fn new(owner: WebID, storage_budget: u32, storage_used: u32) -> Self {
        Self {
            owner,
            storage_budget,
            storage_used,
        }
    }

    /// The agent whose episodic memory this handle can write to.
    pub fn owner(&self) -> &WebID {
        &self.owner
    }

    /// Per-agent storage budget (maximum triples).
    pub fn storage_budget(&self) -> u32 {
        self.storage_budget
    }

    /// Current storage usage (number of triples stored).
    pub fn storage_used(&self) -> u32 {
        self.storage_used
    }

    /// Check if storage budget allows storing additional triples.
    pub fn within_budget(&self, additional: u32) -> bool {
        self.storage_used + additional <= self.storage_budget
    }

    /// Record that `count` triples have been stored.
    ///
    /// # Requires
    /// - `storage_used + count` must not exceed `storage_budget`
    ///
    /// # Ensures
    /// - Increments `storage_used` by `count`
    pub fn record_stored(&mut self, count: u32) -> Result<(), EpisodicBudgetExceeded> {
        if !self.within_budget(count) {
            return Err(EpisodicBudgetExceeded {
                agent: self.owner,
                requested: self.storage_used + count,
                budget: self.storage_budget,
            });
        }
        self.storage_used += count;
        Ok(())
    }
}

/// Error returned when episodic storage budget is exceeded.
#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "episodic storage budget exceeded for agent {agent}: requested {requested}, budget {budget}"
)]
pub struct EpisodicBudgetExceeded {
    pub agent: WebID,
    pub requested: u32,
    pub budget: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn episodic_read_handle_new_test() {
        let handle = EpisodicReadHandle::new_test();
        assert!(handle.can_access(&DataCategory::EpisodicMemory));
        assert!(!handle.can_access(&DataCategory::SemanticMemory));
        assert!(!handle.can_access(&DataCategory::PersonalContext));
    }

    #[test]
    fn episodic_write_handle_within_budget() {
        let mut handle = EpisodicWriteHandle::new_test();
        assert!(handle.within_budget(100));
        assert!(handle.record_stored(50).is_ok());
        assert_eq!(handle.storage_used(), 50);
    }
}
