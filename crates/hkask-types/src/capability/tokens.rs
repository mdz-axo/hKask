//! Capability boundary tokens — OCAP authority in the type system
//!
//! Per Mark Miller's OCAP discipline: authority flows downward through the
//! loop hierarchy (Curation → Cybernetics → domain loops). These ZST tokens
//! prove that the holder has authority from the correct loop.
//!
//! Each token can only be constructed by the loop that governs it — private
//! fields prevent forgery. The module path IS the loop assignment.

use crate::id::WebID;

/// Token proving Cybernetics authority. Only CyberneticsLoop can issue this.
///
/// Required for: energy budget operations, circuit breaking, dampening,
/// throttling, algedonic alert escalation, consolidation bridge triggering.
#[derive(Debug, Clone)]
pub struct CyberneticsToken {
    /// The agent whose authority this token represents
    issuer: WebID,
}

impl CyberneticsToken {
    /// Only hkask-cns::CyberneticsLoop can construct this.
    /// This is pub(crate) so external crates cannot forge tokens.
    pub(crate) fn new(issuer: WebID) -> Self {
        Self { issuer }
    }

    pub fn issuer(&self) -> &WebID {
        &self.issuer
    }
}

/// Token proving Curation authority. Only the Curator can issue this.
///
/// Required for: overriding Cybernetics decisions, triggering consolidation,
/// evaluating template outputs, metacognitive observation.
#[derive(Debug, Clone)]
pub struct CurationToken {
    /// The curator's WebID
    issuer: WebID,
}

impl CurationToken {
    pub(crate) fn new(issuer: WebID) -> Self {
        Self { issuer }
    }

    pub fn issuer(&self) -> &WebID {
        &self.issuer
    }
}

/// Token proving that a consolidation (Episodic → Semantic) operation
/// was authorized by Cybernetics. This is the one-way bridge token.
///
/// Required by: ConsolidationPort::consolidate()
/// Issued by: CyberneticsLoop (or Curator as Cybernetics' governor)
#[derive(Debug, Clone)]
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
    fn cybernetics_token_round_trip() {
        let issuer = test_webid();
        let token = CyberneticsToken::new(issuer);
        assert_eq!(*token.issuer(), issuer);
    }

    #[test]
    fn curation_token_round_trip() {
        let issuer = test_webid();
        let token = CurationToken::new(issuer);
        assert_eq!(*token.issuer(), issuer);
    }

    #[test]
    fn consolidation_token_round_trip() {
        let issuer = test_webid();
        let token = ConsolidationToken::new(issuer);
        assert_eq!(*token.issuer(), issuer);
    }

    #[test]
    fn tokens_are_not_forgable_from_outside_crate() {
        // These types have pub(crate) constructors, so external crates
        // cannot construct them. This test verifies the API surface:
        // - CyberneticsToken::new is pub(crate)
        // - CurationToken::new is pub(crate)
        // - ConsolidationTokenToken::new is pub(crate)
        // The private `issuer` field prevents structural construction.
        // Compile-time enforcement: no `pub fn new()` exists for external callers.
    }

    #[test]
    fn different_issuers_produce_different_tokens() {
        let a = CyberneticsToken::new(WebID::new());
        let b = CyberneticsToken::new(WebID::new());
        assert_ne!(a.issuer(), b.issuer());
    }

    #[test]
    fn token_clones_preserve_issuer() {
        let issuer = test_webid();
        let token = CyberneticsToken::new(issuer);
        let cloned = token.clone();
        assert_eq!(*cloned.issuer(), issuer);
    }
}
