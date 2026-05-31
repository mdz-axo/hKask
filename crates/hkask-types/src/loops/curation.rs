//! Loop 5: Curation — Capability handle
//!
//! The Curation loop is the system regulator. It reads all loop state
//! and writes governance and observability policy:
//! observe → evaluate → compose → regulate
//!
//! Essential subloops:
//! - 5.1 Escalation Routing (ROUTE) — signal → classify → deliver to consumer
//! - 5.2 Metacognitive Adaptation (ADAPT) — outcome → compare to desired → adjust parameter
//!   (merges Bot Evaluation + Threshold Calibration — both are "outcome → compare → adjust")
//!
//! # Capability Discipline
//!
//! `CuratorHandle` is the most privileged handle in the system. The Curator
//! is a single replicant that serves as the user's counterpart in `kask chat`.
//! It can read all loop state and write governance/observability policy.
//!
//! However, even the Curator CANNOT:
//! - Run inference directly (must delegate to inference loop)
//! - Emit spans directly (must use `CnsWriteHandle`)
//! - Access private episodic triples (sovereignty boundary)

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// CuratorHandle — Loop 5 capability handle
// =============================================================================

/// Curation loop capability handle.
///
/// The Curator is the system's single replicant that reads all loop state
/// and writes governance/observability policy. This handle provides
/// cross-loop read access and policy write access.
///
/// # OCAP Boundaries
///
/// - **CAN** read all loop state (inference, memory, governance, observability)
/// - **CAN** write governance policy (escalation directives, capability updates)
/// - **CAN** write observability policy (threshold calibration, expected variety)
/// - **CAN** issue directives to Governance (CalibrateThreshold, UpdateCapabilities)
/// - **CAN** write to semantic memory (consolidation, coaching results)
/// - **CANNOT** run inference (must delegate via `InferenceHandle`)
/// - **CANNOT** emit spans directly (must use `CnsWriteHandle`)
/// - **CANNOT** access private episodic triples (sovereignty boundary)
pub struct CuratorHandle {
    /// The Curator's unique WebID (system singleton)
    curator_id: WebID,
}

impl CuratorHandle {
    /// Create a test handle with the system Curator ID.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            curator_id: WebID::new(),
        }
    }

    /// Create a Curator handle with the given WebID.
    pub fn new(curator_id: WebID) -> Self {
        Self { curator_id }
    }

    /// The Curator's WebID.
    pub fn curator_id(&self) -> &WebID {
        &self.curator_id
    }

    /// Check if the Curator can read data in the given category.
    ///
    /// The Curator can read everything EXCEPT private episodic memory.
    pub fn can_read(&self, category: &DataCategory) -> bool {
        // The Curator can read everything except private episodic memory
        !matches!(category, DataCategory::EpisodicMemory)
    }

    /// Check if the Curator can write data in the given category.
    ///
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

/// Directive types the Curator can issue to Governance.
///
/// These are the policy interventions the Curator can make:
/// - CalibrateThreshold: Adjust a CNS alert threshold
/// - UpdateCapabilities: Modify an agent's capability boundaries
/// - AdjustEnergyBudget: Change an agent's energy budget
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuratorDirective {
    /// Adjust a CNS alert threshold (e.g., variety deficit threshold)
    CalibrateThreshold { domain: String, new_threshold: u64 },
    /// Modify an agent's capability boundaries
    UpdateCapabilities {
        agent: WebID,
        additions: Vec<String>,
        removals: Vec<String>,
    },
    /// Adjust an agent's energy budget
    AdjustEnergyBudget { agent: WebID, new_budget: u64 },
}

// =============================================================================
// CurationRegulation — Loop 5 regulation interface
// =============================================================================

/// Regulation interface for the Curation/Metacognition Loop.
///
/// The Cybernetics Loop signals algedonic alerts to Curation.
/// Curation is the ONLY loop that can regulate the Cybernetics Loop
/// (metacognitive override).
pub trait CurationRegulation: Send + Sync {
    /// Receive an algedonic alert escalation from the Cybernetics Loop.
    fn receive_alert(&self, alert: &CurationAlertSignal);

    /// Metacognitive override — the Curation Loop intervenes when
    /// the Cybernetics Loop becomes unstable (e.g., alert cascade).
    fn metacognitive_override(&self, target: crate::loops::LoopId, reason: &str);
}

/// Signal from Cybernetics to Curation carrying an algedonic alert.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurationAlertSignal {
    /// Domain where the deficit was detected
    pub domain: String,
    /// Current deficit value
    pub deficit: u64,
    /// Threshold that was exceeded
    pub threshold: u64,
    /// Recommended action
    pub recommendation: String,
}
