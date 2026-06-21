//! Minimal crypto types — shared across capability and wallet domains.
//!
//! Only contains value types with zero crypto library dependencies.
//! Conversion to/from `ed25519_dalek` types lives in downstream crates.

use serde::{Deserialize, Serialize};

/// Ed25519 public key — 32 bytes.
///
/// Newtype to prevent accidental mixing with other 32-byte values
/// (hashes, secrets, UUIDs). Conversion to/from `ed25519_dalek::VerifyingKey`
/// lives in `hkask-keystore` where the crypto dependency exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; 32]);

impl Ed25519PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Ed25519PublicKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
