//! AES-256-GCM encryption with Argon2 key derivation

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use thiserror::Error;
use zeroize::Zeroizing;

/// Salt size for Argon2 (16 bytes = 128 bits)
pub(crate) const SALT_SIZE: usize = 16;

/// Nonce size for AES-GCM (12 bytes = 96 bits)
pub(crate) const NONCE_SIZE: usize = 12;

/// Argon2id memory cost: 64 MiB (OWASP recommendation for high-security)
/// This is the amount of memory used in KiB.
pub(crate) const ARGON2_MEMORY_COST: u32 = 65536;

/// Argon2id iteration count: 3 (balanced for interactive use)
/// Higher values increase security but also latency.
pub(crate) const ARGON2_TIME_COST: u32 = 3;

/// Argon2id parallelism: 4 lanes
/// Should match the number of CPU cores available.
pub(crate) const ARGON2_PARALLELISM: u32 = 4;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum EncryptionError {
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Decryption failed: {0}")]
    Decryption(String),
    #[error("Invalid passphrase")]
    InvalidPassphrase,
}

/// Encryption service using AES-256-GCM
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create a new encryption service from a passphrase
    pub fn new(passphrase: &str, salt: &[u8]) -> Result<Self, EncryptionError> {
        if passphrase.is_empty() {
            return Err(EncryptionError::InvalidPassphrase);
        }

        let key = derive_key(passphrase, salt)?;
        let cipher = Aes256Gcm::new_from_slice(&*key)
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        Ok(Self { cipher })
    }

    /// Generate a random salt
    pub fn generate_salt() -> [u8; SALT_SIZE] {
        let mut salt = [0u8; SALT_SIZE];
        rand::rng().fill_bytes(&mut salt);
        salt
    }

    /// Encrypt plaintext data
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt ciphertext data
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if ciphertext.len() < NONCE_SIZE {
            return Err(EncryptionError::Decryption(
                "Ciphertext too short".to_string(),
            ));
        }

        let nonce_bytes = &ciphertext[..NONCE_SIZE];
        let data = &ciphertext[NONCE_SIZE..];

        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| EncryptionError::Decryption(e.to_string()))
    }
}

/// Derive a 32-byte key from a passphrase using Argon2id with secure parameters
///
/// **Security Parameters:**
/// - Algorithm: Argon2id (hybrid, resistant to side-channel and GPU attacks)
/// - Memory: 64 MiB (65536 KiB)
/// - Iterations: 3
/// - Parallelism: 4 lanes
/// - Output: 32 bytes (256 bits for AES-256)
///
/// These parameters follow OWASP recommendations for high-security applications.
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, EncryptionError> {
    let mut key = Zeroizing::new([0u8; 32]);
    let params = Params::new(
        ARGON2_MEMORY_COST,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut *key)
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    //
    // Behavioral tests for encryption service.
    //
    // Each test verifies a stated invariant of a public seam.
    // P8: No test without an invariant. C8: Test depth matches module depth.
    //
    // DDMVSS category mapping:
    //   Domain       → derive_key determinism, length, domain separation
    //   Composition  → EncryptionService roundtrip, nonce randomness
    //   Curation     → InvalidPassphrase guard
    //   Persistence  → Truncated ciphertext rejection
    //

    use super::*;

    // ── Domain: derive_key determinism ────────────────────────────────

    // P8 invariant: same passphrase + same salt → same 32-byte key
    #[test]
    fn derive_key_is_deterministic() {
        let salt = [1u8; SALT_SIZE];
        let key1 = derive_key("pass", &salt).expect("first derive_key should succeed");
        let key2 = derive_key("pass", &salt).expect("second derive_key should succeed");
        assert_eq!(*key1, *key2, "same inputs must produce identical keys");
    }

    // P8 invariant: different passphrase → different key (same salt)
    #[test]
    fn derive_key_differs_with_different_passphrase() {
        let salt = [2u8; SALT_SIZE];
        let key_a = derive_key("pass-A", &salt).expect("derive_key pass-A should succeed");
        let key_b = derive_key("pass-B", &salt).expect("derive_key pass-B should succeed");
        assert_ne!(
            *key_a, *key_b,
            "different passphrases must produce different keys"
        );
    }

    // P8 invariant: different salt → different key (same passphrase)
    #[test]
    fn derive_key_differs_with_different_salt() {
        let salt_a = [3u8; SALT_SIZE];
        let salt_b = [4u8; SALT_SIZE];
        let key_a = derive_key("pass", &salt_a).expect("derive_key salt_a should succeed");
        let key_b = derive_key("pass", &salt_b).expect("derive_key salt_b should succeed");
        assert_ne!(
            *key_a, *key_b,
            "different salts must produce different keys"
        );
    }

    // P8 invariant: derive_key always produces exactly 32 bytes
    #[test]
    fn derive_key_output_is_32_bytes() {
        let salt = [5u8; SALT_SIZE];
        let key = derive_key("pass", &salt).expect("derive_key should succeed");
        assert_eq!(key.len(), 32, "derived key must be exactly 32 bytes");
    }

    // ── Composition: EncryptionService roundtrip ──────────────────────

    // P8 invariant: encrypt then decrypt is identity for any non-empty passphrase
    #[test]
    fn encryption_roundtrip_returns_original_plaintext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("test-passphrase-123", &salt)
            .expect("EncryptionService::new should succeed");
        let plaintext = b"hello world";
        let ciphertext = service.encrypt(plaintext).expect("encrypt should succeed");
        let decrypted = service
            .decrypt(&ciphertext)
            .expect("decrypt should succeed");
        assert_eq!(
            decrypted, plaintext,
            "roundtrip must recover original plaintext"
        );
    }

    // P8 invariant: encrypting the same plaintext twice produces different
    // ciphertexts (random nonce ensures semantic security)
    #[test]
    fn encryption_roundtrip_different_ciphertexts_for_same_plaintext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("test-passphrase-123", &salt)
            .expect("EncryptionService::new should succeed");
        let plaintext = b"hello world";
        let ct1 = service
            .encrypt(plaintext)
            .expect("first encrypt should succeed");
        let ct2 = service
            .encrypt(plaintext)
            .expect("second encrypt should succeed");
        assert_ne!(
            ct1, ct2,
            "two encryptions of the same plaintext must differ"
        );
    }

    // ── Curation: InvalidPassphrase guard ───────────────────────────

    // P8 invariant: empty passphrase is rejected before any crypto work
    #[test]
    fn encryption_service_rejects_empty_passphrase() {
        let salt = EncryptionService::generate_salt();
        let result = EncryptionService::new("", &salt);
        assert!(
            matches!(result, Err(EncryptionError::InvalidPassphrase)),
            "empty passphrase must be rejected with InvalidPassphrase"
        );
    }

    // P8 invariant: decrypting ciphertext with a different passphrase fails
    #[test]
    fn decryption_fails_with_wrong_passphrase() {
        let salt = EncryptionService::generate_salt();
        let service_a = EncryptionService::new("passphrase-A", &salt)
            .expect("service_a creation should succeed");
        let ciphertext = service_a
            .encrypt(b"secret")
            .expect("encrypt should succeed");
        let service_b = EncryptionService::new("passphrase-B", &salt)
            .expect("service_b creation should succeed");
        let result = service_b.decrypt(&ciphertext);
        assert!(
            result.is_err(),
            "decryption with wrong passphrase must fail"
        );
        assert!(
            matches!(result, Err(EncryptionError::Decryption(_))),
            "wrong passphrase must produce Decryption error, got {:?}",
            result
        );
    }

    // ── Persistence: truncated ciphertext rejection ────────────────

    // P8 invariant: ciphertext shorter than NONCE_SIZE bytes is rejected
    #[test]
    fn decrypt_rejects_truncated_ciphertext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("passphrase", &salt)
            .expect("EncryptionService::new should succeed");
        let result = service.decrypt(&[0u8; 5]);
        assert!(
            matches!(result, Err(EncryptionError::Decryption(_))),
            "ciphertext shorter than NONCE_SIZE must be rejected with Decryption error"
        );
    }

    // ── Domain: generate_salt ────────────────────────────────────────

    // P8 invariant: generated salt is not all zeros
    #[test]
    fn generate_salt_is_nonzero() {
        let salt = EncryptionService::generate_salt();
        assert_ne!(
            salt, [0u8; SALT_SIZE],
            "generated salt must not be all zeros"
        );
    }
}
