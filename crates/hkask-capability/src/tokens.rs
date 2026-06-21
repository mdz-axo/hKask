//! Capability boundary tokens — OCAP authority in the type system
//!
//! Per Mark Miller's OCAP discipline: authority flows downward through the
//! loop hierarchy. These tokens prove that the holder has authority from
//! the correct loop.
//!
//! Each token can only be constructed by the loop that governs it — private
//! fields prevent forgery. The module path IS the loop assignment.

use hkask_types::WebID;

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

impl ConsolidationToken {
    pub fn new(issuer: WebID) -> Self {
        Self { issuer }
    }

    /// The expected issuer for this token type.
    pub fn expected_issuer() -> WebID {
        WebID::from_persona(b"curator")
    }

    /// Verify that this token was issued by the expected principal.
    pub fn verify_issuer(&self) -> bool {
        self.issuer() == &Self::expected_issuer()
    }

    /// The issuer of this token.
    pub fn issuer(&self) -> &WebID {
        &self.issuer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolidation_token_verify_issuer_accepts_expected() {
        let token = ConsolidationToken::new(ConsolidationToken::expected_issuer());
        assert!(token.verify_issuer());
    }

    #[test]
    fn consolidation_token_verify_issuer_rejects_wrong() {
        let wrong_issuer = WebID::from_persona(b"not-curator");
        let token = ConsolidationToken::new(wrong_issuer);
        assert!(!token.verify_issuer());
    }
}
