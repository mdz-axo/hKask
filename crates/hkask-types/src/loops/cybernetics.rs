//! Loop 7: Cybernetics — The third master loop
//!
//! The Cybernetic loop manages the Observability (4) and Governance (3) loops,
//! closing the feedback cycle between sensing and acting:
//!
//!   sense (Observability) → compare → decide → act (Governance) → sense again
//!
//! Without the Cybernetic loop, Observability detects anomalies but cannot
//! decide what to do about them, and Governance has no signal to act on.
//! The loop exists because cybernetics is not observability — it is the
//! *entire* feedback cycle, from signal through adaptation.
//!
//! # Subloops
//!
//! - 7.1 Variety Regulation (SENSE+ADAPT) — variety deficit → algedonic alert → threshold calibration → observe effect
//! - 7.2 Bot Health Regulation (SENSE+ADAPT) — bot metrics → evaluate → issue directive → observe improvement
//! - 7.3 Anti-Oscillation (FILTER+RECONCILE) — dampen repeated directives within time window
//!
//! # Relationship to other loops
//!
//! The Cybernetic loop is a master loop that manages two domain loops:
//! - **Observability** (Loop 4) — the sensing half, lives in `hkask-cns`
//! - **Governance** (Loop 3) — the acting half, lives in `hkask-agents`
//!
//! The Curation loop (5) provides the decision-making agent (the Curator)
//! that reads Observability state and writes Governance policy. The
//! Communication loop (6) provides the inter-loop messaging (DISPATCH, DAMPEN).
//!
//! Loop 7 is the *formal closure* of the Observability→Governance feedback
//! cycle. It does not introduce new runtime components — it names the
//! cycle that already exists.
//!
//! # Capability Discipline
//!
//! `CyberneticHandle` provides read access to CNS state and write access to
//! regulation policy (threshold calibration, directives). It CANNOT:
//! - Run inference directly (must delegate to Loop 1)
//! - Emit spans directly (must use CNS write handle)
//! - Access private episodic triples (sovereignty boundary)

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// CyberneticHandle — Loop 7 capability handle
// =============================================================================

/// Cybernetic loop capability handle.
///
/// Provides cross-loop read access to CNS sensing state (Loop 4) and
/// write access to Governance regulation policy (Loop 3). This handle
/// formalizes the feedback cycle that the Cybernetic loop manages.
///
/// # OCAP Boundaries
///
/// - **CAN** read CNS health, variety counters, and alerts
/// - **CAN** calibrate CNS thresholds (ADAPT subloop)
/// - **CAN** issue CuratorDirectives (ADAPT subloop)
/// - **CAN** submit to escalation queue (ROUTE subloop)
/// - **CANNOT** run inference (must delegate via `InferenceHandle`)
/// - **CANNOT** emit spans directly (must use `CnsWriteHandle`)
/// - **CANNOT** access private episodic triples (sovereignty boundary)
pub struct CyberneticHandle {
    /// The agent performing cybernetic regulation (typically the Curator)
    regulator: WebID,
}

impl CyberneticHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            regulator: WebID::new(),
        }
    }

    /// Create a cybernetic handle for a specific regulator agent.
    pub fn new(regulator: WebID) -> Self {
        Self { regulator }
    }

    /// The agent performing cybernetic regulation.
    pub fn regulator(&self) -> &WebID {
        &self.regulator
    }

    /// Check if the regulator can read data in the given category.
    ///
    /// The Cybernetic loop can read everything EXCEPT private episodic memory
    /// (same boundary as the Curator).
    pub fn can_read(&self, category: &DataCategory) -> bool {
        !matches!(category, DataCategory::EpisodicMemory)
    }

    /// Check if the regulator can write data in the given category.
    ///
    /// The Cybernetic loop can write to observability policy (thresholds),
    /// governance policy (capabilities), and semantic memory.
    pub fn can_write(&self, category: &DataCategory) -> bool {
        matches!(
            category,
            DataCategory::SemanticMemory
                | DataCategory::OcapBoundaries
                | DataCategory::TemplateInvocations
        )
    }
}
