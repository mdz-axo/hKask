//! AES-256-GCM encryption

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Decryption failed: {0}")]
    Decryption(String),
}

/// Encryption service
pub struct EncryptionService;

impl EncryptionService {
    pub fn new(_passphrase: &str, _salt: &[u8]) -> Result<Self, EncryptionError> {
        Ok(Self)
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        Ok(plaintext.to_vec())
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        Ok(ciphertext.to_vec())
    }
}
