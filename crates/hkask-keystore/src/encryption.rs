//! AES-256-GCM encryption with Argon2 key derivation

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use thiserror::Error;
use zeroize::Zeroizing;

#[derive(Error, Debug)]
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

/// Salt size for Argon2
pub const SALT_SIZE: usize = 16;

/// Nonce size for AES-GCM
pub const NONCE_SIZE: usize = 12;

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
fn derive_key(passphrase: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, EncryptionError> {
    let mut key = Zeroizing::new([0u8; 32]);
    let params = Params::new(65536, 3, 4, Some(32))
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut *key)
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    Ok(key)
}

/// Prompt user for encryption passphrase interactively
pub fn prompt_passphrase(prompt: &str) -> Result<String, std::io::Error> {
    use std::io::{self, Write};

    print!("{}", prompt);
    io::stdout().flush()?;

    let mut passphrase = String::new();
    io::stdin().read_line(&mut passphrase)?;

    Ok(passphrase.trim().to_string())
}

/// Read passphrase from environment or prompt
pub fn get_passphrase(env_var: &str, prompt: &str) -> Result<Zeroizing<String>, std::io::Error> {
    if let Ok(passphrase) = std::env::var(env_var) {
        Ok(Zeroizing::new(passphrase))
    } else {
        prompt_passphrase(prompt).map(Zeroizing::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("test-passphrase", &salt).unwrap();

        let plaintext = b"Hello, World!";
        let ciphertext = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_different_passphrases() {
        let salt = EncryptionService::generate_salt();
        let service1 = EncryptionService::new("passphrase1", &salt).unwrap();
        let service2 = EncryptionService::new("passphrase2", &salt).unwrap();

        let plaintext = b"Secret data";
        let ciphertext = service1.encrypt(plaintext).unwrap();

        // Different passphrase should fail to decrypt
        let result = service2.decrypt(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_passphrase() {
        let salt = EncryptionService::generate_salt();
        let result = EncryptionService::new("", &salt);
        assert!(matches!(result, Err(EncryptionError::InvalidPassphrase)));
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = EncryptionService::generate_salt();
        let salt2 = EncryptionService::generate_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("passphrase", &salt).unwrap();
        let result = service.decrypt(&[0u8; 5]); // Too short
        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }

    #[test]
    fn test_decrypt_empty_ciphertext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("passphrase", &salt).unwrap();
        let result = service.decrypt(&[]);
        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }
}
