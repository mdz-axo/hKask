//! Shared HMAC operations for capability tokens.
//!
//! Provides HMAC-SHA256 signing, constant-time verification, and signature
//! encoding/decoding used by `DelegationToken`.
//! This module is the single source of truth for OCAP cryptographic primitives
//! per ADR-025.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Incremental HMAC-SHA256 builder for capability token signatures.
///
/// Each token type feeds its authority-bearing fields into the builder
/// via [`update`](Self::update), then finalizes to obtain the signature.
/// This avoids duplicating HMAC construction and finalization logic across
/// token types.
pub struct HmacBuilder {
    mac: HmacSha256,
}

impl HmacBuilder {
    /// Create a new HMAC-SHA256 builder with the given secret key.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            mac: HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size"),
        }
    }

    /// Feed a byte slice into the HMAC computation.
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.mac.update(data);
        self
    }

    /// Finalize the HMAC and return the raw 32-byte digest.
    pub fn finalize(self) -> [u8; 32] {
        self.mac.finalize().into_bytes().into()
    }

    /// Finalize the HMAC and return the hex-encoded signature string.
    pub fn finalize_hex(self) -> String {
        encode_signature(&self.finalize())
    }
}

/// Encode raw HMAC bytes as a hex string for storage/transmission.
///
/// `DelegationToken` uses hex encoding for its HMAC signature.
/// This function is the canonical encoding path.
pub fn encode_signature(hmac_bytes: &[u8]) -> String {
    hex::encode(hmac_bytes)
}

/// Constant-time comparison of two byte slices.
///
/// Prevents timing attacks when verifying HMAC signatures. Both
/// `DelegationToken::verify` delegates to this function.
pub fn verify_hmac_constant_time(expected: &[u8], actual: &[u8]) -> bool {
    expected.ct_eq(actual).into()
}
