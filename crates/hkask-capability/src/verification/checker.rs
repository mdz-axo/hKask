//! Capability checker for composition-oriented capability management.
//!
//! \[NORMATIVE\] With Ed25519 tokens, verification uses the token's own public key —
//! no shared secret is required. The checker validates structural properties
//! (expiry, resource match, holder match) against the token (P4 — Clear Boundaries).

use crate::{DelegationAction, DelegationResource, DelegationToken};
use ed25519_dalek::SigningKey;
use hkask_types::WebID;

/// Capability checker for composition operations.
///
/// \[NORMATIVE\] With Ed25519 tokens, verification uses the token's own public key —
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  _secret is any byte slice (retained for backward compatibility, unused)
    /// post: returns a [`CapabilityChecker`] with no signing key; can verify tokens
    ///       but cannot issue new ones (grant_* methods will panic)
    pub fn new(_secret: &[u8]) -> Self {
        Self { signing_key: None }
    }

    /// Create a capability checker with a signing key for token issuance.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  signing_key is a valid Ed25519 [`SigningKey`]
    /// post: returns a [`CapabilityChecker`] that can both verify and issue tokens
    pub fn with_signing_key(signing_key: SigningKey) -> Self {
        Self {
            signing_key: Some(signing_key),
        }
    }

    /// Verify a capability token's Ed25519 signature.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`CapabilityChecker`]; token is any [`DelegationToken`]
    /// post: returns the result of [`DelegationToken::verify`] — true if Ed25519 signature is valid
    pub fn verify(&self, token: &DelegationToken) -> bool {
        token.verify()
    }

    /// Check if token is valid and not expired
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`CapabilityChecker`]; token is any [`DelegationToken`];
    ///       current_time is any i64 (Unix timestamp)
    /// post: returns true if both signature is valid and token is not expired at current_time;
    ///       returns false otherwise
    pub fn verify_with_time(&self, token: &DelegationToken, current_time: i64) -> bool {
        self.verify(token) && !token.is_expired(current_time)
    }

    /// Check if a holder has capability for a resource/action
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`CapabilityChecker`]; token is any [`DelegationToken`];
    ///       holder is any [`WebID`]; resource, resource_id, action describe the requested access
    /// post: returns true if signature is valid, token.delegated_to matches holder,
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`CapabilityChecker`]; token is any [`DelegationToken`];
    ///       holder is any [`WebID`]; resource is any [`DelegationResource`]
    /// post: returns true if signature is valid, token.delegated_to matches holder,
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self was constructed via `with_signing_key`; tool_name is any non-empty [`String`];
    ///       from and to are any [`WebID`]
    /// post: returns a [`DelegationToken`] for `DelegationResource::Tool` with `DelegationAction::Execute`;
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self was constructed via `with_signing_key`; template_id is any non-empty [`String`];
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    /// post: returns a [`DelegationToken`] for `DelegationResource::Template`;
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self was constructed via `with_signing_key`; manifest_id is any non-empty [`String`];
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    /// post: returns a [`DelegationToken`] for `DelegationResource::Registry` with the given manifest_id;
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self was constructed via `with_signing_key`; action is any [`DelegationAction`];
    ///       from and to are any [`WebID`]
    /// post: returns a [`DelegationToken`] for `DelegationResource::Registry` with resource_id "*";
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self was constructed via `with_signing_key`; cascade_id is any non-empty [`String`];
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    /// post: returns a [`DelegationToken`] for `DelegationResource::Registry` with the given cascade_id;
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self was constructed via `with_signing_key`; spec_id is any non-empty [`String`];
    ///       action is any [`DelegationAction`]; from and to are any [`WebID`]
    /// post: returns a [`DelegationToken`] for `DelegationResource::Registry` with the given spec_id;
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`CapabilityChecker`]; token is any [`DelegationToken`];
    ///       new_to is any [`WebID`]; current_time is any i64
    /// post: returns `Some(attenuated_token)` if self has a signing key and token.can_attenuate();
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
    use crate::derive_signing_key;
    use hkask_types::WebID;

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
