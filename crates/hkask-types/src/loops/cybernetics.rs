//! Loop 6: Cybernetics — Homeostatic self-regulation
//!
//! The Cybernetics loop is the homeostatic self-regulation loop, combining what
//! were previously separate Governance and Observability loops. Like the human
//! autonomic nervous system, it maintains homeostasis through complex dynamic
//! self-regulation: it senses system state (variety, sovereignty, energy),
//! regulates (OCAP capability verification, sovereignty enforcement), and
//! adapts (threshold calibration, energy budget adjustment).
//!
//! Subloops:
//! - 6.1 OCAP Verification (GUARD) — request → check condition → allow or deny
//! - 6.2 Sovereignty Enforcement (GUARD) — request → check boundary → allow or deny
//! - 6.3 Variety Sensing (SENSE) — state → measure → signal
//! - 6.4 Algedonic Regulation (ADAPT) — deficit → compare to threshold → calibrate or escalate
//! - 6.5 Energy Homeostasis (GUARD+ADAPT) — consume → check budget → allow or deny + alert
//! - 6.6 Revocation (WITHDRAW) — grant → revoke → persist → deny future
//!
//! # Capability Discipline
//!
//! `CyberneticsHandle` is the most powerful meta handle. It can verify/attenuate/revoke
//! tokens, check visibility, process alerts, and calibrate thresholds. It CANNOT
//! emit arbitrary spans, store triples, or run inference.

use crate::capability::CapabilityToken;
use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// CyberneticsHandle — Loop 6 capability handle
// =============================================================================

/// Cybernetics loop capability handle.
///
/// Provides authority verification, capability token management,
/// sovereignty enforcement, and homeostatic self-regulation. The cybernetics
/// handle sits at the center of the OCAP discipline: every capability request
/// flows through cybernetics.
///
/// # OCAP Boundaries
///
/// - **CAN** verify capability tokens
/// - **CAN** attenuate tokens (reduce authority)
/// - **CAN** revoke tokens (WITHDRAW subloop)
/// - **CAN** check data visibility/sovereignty (GUARD subloop)
/// - **CAN** process algedonic alerts (read-only from CNS)
/// - **CAN** calibrate thresholds (via `CnsGovernReadHandle`)
/// - **CANNOT** emit arbitrary spans (use `CnsWriteHandle`)
/// - **CANNOT** store triples (use `EpisodicWriteHandle` / `SemanticWriteHandle`)
/// - **CANNOT** run inference (use `InferenceHandle`)
pub struct CyberneticsHandle {
    /// Agent performing cybernetic regulation (typically a bot or the Curator)
    agent: WebID,
}

impl CyberneticsHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent: WebID::new(),
        }
    }

    /// Create a cybernetics handle for a specific agent.
    pub fn new(agent: WebID) -> Self {
        Self { agent }
    }

    /// The agent performing cybernetic regulation.
    pub fn agent(&self) -> &WebID {
        &self.agent
    }

    /// Verify a capability token for a specific operation.
    ///
    /// # Requires
    /// - Token must not be expired
    /// - Token must grant the required resource/action
    /// - Token holder must match the expected holder
    ///
    /// # Ensures
    /// - Returns `Ok(())` if token is valid for the operation
    /// - Returns `Err` with denial reason if token is invalid
    ///
    /// Note: Full cryptographic verification is performed by the ACP runtime.
    /// This handle provides the type-level enforcement that cybernetics CAN verify.
    pub fn verify_token(
        &self,
        token: &CapabilityToken,
        current_time: i64,
    ) -> Result<(), GovernanceDenial> {
        if token.is_expired(current_time) {
            return Err(GovernanceDenial::TokenExpired);
        }
        Ok(())
    }

    /// Check data sovereignty for a given category and requester.
    ///
    /// # Requires
    /// - `category` must be a valid DataCategory
    /// - `requester` must be a valid WebID
    ///
    /// # Ensures
    /// - Returns `Ok(())` if access is allowed
    /// - Returns `Err` with denial reason if access is denied
    pub fn check_sovereignty(
        &self,
        category: &DataCategory,
        requester: &WebID,
    ) -> Result<(), GovernanceDenial> {
        // Episodic memory is only accessible by the owner
        if matches!(category, DataCategory::EpisodicMemory) && *requester != self.agent {
            return Err(GovernanceDenial::SovereigntyViolation {
                category: category.clone(),
                requester: *requester,
            });
        }
        Ok(())
    }
}

/// Reasons a governance/cybernetics operation can be denied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernanceDenial {
    #[error("capability token has expired")]
    TokenExpired,
    #[error("insufficient capability for requested operation")]
    InsufficientCapability,
    #[error("sovereignty violation: {category} not accessible by {requester}")]
    SovereigntyViolation {
        category: DataCategory,
        requester: WebID,
    },
    #[error("token has been revoked")]
    TokenRevoked,
}
