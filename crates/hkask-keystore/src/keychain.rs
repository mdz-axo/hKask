//! OS keychain integration

use hkask_types::WebID;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeychainError {
    #[error("Keychain error: {0}")]
    Platform(String),
    #[error("Secret not found: {0}")]
    NotFound(String),
}

/// Keychain service
pub struct Keychain;

impl Keychain {
    pub fn new() -> Self {
        Self
    }

    pub fn store(&self, webid: &WebID, secret: &str) -> Result<(), KeychainError> {
        // Stub implementation
        let _ = (webid, secret);
        Ok(())
    }

    pub fn retrieve(&self, webid: &WebID) -> Result<String, KeychainError> {
        // Stub implementation
        let _ = webid;
        Err(KeychainError::NotFound("No secret stored".to_string()))
    }

    pub fn delete(&self, webid: &WebID) -> Result<(), KeychainError> {
        // Stub implementation
        let _ = webid;
        Ok(())
    }
}

impl Default for Keychain {
    fn default() -> Self {
        Self::new()
    }
}
