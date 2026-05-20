//! OS keychain integration

use hkask_types::WebID;
use keyring::{Entry, Error as KeyringError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeychainError {
    #[error("Platform keychain error: {0}")]
    Platform(String),
    #[error("Secret not found: {0}")]
    NotFound(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
}

impl From<KeyringError> for KeychainError {
    fn from(err: KeyringError) -> Self {
        KeychainError::Platform(err.to_string())
    }
}

/// Keychain service for secure credential storage
pub struct Keychain {
    service_name: String,
}

impl Keychain {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    pub fn store(&self, webid: &WebID, secret: &str) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.0.to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .set_password(secret)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        Ok(())
    }

    pub fn retrieve(&self, webid: &WebID) -> Result<String, KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.0.to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| KeychainError::NotFound(e.to_string()))
    }

    pub fn delete(&self, webid: &WebID) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.0.to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        Ok(())
    }
}

impl Default for Keychain {
    fn default() -> Self {
        Self::new("hkask")
    }
}

/// Key ring for macaroon key rotation
#[derive(Debug, Clone)]
pub struct KeyRing {
    current_key: [u8; 32],
    previous_keys: Vec<[u8; 32]>,
}

impl KeyRing {
    pub fn new(current_key: [u8; 32]) -> Self {
        Self {
            current_key,
            previous_keys: Vec::new(),
        }
    }

    pub fn current_key(&self) -> &[u8; 32] {
        &self.current_key
    }

    pub fn previous_keys(&self) -> &[[u8; 32]] {
        &self.previous_keys
    }

    pub fn rotate(&mut self, new_key: [u8; 32]) {
        self.previous_keys.push(self.current_key);
        self.current_key = new_key;
    }

    pub fn verify_with_rotation(&self, key_to_check: &[u8; 32]) -> bool {
        if key_to_check == &self.current_key {
            return true;
        }
        self.previous_keys.contains(key_to_check)
    }
}

/// Generate secure random key for macaroons
pub fn generate_macaroon_key() -> [u8; 32] {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    key
}

