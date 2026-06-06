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

// CuratorHandle — Loop 5 capability handle

/// The Curator's capability handle. Single replicant — the user's
/// counterpart in `kask chat`. Can read all loop state and write
/// governance/observability policy.
///
/// **Singleton invariant:** There is exactly one Curator per hKask system.
/// `CurationLoop` owns the single `CuratorHandle` instance; all other code
/// accesses it through `CuratorContext::handle()`. Construct via
/// `CuratorHandle::system()` — the `new(WebID)` constructor is `pub(crate)`
/// to prevent external callers from creating additional handles.
#[derive(Clone)]
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

    /// Create the system CuratorHandle using the system WebID.
    ///
    /// The Curator is a singleton — the user's counterpart in `kask chat`.
    /// This constructor enforces that convention by deriving the ID from
    /// the "curator" persona.
    pub fn system() -> Self {
        Self {
            curator_id: WebID::from_persona(b"curator"),
        }
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
    /// authority. This token is required by `ConsolidationBridge::consolidate()`.
    pub fn issue_consolidation_token(&self) -> ConsolidationToken {
        ConsolidationToken::new(self.curator_id)
    }
}

// CuratorDirective — Curation → Cybernetics directives

/// Directives the Curator issues to Cybernetics.
///
/// Per ARL IP-3: when the Curation Confidence Gate is in the transition zone
/// (0.3 < R̄ < 0.8), the regulated response is `SeekMoreEvidence`, which
/// is routed through Cybernetics to the Inference Loop.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Override gas budget beyond Cybernetics set-points.
    ///
    /// This is the Curation-level metacognitive override. Cybernetics
    /// uses `ActionType::AdjustGasBudget` for automatic within-bounds
    /// regulation. Curation uses `OverrideGasBudget` to exceed bounds.
    OverrideGasBudget {
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
    /// Replenish an agent's gas budget by a specific amount.
    ///
    /// Used when an agent has exhausted its budget but Curation
    /// determines it should continue operating. This is the Curator's
    /// ability to inject gas into the system, analogous to Ethereum's
    /// gas refund mechanism but governed by human/curator authority.
    ReplenishBudget {
        agent: WebID,
        amount: u64,
        /// Priority weight for replenishment scaling (0.0–1.0).
        /// When present, Cybernetics scales the replenishment amount by this priority.
        /// Defaults to 1.0 (full replenishment).
        priority: Option<f64>,
    },
    /// Clear a Curation override on an agent's gas budget.
    ///
    /// Removes the agent from the active-overrides registry so that
    /// normal replenishment resumes. This is the inverse of `OverrideGasBudget`.
    ClearOverride {
        agent: WebID,
    },
}

impl CuratorDirective {
    /// Returns the snake_case variant name for logging and fingerprinting.
    pub fn variant_name(&self) -> &'static str {
        match self {
            CuratorDirective::CalibrateThreshold { .. } => "calibrate_threshold",
            CuratorDirective::UpdateCapabilities { .. } => "update_capabilities",
            CuratorDirective::OverrideGasBudget { .. } => "override_gas_budget",
            CuratorDirective::SeekMoreEvidence { .. } => "seek_more_evidence",
            CuratorDirective::ReplenishBudget { .. } => "replenish_budget",
            CuratorDirective::ClearOverride { .. } => "clear_override",
        }
    }

    /// Returns the agent targeted by this directive, if applicable.
    ///
    /// Directives that target a domain rather than an agent
    /// (e.g., `CalibrateThreshold`, `SeekMoreEvidence`) return `None`.
    pub fn agent_target(&self) -> Option<WebID> {
        match self {
            CuratorDirective::CalibrateThreshold { .. } => None,
            CuratorDirective::UpdateCapabilities { agent, .. } => Some(*agent),
            CuratorDirective::OverrideGasBudget { agent, .. } => Some(*agent),
            CuratorDirective::SeekMoreEvidence { .. } => None,
            CuratorDirective::ReplenishBudget { agent, .. } => Some(*agent),
            CuratorDirective::ClearOverride { agent } => Some(*agent),
        }
    }

    /// Whether this directive is a metacognitive override.
    ///
    /// Metacognitive overrides are higher-order Curation interventions that
    /// reconfigure Cybernetics regulation itself. They are subject to the
    /// override cooldown in addition to per-fingerprint dedup, because
    /// override oscillation is especially destabilizing.
    pub fn is_metacognitive(&self) -> bool {
        matches!(
            self,
            CuratorDirective::OverrideGasBudget { .. } | CuratorDirective::SeekMoreEvidence { .. }
        )
    }
}
