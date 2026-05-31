//! Loop 6: Cybernetics — homeostatic self-regulation
//!
//! sense → regulate → adapt
//!
//! The Cybernetics loop senses system state (variety, sovereignty, energy),
//! regulates (OCAP verification, sovereignty enforcement, energy throttling,
//! circuit-breaking, dampening), and adapts (threshold calibration,
//! energy budget adjustment).
//!
//! Essential subloops:
//! - 6.1 Access Guard (GUARD) — request → check condition → allow or deny
//! - 6.3 Variety Sensing (SENSE) — state → measure → signal
//! - 6.4 Algedonic Regulation (ADAPT) — deficit → compare to threshold → calibrate or escalate
//! - 6.6 Revocation (WITHDRAW) — grant → revoke → persist → deny future
//!
//! Regulation functions (part of the regulate phase, not separate subloop cycles):
//! - Energy homeostasis — core Cybernetics regulation (thermodynamic resource allocation)
//! - Circuit breaking — stopping cascading failure is regulation
//! - Dampening — preventing oscillation in feedback loops is regulation
//! - Channel throttling — applied TO Communication channels, not Communication's own intelligence

use crate::capability::CapabilityToken;
use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// CyberneticsHandle — Loop 6 capability handle
// =============================================================================

/// Cybernetics loop capability handle. Verifies tokens, enforces
/// sovereignty, processes alerts, calibrates thresholds.
pub struct CyberneticsHandle {
    agent: WebID,
}

impl CyberneticsHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent: WebID::new(),
        }
    }

    pub fn new(agent: WebID) -> Self {
        Self { agent }
    }

    pub fn agent(&self) -> &WebID {
        &self.agent
    }

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

    pub fn check_sovereignty(
        &self,
        category: &DataCategory,
        requester: &WebID,
    ) -> Result<(), GovernanceDenial> {
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
