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

use crate::capability::tokens::CyberneticsToken;
use crate::id::WebID;

// =============================================================================
// CyberneticsHandle — Loop 6 capability handle
// =============================================================================

/// The Cybernetics Loop's capability handle.
///
/// Only the Cybernetics Loop (`hkask_cns::CyberneticsLoop`) holds this handle.
/// It authorizes energy budget operations, circuit breaking, dampening,
/// throttling, and algedonic alert escalation.
///
/// **Singleton invariant:** There is exactly one CyberneticsHandle per hKask system.
/// `CyberneticsLoop` owns the single instance.
#[derive(Clone)]
pub struct CyberneticsHandle {
    cybernetics_id: WebID,
}

impl CyberneticsHandle {
    /// Create the system CyberneticsHandle using the system WebID.
    ///
    /// The Cybernetics Loop is a singleton — the homeostatic regulator.
    /// This constructor enforces that convention by deriving the ID from
    /// the "cybernetics" persona.
    pub fn system() -> Self {
        Self {
            cybernetics_id: WebID::from_persona(b"cybernetics"),
        }
    }

    /// The WebID of the Cybernetics Loop.
    pub fn cybernetics_id(&self) -> &WebID {
        &self.cybernetics_id
    }

    /// Issue a CyberneticsToken authorizing operations governed by
    /// the Cybernetics Loop.
    ///
    /// Only the Cybernetics Loop (or its governor, the Curator) should
    /// call this. The token proves that an operation was authorized by
    /// the homeostatic regulator.
    pub fn issue_cybernetics_token(&self) -> CyberneticsToken {
        CyberneticsToken::new(self.cybernetics_id)
    }
}
