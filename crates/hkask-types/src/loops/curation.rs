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

use crate::capability::tokens::ConsolidationToken;
use crate::id::WebID;
use crate::sovereignty::DataCategory;
use crate::visibility::Visibility;

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

    /// Curator can read everything EXCEPT private episodic memory
    pub fn can_read(&self, category: &DataCategory) -> bool {
        !matches!(category, DataCategory::EpisodicMemory)
    }

    /// Curator can write to shared and public categories that it governs
    pub fn can_write(&self, category: &DataCategory) -> bool {
        matches!(
            category.default_visibility(),
            Visibility::Shared | Visibility::Public
        ) && !matches!(category, DataCategory::HLexiconTerms)
    }

    /// Issue a ConsolidationToken authorizing an Episodic → Semantic bridge traversal.
    ///
    /// The Curator is Cybernetics' governor, so it can delegate consolidation
    /// authority. This token is required by `ConsolidationPort::consolidate()`.
    pub fn issue_consolidation_token(&self) -> ConsolidationToken {
        ConsolidationToken::new(self.curator_id)
    }
}

// =============================================================================
// CuratorDirective — Curation → Cybernetics directives
// =============================================================================

/// Directives the Curator issues to Cybernetics.
///
/// Per ARL IP-3: when the Curation Confidence Gate is in the transition zone
/// (0.3 < R̄ < 0.8), the regulated response is `SeekMoreEvidence`, which
/// is routed through Cybernetics to the Inference Loop.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuratorDirective {
    CalibrateThreshold {
        domain: String,
        new_threshold: u64,
    },
    UpdateCapabilities {
        agent: WebID,
        additions: Vec<String>,
        removals: Vec<String>,
    },
    AdjustEnergyBudget {
        agent: WebID,
        new_budget: u64,
    },
    /// IP-3: Confidence-gated metacognitive directive.
    /// The Curator requests additional evidence to increase confidence
    /// in a pending decision. Routed through Cybernetics to Inference.
    SeekMoreEvidence {
        /// The decision context requiring more evidence.
        context: String,
        /// Which evidence channel to verify (from sensitivity analysis).
        /// E.g., "llm_confidence", "template_match", "validation_result".
        channel: String,
        /// Current R̄ from the Curation Confidence Gate.
        confidence: String,
    },
}
