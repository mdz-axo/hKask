//! OCAP Port — Object Capability Security Types
//!
//! Type definitions for object capability security.
//! The concrete OCAPAdapter implements these operations as inherent methods.

use hkask_types::WebID;

/// OCAP verification result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OCAPResult {
    Valid,
    Expired,
    InvalidSignature,
    InvalidChain,
    Revoked,
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
