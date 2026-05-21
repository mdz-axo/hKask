//! OCAP delegation with attenuation history and expiry enforcement
//!
//! This module provides:
//! - **Attenuation History**: Track capability delegation chains
//! - **Expiry Enforcement**: Enforce capability token expiration
//! - **Delegation Verification**: Verify attenuation chains cryptographically

use crate::security::{AttenuationHistory, ExpiryEnforcer};
use hkask_types::{CapabilityToken, WebID};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// OCAP manager for capability delegation and tracking
pub struct OCAP {
    attenuation_history: Arc<RwLock<HashMap<String, AttenuationHistory>>>,
    expiry_enforcer: ExpiryEnforcer,
}

impl OCAP {
    pub fn new() -> Self {
        Self {
            attenuation_history: Arc::new(RwLock::new(HashMap::new())),
            expiry_enforcer: ExpiryEnforcer::default(),
        }
    }

    pub fn with_expiry(enforcer: ExpiryEnforcer) -> Self {
        Self {
            attenuation_history: Arc::new(RwLock::new(HashMap::new())),
            expiry_enforcer: enforcer,
        }
    }

    /// Record an attenuation event in the history
    pub async fn record_attenuation(
        &self,
        root_nonce: &str,
        delegated_from: &WebID,
        delegated_to: &WebID,
        timestamp: i64,
        attenuation_level: u8,
    ) {
        let mut history_map = self.attenuation_history.write().await;
        let history = history_map
            .entry(root_nonce.to_string())
            .or_insert_with(|| AttenuationHistory::new(root_nonce.to_string()));
        
        history.add_entry(
            delegated_from.to_string(),
            delegated_to.to_string(),
            timestamp,
            attenuation_level,
        );
    }

    /// Verify attenuation chain for a capability token
    pub async fn verify_attenuation(&self, token: &CapabilityToken) -> bool {
        let history_map = self.attenuation_history.read().await;
        let root_nonce = token.root_context_nonce();
        
        if let Some(history) = history_map.get(root_nonce) {
            history.verify_chain() && history.chain_length() >= token.attenuation_level as usize
        } else {
            // No history recorded, verify via token's internal chain
            token.verify_attenuation_chain(root_nonce, token.attenuation_level)
        }
    }

    /// Check if capability token is expired
    pub fn is_expired(&self, token: &CapabilityToken, current_time: i64) -> bool {
        token
            .expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }

    /// Validate capability token expiry
    pub fn validate_expiry(&self, token: &CapabilityToken, current_time: i64) -> bool {
        token
            .expires_at
            .map(|exp| current_time <= exp)
            .unwrap_or(true)
    }

    /// Enforce expiry and return calculated expiry time for new tokens
    pub fn calculate_expiry(&self, creation_time: i64) -> i64 {
        self.expiry_enforcer.calculate_expiry(creation_time)
    }

    /// Get attenuation history for a root nonce
    pub async fn get_attenuation_history(&self, root_nonce: &str) -> Option<AttenuationHistory> {
        let history_map = self.attenuation_history.read().await;
        history_map.get(root_nonce).cloned()
    }

    /// Get max lifetime in seconds
    pub fn max_lifetime_secs(&self) -> u64 {
        self.expiry_enforcer.max_lifetime_secs()
    }

    /// Create an attenuated token with history tracking
    pub async fn attenuate_with_history(
        &self,
        parent: &CapabilityToken,
        new_to: WebID,
        secret: &[u8],
        current_time: i64,
    ) -> Option<CapabilityToken> {
        if !parent.can_attenuate() {
            return None;
        }

        // Calculate expiry based on enforcer policy
        let expires_at = Some(self.calculate_expiry(current_time));
        
        let child = parent.attenuate_with_expiry(new_to, secret, expires_at);
        
        if let Some(ref child_token) = child {
            // Record attenuation in history
            self.record_attenuation(
                parent.root_context_nonce(),
                &parent.delegated_to,
                &child_token.delegated_to,
                current_time,
                child_token.attenuation_level,
            ).await;
        }
        
        child
    }
}

impl Default for OCAP {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{CapabilityAction, CapabilityResource};

    #[tokio::test]
    async fn test_ocap_attenuation_history() {
        let ocap = OCAP::new();
        let webid1 = WebID::new();
        let webid2 = WebID::new();
        let webid3 = WebID::new();

        ocap.record_attenuation("root-nonce", &webid1, &webid2, 1000, 0).await;
        ocap.record_attenuation("root-nonce", &webid2, &webid3, 1001, 1).await;

        let history = ocap.get_attenuation_history("root-nonce").await;
        assert!(history.is_some());
        assert_eq!(history.unwrap().chain_length(), 2);
    }

    #[tokio::test]
    async fn test_ocap_verify_attenuation() {
        let ocap = OCAP::new();
        let webid1 = WebID::new();
        let webid2 = WebID::new();

        ocap.record_attenuation("root-nonce", &webid1, &webid2, 1000, 0).await;

        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "test-tool".to_string(),
            CapabilityAction::Execute,
            webid1,
            webid2,
            b"test-secret",
        );

        // Token should verify (internal chain check)
        assert!(ocap.verify_attenuation(&token).await);
    }

    #[test]
    fn test_ocap_expiry_enforcement() {
        use hkask_types::CapabilityToken;
        
        let ocap = OCAP::new();
        let creation_time = 1000;
        let expiry = ocap.calculate_expiry(creation_time);

        assert_eq!(expiry, creation_time + 3600); // Default 1 hour
        
        // Create token with expiry set
        let mut token_valid = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            WebID::new(),
            b"secret",
        );
        token_valid.expires_at = Some(expiry - 1);
        assert!(ocap.validate_expiry(&token_valid, expiry - 1));
        
        // Create token with expiry set
        let mut token_expired = CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            WebID::new(),
            b"secret",
        );
        token_expired.expires_at = Some(expiry);
        assert!(!ocap.validate_expiry(&token_expired, expiry + 1));
    }

    #[tokio::test]
    async fn test_ocp_attenuate_with_history() {
        let ocap = OCAP::new();
        let webid1 = WebID::new();
        let webid2 = WebID::new();
        let secret = b"test-secret";
        let current_time = 1000;

        let parent = CapabilityToken::new(
            CapabilityResource::Tool,
            "test-tool".to_string(),
            CapabilityAction::Execute,
            webid1,
            webid1,
            secret,
        );

        let child = ocap.attenuate_with_history(&parent, webid2, secret, current_time).await;
        assert!(child.is_some());

        let child_token = child.unwrap();
        assert_eq!(child_token.attenuation_level, 1);
        assert!(child_token.expires_at.is_some());

        // Verify history was recorded
        let history = ocap.get_attenuation_history(parent.root_context_nonce()).await;
        assert!(history.is_some());
        assert_eq!(history.unwrap().chain_length(), 1);
    }

    #[test]
    fn test_ocap_custom_expiry() {
        use std::time::Duration;
        let enforcer = ExpiryEnforcer::new(Duration::from_secs(7200)); // 2 hours
        let ocap = OCAP::with_expiry(enforcer);

        let creation_time = 1000;
        let expiry = ocap.calculate_expiry(creation_time);

        assert_eq!(expiry, creation_time + 7200);
        assert_eq!(ocap.max_lifetime_secs(), 7200);
    }
}
