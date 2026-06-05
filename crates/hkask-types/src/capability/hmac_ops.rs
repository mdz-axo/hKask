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

/// Compute an HMAC-SHA256 over a flat list of byte slices.
///
/// Convenience wrapper around [`HmacBuilder`] for the common case where
/// all fields are known upfront.
#[allow(dead_code)]
pub fn compute_hmac(secret: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut builder = HmacBuilder::new(secret);
    for field in fields {
        builder.update(field);
    }
    builder.finalize()
}

/// Encode raw HMAC bytes as a hex string for storage/transmission.
///
/// `DelegationToken` uses hex encoding for its HMAC signature.
/// This function is the canonical encoding path.
pub fn encode_signature(hmac_bytes: &[u8]) -> String {
    hex::encode(hmac_bytes)
}

/// Decode a hex-encoded signature string back to raw bytes.
///
/// Inverse of [`encode_signature`].
#[allow(dead_code)]
pub fn decode_signature(encoded: &str) -> Result<Vec<u8>, String> {
    hex::decode(encoded).map_err(|e| e.to_string())
}

/// Constant-time comparison of two byte slices.
///
/// Prevents timing attacks when verifying HMAC signatures. Both
/// `DelegationToken::verify` delegates to this function.
pub fn verify_hmac_constant_time(expected: &[u8], actual: &[u8]) -> bool {
    expected.ct_eq(actual).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_builder_produces_consistent_output() {
        let secret = b"test-secret";
        let mut builder = HmacBuilder::new(secret);
        builder.update(b"field1");
        builder.update(b"field2");
        let sig = builder.finalize_hex();

        // Same fields via compute_hmac must produce the same result
        let fields: &[&[u8]] = &[b"field1", b"field2"];
        let raw = compute_hmac(secret, fields);
        assert_eq!(sig, encode_signature(&raw));
    }

    #[test]
    fn verify_constant_time_matches() {
        let secret = b"secret";
        let sig = compute_hmac(secret, &[b"data"]);
        let encoded = encode_signature(&sig);

        assert!(verify_hmac_constant_time(
            encoded.as_bytes(),
            encode_signature(&compute_hmac(secret, &[b"data"])).as_bytes(),
        ));
    }

    #[test]
    fn verify_constant_time_rejects_tampered() {
        let secret = b"secret";
        let sig = compute_hmac(secret, &[b"data"]);
        let encoded = encode_signature(&sig);

        assert!(!verify_hmac_constant_time(
            encoded.as_bytes(),
            b"tampered_signature",
        ));
    }

    #[test]
    fn encode_decode_roundtrip() {
        let bytes: [u8; 32] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98,
            0x76, 0x54, 0x32, 0x10,
        ];
        let encoded = encode_signature(&bytes);
        let decoded = decode_signature(&encoded).unwrap();
        assert_eq!(decoded, bytes.to_vec());
    }
}
