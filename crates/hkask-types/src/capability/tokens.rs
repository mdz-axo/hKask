//! Capability boundary tokens — OCAP authority in the type system
//!
//! Per Mark Miller's OCAP discipline: authority flows downward through the
//! loop hierarchy. These tokens prove that the holder has authority from
//! the correct loop.
//!
//! Each token can only be constructed by the loop that governs it — private
//! fields prevent forgery. The module path IS the loop assignment.

use crate::id::WebID;

/// Token proving that a consolidation (Episodic → Semantic) operation
/// was authorized.
///
/// Only the Curation Loop (`hkask_agents::CurationLoop`) can mint this token.
/// It authorizes consolidation from Episodic to Semantic memory.
///
/// Required by: ConsolidationBridge::consolidate()
/// Issued by: CyberneticsLoop (or Curator as Cybernetics' governor)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ConsolidationToken {
    /// The agent whose consolidation this authorizes
    issuer: WebID,
}

/// Verification protocol for OCAP tokens.
///
/// Per Mark Miller's OCAP discipline: authority must flow through
/// designated channels. Tokens prove authority; verifiers confirm
/// the issuer is the expected principal.
pub trait IssuerVerification {
    /// The expected issuer for this token type.
    fn expected_issuer() -> WebID;

    /// Verify that this token was issued by the expected principal.
    fn verify_issuer(&self) -> bool {
        self.issuer() == &Self::expected_issuer()
    }

    /// The issuer of this token.
    fn issuer(&self) -> &WebID;
}

impl ConsolidationToken {
    pub(crate) fn new(issuer: WebID) -> Self {
        Self { issuer }
    }
}

impl IssuerVerification for ConsolidationToken {
    fn expected_issuer() -> WebID {
        WebID::from_persona(b"curator")
    }

    fn issuer(&self) -> &WebID {
        &self.issuer
    }
}
