//! OCAP (Object-Capability) delegation and verification
//!
//! Implements capability-based access control per Mark Miller's OCAP principles.
//! Capabilities must be presented for all composition operations.
//!
//! **Security Model:**
//! - Ambient authority: capabilities must be explicitly passed
//! - Attenuation: delegation reduces authority (max 7 levels)
//! - Verification: all capabilities cryptographically signed

use hkask_types::{
    CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken, WebID,
};
use serde::{Deserialize, Serialize};

/// OCAP verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcapResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub attenuation_level: u8,
}

impl OcapResult {
    pub fn allow(attenuation_level: u8) -> Self {
        Self {
            allowed: true,
            reason: None,
            attenuation_level,
        }
    }

    pub fn deny(reason: &str) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.to_string()),
            attenuation_level: 0,
        }
    }
}

/// OCAP manager for capability verification
pub struct OCAP {
    checker: CapabilityChecker,
}

impl OCAP {
    /// Create new OCAP manager with secret key
    pub fn new(secret: &[u8]) -> Self {
        Self {
            checker: CapabilityChecker::new(secret),
        }
    }

    /// Verify a capability token
    pub fn verify(&self, token: &CapabilityToken) -> bool {
        self.checker.verify(token)
    }

    /// Verify token with time check
    pub fn verify_with_time(&self, token: &CapabilityToken, current_time: i64) -> bool {
        self.checker.verify_with_time(token, current_time)
    }

    /// Check capability for composition operation
    pub fn check_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
        current_time: i64,
    ) -> OcapResult {
        if !self.verify_with_time(token, current_time) {
            return OcapResult::deny("Token expired or invalid signature");
        }

        if token.delegated_to != *holder {
            return OcapResult::deny("Token not delegated to holder");
        }

        if !token.is_valid_for(resource, resource_id, action) {
            return OcapResult::deny(&format!(
                "Token does not grant {:?} action on {:?}",
                action, resource
            ));
        }

        OcapResult::allow(token.attenuation_level)
    }

    /// Check capability for template operations
    pub fn check_template(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        template_id: &str,
        action: CapabilityAction,
        current_time: i64,
    ) -> OcapResult {
        self.check_capability(
            token,
            holder,
            CapabilityResource::Template,
            template_id,
            action,
            current_time,
        )
    }

    /// Check capability for manifest operations
    pub fn check_manifest(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        manifest_id: &str,
        action: CapabilityAction,
        current_time: i64,
    ) -> OcapResult {
        self.check_capability(
            token,
            holder,
            CapabilityResource::Manifest,
            manifest_id,
            action,
            current_time,
        )
    }

    /// Check capability for registry operations
    pub fn check_registry(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        action: CapabilityAction,
        current_time: i64,
    ) -> OcapResult {
        self.check_capability(
            token,
            holder,
            CapabilityResource::Registry,
            "*",
            action,
            current_time,
        )
    }

    /// Check capability for cascade operations
    pub fn check_cascade(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        cascade_id: &str,
        action: CapabilityAction,
        current_time: i64,
    ) -> OcapResult {
        self.check_capability(
            token,
            holder,
            CapabilityResource::Cascade,
            cascade_id,
            action,
            current_time,
        )
    }

    /// Create attenuated token for delegation
    pub fn attenuate(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<CapabilityToken> {
        self.checker.attenuate(token, new_to, current_time)
    }

    /// Grant template capability
    pub fn grant_template(
        &self,
        template_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        self.checker.grant_template(template_id, action, from, to)
    }

    /// Grant manifest capability
    pub fn grant_manifest(
        &self,
        manifest_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        self.checker.grant_manifest(manifest_id, action, from, to)
    }

    /// Grant cascade capability
    pub fn grant_cascade(
        &self,
        cascade_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        self.checker.grant_cascade(cascade_id, action, from, to)
    }
}

impl Default for OCAP {
    fn default() -> Self {
        Self::new(b"default-ocap-secret-key")
    }
}

impl OCAP {
    /// Get the underlying checker
    pub fn checker(&self) -> &CapabilityChecker {
        &self.checker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn current_time() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn test_ocap_new() {
        let _ocap = OCAP::new(b"test-secret");
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            WebID::new(),
            b"wrong-secret",
        );
        assert!(!OCAP::default().checker().verify(&token));
    }

    #[test]
    fn test_ocap_grant_and_verify_template() {
        let ocap = OCAP::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();

        let token = ocap.grant_template(
            "prompt/test".to_string(),
            CapabilityAction::Render,
            from,
            to,
        );

        assert!(ocap.verify(&token));
        assert!(
            ocap.check_template(
                &token,
                &to,
                "prompt/test",
                CapabilityAction::Render,
                current_time()
            )
            .allowed
        );
    }

    #[test]
    fn test_ocap_deny_wrong_holder() {
        let ocap = OCAP::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();
        let wrong_holder = WebID::new();

        let token = ocap.grant_template(
            "prompt/test".to_string(),
            CapabilityAction::Render,
            from,
            to,
        );

        let result = ocap.check_template(
            &token,
            &wrong_holder,
            "prompt/test",
            CapabilityAction::Render,
            current_time(),
        );
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("holder"));
    }

    #[test]
    fn test_ocap_deny_wrong_action() {
        let ocap = OCAP::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();

        let token = ocap.grant_template(
            "prompt/test".to_string(),
            CapabilityAction::Render,
            from,
            to,
        );

        let result = ocap.check_template(
            &token,
            &to,
            "prompt/test",
            CapabilityAction::Write,
            current_time(),
        );
        assert!(!result.allowed);
    }

    #[test]
    fn test_ocap_attenuation() {
        let ocap = OCAP::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();
        let new_to = WebID::new();

        let token = ocap.grant_template(
            "prompt/test".to_string(),
            CapabilityAction::Render,
            from,
            to,
        );

        assert_eq!(token.attenuation_level, 0);
        assert!(token.can_attenuate());

        let attenuated = ocap.attenuate(&token, new_to, current_time());
        assert!(attenuated.is_some());
        let attenuated = attenuated.unwrap();
        assert_eq!(attenuated.attenuation_level, 1);
    }

    #[test]
    fn test_ocap_attenuation_max() {
        let ocap = OCAP::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();
        let secret = b"test-secret";

        // Start at max_attenuation - 1, so one more attenuation is allowed
        let token = CapabilityToken::new_with_attenuation(
            CapabilityResource::Template,
            "test".to_string(),
            CapabilityAction::Render,
            from,
            to,
            secret,
            None,
            6,
            7,
            None, // context_nonce
        );

        // Should be able to attenuate once more (6 < 7)
        assert!(token.can_attenuate());
        let attenuated = ocap.attenuate(&token, WebID::new(), current_time());
        assert!(attenuated.is_some());
        let attenuated = attenuated.unwrap();
        assert_eq!(attenuated.attenuation_level, 7);

        // Now at max, cannot attenuate further
        assert!(!attenuated.can_attenuate());
        let further = ocap.attenuate(&attenuated, WebID::new(), current_time());
        assert!(further.is_none());
    }
}
