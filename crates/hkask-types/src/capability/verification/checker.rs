//! Capability checker for composition-oriented capability management.
//!
//! \[NORMATIVE\] With Ed25519 tokens, verification uses the token's own public key —
//! no shared secret is required. The checker validates structural properties
//! (expiry, resource match, holder match) against the token (P4 — Clear Boundaries).

use crate::WebID;
use crate::capability::{DelegationAction, DelegationResource, DelegationToken};
use ed25519_dalek::SigningKey;

/// Capability checker for composition operations.
///
/// no shared secret is required. The checker validates structural properties
/// (expiry, resource match, holder match) against the token (P4 — Clear Boundaries).
pub struct CapabilityChecker {
    /// Optional Ed25519 signing key for token creation (grant_* methods).
    /// When absent, grant_* methods panic — token issuance requires a signing key.
    signing_key: Option<SigningKey>,
}

impl CapabilityChecker {
    /// Create a new capability checker without a signing key (verify-only).
    /// The `secret` parameter is retained for backward compatibility but unused.
    ///
    ///       but cannot issue new ones (grant_* methods will panic)
    pub fn new(_secret: &[u8]) -> Self {
        Self { signing_key: None }
    }

    /// Create a capability checker with a signing key for token issuance.
    ///
    pub fn with_signing_key(signing_key: SigningKey) -> Self {
        Self {
            signing_key: Some(signing_key),
        }
    }

    /// Verify a capability token's Ed25519 signature.
    ///
    pub fn verify(&self, token: &DelegationToken) -> bool {
        token.verify()
    }

    /// Check if token is valid and not expired
    ///
    ///       current_time is any i64 (Unix timestamp)
    ///       returns false otherwise
    pub fn verify_with_time(&self, token: &DelegationToken, current_time: i64) -> bool {
        self.verify(token) && !token.is_expired(current_time)
    }

    /// Check if a holder has capability for a resource/action
    ///
    ///       holder is any [`WebID`]; resource, resource_id, action describe the requested access
    ///       and token.is_valid_for(resource, resource_id, action) is true;
    ///       returns false otherwise
    pub fn check(
        &self,
        token: &DelegationToken,
        holder: &WebID,
        resource: DelegationResource,
        resource_id: &str,
        action: DelegationAction,
    ) -> bool {
        self.verify(token)
            && token.delegated_to == *holder
            && token.is_valid_for(resource, resource_id, action)
    }

    /// Check if holder has any capability for a resource type
    ///
    ///       holder is any [`WebID`]; resource is any [`DelegationResource`]
    ///       and token.grants_resource(resource) is true; returns false otherwise
    pub fn check_resource(
        &self,
        token: &DelegationToken,
        holder: &WebID,
        resource: DelegationResource,
    ) -> bool {
        self.verify(token) && token.delegated_to == *holder && token.grants_resource(resource)
    }

    /// Create a capability token for a tool.
    /// Requires a signing key — panics if constructed via `new()` instead of `with_signing_key()`.
    ///
    ///       from and to are any [`WebID`]
    ///       panics if no signing key is available
    pub fn grant_tool(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        let sk = self.signing_key.as_ref().expect("CapabilityChecker::grant_tool requires a signing key. Use with_signing_key() to construct.");
        DelegationToken::new(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for a template operation
    ///
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    ///       panics if no signing key is available
    pub fn grant_template(
        &self,
        template_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_template requires a signing key");
        DelegationToken::new(
            DelegationResource::Template,
            template_id,
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for a manifest operation
    ///
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    ///       panics if no signing key is available
    pub fn grant_manifest(
        &self,
        manifest_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_manifest requires a signing key");
        DelegationToken::new(
            DelegationResource::Registry,
            manifest_id,
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for registry operations
    ///
    ///       from and to are any [`WebID`]
    ///       panics if no signing key is available
    pub fn grant_registry(
        &self,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_registry requires a signing key");
        DelegationToken::new(
            DelegationResource::Registry,
            "*".to_string(),
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for cascade operations
    ///
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    ///       panics if no signing key is available
    pub fn grant_cascade(
        &self,
        cascade_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_cascade requires a signing key");
        DelegationToken::new(
            DelegationResource::Registry,
            cascade_id,
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for spec operations
    ///
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    ///       panics if no signing key is available
    pub fn grant_spec(
        &self,
        spec_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_spec requires a signing key");
        DelegationToken::new(DelegationResource::Registry, spec_id, action, from, to, sk)
    }

    /// Create an attenuated token for delegation
    ///
    ///       new_to is any [`WebID`]; current_time is any i64
    ///       returns `None` if no signing key is available or attenuation limit reached
    pub fn attenuate(
        &self,
        token: &DelegationToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<DelegationToken> {
        let sk = self.signing_key.as_ref()?;
        token.attenuate(new_to, sk, current_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::derive_signing_key;
    use crate::id::WebID;

    // contract: types-cap-verify-001
    #[test]
    fn capability_checker_new_creates_checker() {
        let secret = b"test-secret-32-bytes-long!!";
        let sk = derive_signing_key(secret);
        let checker = CapabilityChecker::with_signing_key(sk);
        // Verify it can verify a token it created
        let from = WebID::from_persona(b"issuer");
        let to = WebID::from_persona(b"holder");
        let token = checker.grant_tool("test_tool".into(), from, to);
        assert!(checker.verify(&token));
    }
}
