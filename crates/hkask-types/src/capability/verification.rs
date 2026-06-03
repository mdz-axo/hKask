//! Verification logic for capability tokens
//!
//! Contains `CapabilityChecker` for composition-oriented capability management.

use super::{DelegationAction, DelegationResource, DelegationToken};
use crate::WebID;
use zeroize::Zeroizing;

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
    pub fn verify(&self, token: &DelegationToken) -> bool {
        token.verify(&self.secret)
    }

    /// Check if token is valid and not expired
    pub fn verify_with_time(&self, token: &DelegationToken, current_time: i64) -> bool {
        self.verify(token) && !token.is_expired(current_time)
    }

    /// Check if a holder has capability for a resource/action
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
    pub fn check_resource(
        &self,
        token: &DelegationToken,
        holder: &WebID,
        resource: DelegationResource,
    ) -> bool {
        self.verify(token) && token.delegated_to == *holder && token.grants_resource(resource)
    }

    /// Create a capability token for a tool
    pub fn grant_tool(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for a template operation
    pub fn grant_template(
        &self,
        template_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Template,
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
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Registry,
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
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Registry,
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
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Registry,
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
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Registry,
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
        token: &DelegationToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<DelegationToken> {
        token.attenuate(new_to, &self.secret, current_time)
    }
}
