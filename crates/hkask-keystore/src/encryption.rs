//! AES-256-GCM encryption with Argon2 key derivation

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use std::time::Instant;
use thiserror::Error;
use tracing::info;
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
    ///
    // REQ: P9-CNS-KS-001 pre: operation valid, post: cns.keystore span emitted
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        // P9: CNS span
        info!(target: "cns.keystore", operation = "encrypt", plaintext_len = plaintext.len(), status = "started", "CNS");
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

        // P9: CNS span
        info!(target: "cns.keystore", operation = "encrypt", plaintext_len = plaintext.len(), status = "completed", "CNS");
        Ok(result)
    }

    /// Decrypt ciphertext data
    ///
    // REQ: P9-CNS-KS-001 pre: operation valid, post: cns.keystore span emitted
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        // P9: CNS span
        info!(target: "cns.keystore", operation = "decrypt", ciphertext_len = ciphertext.len(), status = "started", "CNS");
        if ciphertext.len() < NONCE_SIZE {
            return Err(EncryptionError::Decryption(
                "Ciphertext too short".to_string(),
            ));
        }

        let nonce_bytes = &ciphertext[..NONCE_SIZE];
        let data = &ciphertext[NONCE_SIZE..];

        let nonce = Nonce::from_slice(nonce_bytes);

        let result = self
            .cipher
            .decrypt(nonce, data)
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;
        // P9: CNS span
        info!(target: "cns.keystore", operation = "decrypt", ciphertext_len = ciphertext.len(), status = "completed", "CNS");
        Ok(result)
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
    // P9: CNS span
    let start = Instant::now();
    info!(target: "cns.keystore", operation = "derive_key", status = "started", "CNS");
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
    // P9: CNS span
    let latency_ms = start.elapsed().as_millis() as u64;
    info!(target: "cns.keystore", operation = "derive_key", status = "completed", latency_ms = latency_ms, "CNS");
    Ok(key)
}
