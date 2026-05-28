//! Verification logic for capability tokens
//!
//! Contains `VerificationResult` for distributed verification outcomes
//! and `CapabilityChecker` for composition-oriented capability management.

use super::{CapabilityAction, CapabilityResource, CapabilityToken};
use crate::WebID;
use zeroize::Zeroizing;

/// Cryptographic verification result for distributed verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
    /// Signature valid, not expired — capability can be used
    Valid,
    /// Signature valid, but expired — capability is "zombie" (valid but unusable)
    Zombie,
    /// Signature invalid — capability is tampered or forged
    Invalid,
}

impl VerificationResult {
    /// Check if verification succeeded (valid or zombie)
    pub fn is_valid(&self) -> bool {
        matches!(self, VerificationResult::Valid | VerificationResult::Zombie)
    }

    /// Check if capability can be used (valid only, not zombie)
    pub fn is_usable(&self) -> bool {
        matches!(self, VerificationResult::Valid)
    }

    /// Get human-readable description
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationResult::Valid => "valid",
            VerificationResult::Zombie => "zombie (expired but valid signature)",
            VerificationResult::Invalid => "invalid (signature verification failed)",
        }
    }
}

/// Capability checker for composition operations
pub struct CapabilityChecker {
    secret: Zeroizing<Vec<u8>>,
}

impl CapabilityChecker {
    /// Create a new capability checker with the given secret
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: Zeroizing::new(secret.to_vec()),
        }
    }

    /// Verify a capability token
    pub fn verify(&self, token: &CapabilityToken) -> bool {
        token.verify(&self.secret)
    }

    /// Check if token is valid and not expired
    pub fn verify_with_time(&self, token: &CapabilityToken, current_time: i64) -> bool {
        self.verify(token) && !token.is_expired(current_time)
    }

    /// Check if a holder has capability for a resource/action
    pub fn check(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> bool {
        self.verify(token)
            && token.delegated_to == *holder
            && token.is_valid_for(resource, resource_id, action)
    }

    /// Check if holder has any capability for a resource type
    pub fn check_resource(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        resource: CapabilityResource,
    ) -> bool {
        self.verify(token) && token.delegated_to == *holder && token.grants_resource(resource)
    }

    /// Create a capability token for a tool
    pub fn grant_tool(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for a template operation
    pub fn grant_template(
        &self,
        template_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Template,
            template_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for a manifest operation
    pub fn grant_manifest(
        &self,
        manifest_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Manifest,
            manifest_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for registry operations
    pub fn grant_registry(
        &self,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Registry,
            "*".to_string(),
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for cascade operations
    pub fn grant_cascade(
        &self,
        cascade_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Cascade,
            cascade_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for spec operations
    pub fn grant_spec(
        &self,
        spec_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Spec,
            spec_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create an attenuated token for delegation
    pub fn attenuate(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<CapabilityToken> {
        token.attenuate(new_to, &self.secret, current_time)
    }

    /// Verify a capability token for tool access (OCAP-idiomatic)
    ///
    /// The holder presents the token; the checker verifies it.
    /// Checks: signature, expiry, holder match, resource/action match.
    pub fn verify_tool_capability(
        &self,
        token: &CapabilityToken,
        expected_holder: &WebID,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> bool {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // 1. Verify signature and expiry
        if !self.verify_with_time(token, current_time) {
            return false;
        }

        // 2. Verify holder matches
        if token.delegated_to != *expected_holder {
            return false;
        }

        // 3. Verify resource/action match
        if !token.is_valid_for(resource, resource_id, action) {
            return false;
        }

        true
    }
}
