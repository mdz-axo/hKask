//! OCAP Adapter with Cryptographic Binding

use crate::ports::ocap_port::{DelegationEntry, OCAPConfig, OCAPResult};
use hkask_types::CapabilityToken;
use hkask_types::WebID;
use sha2::Sha256;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct OCAPAdapter {
    config: OCAPConfig,
    delegation_history: Arc<RwLock<HashMap<String, Vec<DelegationEntry>>>>,
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl OCAPAdapter {
    pub fn new(config: OCAPConfig) -> Self {
        Self {
            config,
            delegation_history: Arc::new(RwLock::new(HashMap::new())),
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    fn compute_chain_hash(
        &self,
        parent_hash: &str,
        from: &WebID,
        to: &WebID,
        ts: i64,
        level: u8,
    ) -> String {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.config.hmac_secret).unwrap();
        mac.update(parent_hash.as_bytes());
        mac.update(from.to_string().as_bytes());
        mac.update(to.to_string().as_bytes());
        mac.update(&ts.to_be_bytes());
        mac.update(&[level]);
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn verify_signature(&self, token: &CapabilityToken) -> bool {
        token.verify(&self.config.hmac_secret)
    }

    pub fn verify_attenuation_chain(&self, token: &CapabilityToken) -> OCAPResult {
        if !self.verify_signature(token) {
            return OCAPResult::InvalidSignature;
        }
        if let Some(exp) = token.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            if now > exp {
                return OCAPResult::Expired;
            }
        }
        if token.attenuation_level > self.config.max_attenuation_level {
            return OCAPResult::InvalidChain;
        }
        OCAPResult::Valid
    }

    pub fn is_expired(&self, token: &CapabilityToken, current_time: i64) -> bool {
        token
            .expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }

    pub fn record_delegation(
        &self,
        parent: &CapabilityToken,
        child: &CapabilityToken,
        timestamp: i64,
    ) {
        let root = parent.root_context_nonce();
        let hash = self.compute_chain_hash(
            root,
            &parent.delegated_to,
            &child.delegated_to,
            timestamp,
            child.attenuation_level,
        );
        let entry = DelegationEntry {
            delegated_from: parent.delegated_to,
            delegated_to: child.delegated_to,
            timestamp,
            attenuation_level: child.attenuation_level,
            chain_hash: hash,
        };
        let mut map = self.delegation_history.blocking_write();
        map.entry(root.to_string()).or_default().push(entry);
    }

    pub fn get_delegation_history(&self, root_nonce: &str) -> Vec<DelegationEntry> {
        self.delegation_history
            .blocking_read()
            .get(root_nonce)
            .cloned()
            .unwrap_or_default()
    }

    pub fn is_revoked(&self, token: &CapabilityToken) -> bool {
        self.revoked_tokens
            .blocking_read()
            .contains(&token.fingerprint())
    }

    pub fn revoke(&self, token: &CapabilityToken) {
        self.revoked_tokens
            .blocking_write()
            .insert(token.fingerprint());
    }
}
