//! Authentication context and signing key derivation.

use crate::WebID;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

use super::token_types::DelegationToken;

/// Verified authentication context — caller's identity and capability token.
/// Both API (middleware verification) and CLI (keystore resolution) produce this type.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub token: DelegationToken,
    pub webid: WebID,
}

/// Derive an Ed25519 signing key from arbitrary secret bytes.
///
/// \[NORMATIVE\] Hashes the input with SHA-256 to produce a 32-byte seed,
/// then constructs a `SigningKey`. This allows existing HMAC-secret-based
/// callers to migrate to Ed25519 without changing their secret management (P4 — Clear Boundaries).
pub fn derive_signing_key(secret: &[u8]) -> SigningKey {
    let seed: [u8; 32] = Sha256::digest(secret).into();
    SigningKey::from_bytes(&seed)
}
