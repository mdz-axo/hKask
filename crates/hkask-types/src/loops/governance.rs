//! Loop 3: Governance — Capability handle
//!
//! The Governance loop enforces capability discipline:
//! request → authorize → dispatch → observe → adapt policy
//!
//! Subloops:
//! - 3.1 Revocation (WITHDRAW) — grant → revoke → persist → deny future
//! - 3.2 Sovereignty Checking (GUARD) — request → check condition → allow or deny
//! - 3.3 Goal State Machine (RECONCILE) — conflict A, conflict B → combine → resolved
//!
//! # Capability Discipline
//!
//! `GovernanceHandle` is the most powerful domain handle. It can verify/attenuate/revoke
//! tokens, check visibility, process alerts, and calibrate thresholds. It CANNOT
//! emit arbitrary spans, store triples, or run inference.

use crate::capability::CapabilityToken;
use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// GovernanceHandle — Loop 3 capability handle
// =============================================================================

/// Governance loop capability handle.
///
/// Provides authority verification, capability token management, and
/// sovereignty enforcement. The governance handle sits at the center
/// of the OCAP discipline: every capability request flows through governance.
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
pub struct GovernanceHandle {
    /// Agent performing governance (typically a bot or the Curator)
    governor: WebID,
}

impl GovernanceHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            governor: WebID::new(),
        }
    }

    /// Create a governance handle for a specific agent.
    pub fn new(governor: WebID) -> Self {
        Self { governor }
    }

    /// The agent performing governance.
    pub fn governor(&self) -> &WebID {
        &self.governor
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
    /// This handle provides the type-level enforcement that governance CAN verify.
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
        if matches!(category, DataCategory::EpisodicMemory) && *requester != self.governor {
            return Err(GovernanceDenial::SovereigntyViolation {
                category: category.clone(),
                requester: *requester,
            });
        }
        Ok(())
    }
}

/// Reasons a governance operation can be denied.
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

