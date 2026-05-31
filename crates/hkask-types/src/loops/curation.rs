//! Loop 5: Curation — metacognitive observer
//!
//! observe → evaluate → compose → regulate
//!
//! The Curator is the ONLY loop that can override Cybernetics.
//! It observes system state and intervenes when Cybernetics
//! can't self-stabilize (e.g., alert cascade).
//!
//! Essential subloops:
//! - 5.1 Escalation Routing (ROUTE) — signal → classify → deliver to consumer
//! - 5.2 Metacognitive Adaptation (ADAPT) — outcome → compare to desired → adjust parameter

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// CuratorHandle — Loop 5 capability handle
// =============================================================================

/// The Curator's capability handle. Single replicant — the user's
/// counterpart in `kask chat`. Can read all loop state and write
/// governance/observability policy.
pub struct CuratorHandle {
    curator_id: WebID,
}

impl CuratorHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            curator_id: WebID::new(),
        }
    }

    pub fn new(curator_id: WebID) -> Self {
        Self { curator_id }
    }

    pub fn curator_id(&self) -> &WebID {
        &self.curator_id
    }

    /// The Curator can read everything EXCEPT private episodic memory.
    pub fn can_read(&self, category: &DataCategory) -> bool {
        !matches!(category, DataCategory::EpisodicMemory)
    }

    /// The Curator can write to semantic memory, governance, and observability policy.
    pub fn can_write(&self, category: &DataCategory) -> bool {
        matches!(
            category,
            DataCategory::SemanticMemory
                | DataCategory::OcapBoundaries
                | DataCategory::TemplateInvocations
        )
    }
}

// =============================================================================
// CuratorDirective — Curation → Cybernetics directives
// =============================================================================

/// Directives the Curator issues to Cybernetics.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuratorDirective {
    CalibrateThreshold { domain: String, new_threshold: u64 },
    UpdateCapabilities {
        agent: WebID,
        additions: Vec<String>,
        removals: Vec<String>,
    },
    AdjustEnergyBudget { agent: WebID, new_budget: u64 },
}
