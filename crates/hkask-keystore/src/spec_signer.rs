//! Ed25519 spec signing — authenticates spec provenance (MDS §3 Trust)
//!
//! Specs are curated, not governed. The signature authenticates the spec's
//! provenance — that it was registered by a known agent — not its authority.
//! The signing key is derived from the keystore's master secret via HKDF.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkask_types::InfrastructureError;
use tracing::info;

/// Ed25519 signing key for spec provenance authentication.
///
/// The signing key is derived from a master secret via HKDF-SHA256 with
/// the domain separation context `"hkask:spec-signing-key"`. This ensures
/// the spec-signing key is cryptographically independent from all other
/// internal secrets (A2A, OCAP, MCP, etc.).
pub struct Ed25519SpecSigner {
    signing_key: SigningKey,
}

impl Ed25519SpecSigner {
    /// Create a new spec signer from a 32-byte master secret.
    ///
    /// The secret is hashed via HKDF-SHA256 with context `"hkask:spec-signing-key"`
    /// to derive a 32-byte Ed25519 seed. This ensures domain separation from
    /// other keys derived from the same master secret.
    ///
    /// REQ: KEY-020
    /// pre:  master_secret is non-empty
    /// post: returns Ed25519SpecSigner with derived signing key
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
    ///
    /// REQ: KEY-021
    /// pre:  canonical_json is non-empty
    /// post: returns 128-char hex-encoded Ed25519 signature
    pub fn sign_spec(&self, canonical_json: &[u8]) -> String {
        // P9: CNS span
        info!(target: "cns.keystore", operation = "sign_spec", status = "started", "CNS");
        let signature = self.signing_key.sign(canonical_json);
        let sig_hex = hex::encode(signature.to_bytes());
        // P9: CNS span
        info!(target: "cns.keystore", operation = "sign_spec", status = "completed", "CNS");
        sig_hex
    }

    /// Verify a spec signature against its canonical JSON.
    ///
    /// Returns `Ok(())` if the signature is valid, `Err` otherwise.
    /// The `hex_signature` must be a 128-character hex string encoding
    /// the 64-byte Ed25519 signature.
    ///
    /// REQ: KEY-022
    /// pre:  canonical_json is non-empty, hex_signature is 128 hex chars
    /// post: returns Ok(()) if signature valid, Err otherwise
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
    ///
    /// REQ: KEY-023
    /// post: returns Ed25519 VerifyingKey
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Return the verifying key bytes (32 bytes) as a hex string.
    ///
    /// REQ: KEY-024
    /// post: returns 64-char hex-encoded verifying key
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
