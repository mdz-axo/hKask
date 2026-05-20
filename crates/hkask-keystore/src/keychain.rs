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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_new() {
        let keychain = Keychain::new("test-service");
        assert_eq!(keychain.service_name, "test-service");
    }

    #[test]
    fn test_keychain_default() {
        let keychain = Keychain::default();
        assert_eq!(keychain.service_name, "hkask");
    }
}
