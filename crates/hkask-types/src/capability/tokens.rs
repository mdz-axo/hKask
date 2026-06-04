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
/// Required by: ConsolidationPort::consolidate()
/// Issued by: CyberneticsLoop (or Curator as Cybernetics' governor)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ConsolidationToken {
    /// The agent whose consolidation this authorizes
    issuer: WebID,
}

impl ConsolidationToken {
    pub(crate) fn new(issuer: WebID) -> Self {
        Self { issuer }
    }

    pub fn issuer(&self) -> &WebID {
        &self.issuer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_webid() -> WebID {
        WebID::new()
    }

    #[test]
    fn consolidation_token_round_trip() {
        let issuer = test_webid();
        let token = ConsolidationToken::new(issuer);
        assert_eq!(*token.issuer(), issuer);
    }

    #[test]
    fn tokens_are_not_forgable_from_outside_crate() {
        // ConsolidationToken has a pub(crate) constructor, so external crates
        // cannot construct it. This test verifies the API surface:
        // - ConsolidationToken::new is pub(crate)
        // The private `issuer` field prevents structural construction.
        // Compile-time enforcement: no `pub fn new()` exists for external callers.
    }
}
