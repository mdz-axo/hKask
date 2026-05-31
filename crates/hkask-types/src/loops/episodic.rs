//! Loop 2a: Episodic Memory — private, agent-scoped experience
//!
//! experience → encode → store (private) → recall → temporal weight → context
//!
//! Essential subloops:
//! - 2a.1 Experience Encoding (FILTER) — filter and classify incoming experience
//! - 2a.2 Temporal Attention (ADAPT) — weight by recency: e^(-λ × time_since_storage)
//! - 2a.3 Confidence Decay (RECONCILE) — confidence decreases over time
//! - 2a.4 Confidence Retraction (RECONCILE) — reduce confidence without deleting
//!
//! Cybernetics regulation: storage budget adjustment
//!
//! Episodic memory is PRIVATE to the agent. Only the owning agent can
//! store or read their own episodic triples.

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// Experience Classification (Loop 2a.1)
// =============================================================================

/// Classification of an episodic experience for encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceClassification {
    Success,
    Failure,
    Observation,
    Inference,
    Instruction,
}

impl ExperienceClassification {
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

// =============================================================================
// EpisodicReadHandle — Loop 2a read access
// =============================================================================

/// Episodic memory read handle. Bound to a single WebID.
/// Can only read triples owned by that agent.
pub struct EpisodicReadHandle {
    owner: WebID,
    query_budget: u32,
}

impl EpisodicReadHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            owner: WebID::new(),
            query_budget: 100,
        }
    }

    pub fn new(owner: WebID, query_budget: u32) -> Self {
        Self {
            owner,
            query_budget,
        }
    }

    pub fn owner(&self) -> &WebID {
        &self.owner
    }

    pub fn query_budget(&self) -> u32 {
        self.query_budget
    }

    pub fn can_access(&self, category: &DataCategory) -> bool {
        matches!(category, DataCategory::EpisodicMemory)
    }
}

// =============================================================================
// EpisodicWriteHandle — Loop 2a write access
// =============================================================================

/// Episodic memory write handle. Bound to a single WebID.
/// Can only store triples owned by that agent.
pub struct EpisodicWriteHandle {
    owner: WebID,
    storage_budget: u32,
    storage_used: u32,
}

impl EpisodicWriteHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            owner: WebID::new(),
            storage_budget: 10000,
            storage_used: 0,
        }
    }

    pub fn new(owner: WebID, storage_budget: u32, storage_used: u32) -> Self {
        Self {
            owner,
            storage_budget,
            storage_used,
        }
    }

    pub fn owner(&self) -> &WebID {
        &self.owner
    }

    pub fn storage_budget(&self) -> u32 {
        self.storage_budget
    }

    pub fn storage_used(&self) -> u32 {
        self.storage_used
    }

    pub fn within_budget(&self, additional: u32) -> bool {
        self.storage_used + additional <= self.storage_budget
    }

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
