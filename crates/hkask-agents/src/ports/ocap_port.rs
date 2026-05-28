//! OCAP Port — Object Capability Security Interface

use hkask_types::{CapabilityToken, WebID};

/// OCAP verification result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OCAPResult {
    Valid,
    Expired,
    InvalidSignature,
    InvalidChain,
    Revoked,
}

/// OCAP Port — Object capability security interface
pub trait OCAPPort {
    fn verify_signature(&self, token: &CapabilityToken) -> bool;
    fn verify_attenuation_chain(&self, token: &CapabilityToken) -> OCAPResult;
    fn is_expired(&self, token: &CapabilityToken, current_time: i64) -> bool;
    fn record_delegation(&self, parent: &CapabilityToken, child: &CapabilityToken, timestamp: i64);
    fn get_delegation_history(&self, root_nonce: &str) -> Vec<DelegationEntry>;
    fn is_revoked(&self, token: &CapabilityToken) -> bool;
    fn revoke(&self, token: &CapabilityToken);
}

/// Delegation entry for history tracking
#[derive(Debug, Clone)]
pub struct DelegationEntry {
    pub delegated_from: WebID,
    pub delegated_to: WebID,
    pub timestamp: i64,
    pub attenuation_level: u8,
    pub chain_hash: String,
}

/// OCAP configuration
#[derive(Debug, Clone)]
pub struct OCAPConfig {
    pub max_attenuation_level: u8,
    pub default_lifetime_secs: i64,
    pub hmac_secret: Vec<u8>,
}

impl Default for OCAPConfig {
    fn default() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Self {
            max_attenuation_level: 7,
            default_lifetime_secs: 3600,
            hmac_secret: bytes.to_vec(),
        }
    }
}
