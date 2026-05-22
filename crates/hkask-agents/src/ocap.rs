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
            )
            .await;
        }

        child
    }
}

impl Default for OCAP {
    fn default() -> Self {
        Self::new()
    }
}
