//! Loop authority tokens — ZST capability tokens for loop-authorized operations.
//!
//! These are capability boundary tokens per Mark Miller's OCAP discipline.
//! Each token can only be constructed by the loop that governs it — private
//! fields prevent forgery. The module path IS the loop assignment.
//!
//! Unlike `DelegationToken` (in `hkask-capability`), these carry no
//! cryptographic signature and are not intended for inter-agent delegation.

use crate::id::WebID;

/// Token proving that a consolidation (Episodic → Semantic) operation
/// was authorized.
///
/// Only the Curation Loop can mint this token. It authorizes consolidation
/// from Episodic to Semantic memory.
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
