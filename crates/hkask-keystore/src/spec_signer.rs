//! Ed25519 spec signing — authenticates spec provenance (DDMVSS §7 Trust)
//!
//! Specs are curated, not governed. The signature authenticates the spec's
//! provenance — that it was registered by a known agent — not its authority.
//! The signing key is derived from the keystore's master secret via HKDF.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkask_types::InfrastructureError;

/// Ed25519 signing key for spec provenance authentication.
///
/// The signing key is derived from a master secret via HKDF-SHA256 with
/// the domain separation context `"hkask:spec-signing-key"`. This ensures
/// the spec-signing key is cryptographically independent from all other
/// internal secrets (ACP, OCAP, MCP, etc.).
pub struct Ed25519SpecSigner {
    signing_key: SigningKey,
}

impl Ed25519SpecSigner {
    /// Create a new spec signer from a 32-byte master secret.
    ///
    /// The secret is hashed via HKDF-SHA256 with context `"hkask:spec-signing-key"`
    /// to derive a 32-byte Ed25519 seed. This ensures domain separation from
    /// other keys derived from the same master secret.
    pub fn from_master_secret(master_secret: &[u8]) -> Self {
        let sub_key = crate::master_key::derive_sub_key(master_secret, "hkask:spec-signing-key");
        let seed: [u8; 32] = sub_key
            .as_slice()
            .try_into()
            .expect("derive_sub_key always produces 32 bytes");
        let signing_key = SigningKey::from_bytes(&seed);
        Self { signing_key }
    }

    /// Sign the canonical JSON representation of a spec.
    ///
    /// Returns the signature as a hex-encoded string (128 hex chars = 64 bytes).
    /// The canonical JSON must be produced by the caller (sorted keys, no
    /// whitespace) to ensure deterministic verification.
    pub fn sign_spec(&self, canonical_json: &[u8]) -> String {
        let signature = self.signing_key.sign(canonical_json);
        hex::encode(signature.to_bytes())
    }

    /// Verify a spec signature against its canonical JSON.
    ///
    /// Returns `Ok(())` if the signature is valid, `Err` otherwise.
    /// The `hex_signature` must be a 128-character hex string encoding
    /// the 64-byte Ed25519 signature.
    pub fn verify_spec(
        &self,
        canonical_json: &[u8],
        hex_signature: &str,
    ) -> Result<(), SpecSignatureError> {
        let sig_bytes = hex::decode(hex_signature).map_err(|_| SpecSignatureError::InvalidHex)?;
        let signature = Signature::try_from(sig_bytes.as_slice())
            .map_err(|_| SpecSignatureError::InvalidSignatureLength)?;
        self.signing_key
            .verifying_key()
            .verify(canonical_json, &signature)
            .map_err(|_| SpecSignatureError::VerificationFailed)
    }

    /// Return the verifying (public) key for this signer.
    ///
    /// Useful for storing the public key alongside a spec so that
    /// consumers who don't have the master secret can still verify.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Return the verifying key bytes (32 bytes) as a hex string.
    pub fn verifying_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }
}

/// Errors that can occur during spec signature verification.
#[derive(Debug, thiserror::Error)]
pub enum SpecSignatureError {
    #[error("Invalid hex in signature")]
    InvalidHex,
    #[error("Invalid signature length (expected 64 bytes)")]
    InvalidSignatureLength,
    #[error("Signature verification failed")]
    VerificationFailed,
}

impl From<SpecSignatureError> for InfrastructureError {
    fn from(e: SpecSignatureError) -> Self {
        match e {
            SpecSignatureError::InvalidHex
            | SpecSignatureError::InvalidSignatureLength
            | SpecSignatureError::VerificationFailed => {
                InfrastructureError::Serialization(e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn master_secret_for_test() -> [u8; 32] {
        [0x42u8; 32]
    }

    // P8 invariant: same canonical JSON + same signer → same signature
    #[test]
    fn sign_spec_is_deterministic() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let json = b"{\"category\":\"Domain\",\"name\":\"test-spec\"}";
        let sig1 = signer.sign_spec(json);
        let sig2 = signer.sign_spec(json);
        assert_eq!(sig1, sig2, "same input must produce same signature");
        assert_eq!(sig1.len(), 128, "Ed25519 signature hex is 128 chars");
    }

    // P8 invariant: different canonical JSON → different signature
    #[test]
    fn sign_spec_differs_for_different_payload() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let sig_a = signer.sign_spec(b"{\"name\":\"a\"}");
        let sig_b = signer.sign_spec(b"{\"name\":\"b\"}");
        assert_ne!(
            sig_a, sig_b,
            "different payloads must produce different signatures"
        );
    }

    // P8 invariant: verify succeeds for valid signature
    #[test]
    fn verify_spec_succeeds_for_valid_signature() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let json = b"{\"name\":\"test-spec\",\"category\":\"Domain\"}";
        let sig = signer.sign_spec(json);
        assert!(
            signer.verify_spec(json, &sig).is_ok(),
            "valid signature must verify successfully"
        );
    }

    // P8 invariant: verify fails for wrong payload
    #[test]
    fn verify_spec_fails_for_wrong_payload() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let sig = signer.sign_spec(b"{\"name\":\"original\"}");
        assert!(
            signer
                .verify_spec(b"{\"name\":\"tampered\"}", &sig)
                .is_err(),
            "wrong payload must fail verification"
        );
    }

    // P8 invariant: verify fails for invalid hex
    #[test]
    fn verify_spec_fails_for_invalid_hex() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let result = signer.verify_spec(b"{}", "not-valid-hex");
        assert!(matches!(result, Err(SpecSignatureError::InvalidHex)));
    }

    // P8 invariant: verify fails for wrong-length signature
    #[test]
    fn verify_spec_fails_for_wrong_length() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let short_sig = hex::encode([0u8; 32]);
        let result = signer.verify_spec(b"{}", &short_sig);
        assert!(matches!(
            result,
            Err(SpecSignatureError::InvalidSignatureLength)
        ));
    }

    // P8 invariant: different master secrets produce different verifying keys
    #[test]
    fn different_master_secrets_produce_different_keys() {
        let signer_a = Ed25519SpecSigner::from_master_secret(&[0x41u8; 32]);
        let signer_b = Ed25519SpecSigner::from_master_secret(&[0x42u8; 32]);
        assert_ne!(
            signer_a.verifying_key_hex(),
            signer_b.verifying_key_hex(),
            "different master secrets must produce different verifying keys"
        );
    }

    // P8 invariant: verifying_key_hex is 64 hex chars (32 bytes)
    #[test]
    fn verifying_key_hex_is_64_chars() {
        let signer = Ed25519SpecSigner::from_master_secret(&master_secret_for_test());
        let hex = signer.verifying_key_hex();
        assert_eq!(hex.len(), 64, "Ed25519 verifying key hex is 64 chars");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "key must be hex"
        );
    }
}
