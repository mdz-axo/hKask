//! OS keychain integration

use hkask_types::SecretRef;
use hkask_types::WebID;
use keyring::{Entry, Error as KeyringError};
use thiserror::Error;
use zeroize::Zeroizing;

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

/// Key ring for holding cryptographic keys
#[derive(Debug, Clone, zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
pub struct KeyRing {
    key: [u8; 32],
}

impl KeyRing {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn rotate(&mut self, new_key: [u8; 32]) {
        self.key = new_key;
    }

    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }
}

/// Get or create OCAP secret for a WebID
pub fn get_or_create_ocap_secret(
    keychain: &Keychain,
    webid: &WebID,
) -> Result<String, KeychainError> {
    // Try to retrieve existing secret
    match keychain.retrieve(webid) {
        Ok(secret) => Ok(secret),
        Err(KeychainError::NotFound(_)) => {
            // Generate new secret and store it
            let secret: String = (0..32)
                .map(|_| rand::random::<u8>())
                .map(|b| format!("{:02x}", b))
                .collect();
            keychain.store(webid, &secret)?;
            Ok(secret)
        }
        Err(e) => Err(e),
    }
}

/// Resolve a SecretRef to actual secret bytes
pub fn resolve(secret_ref: &SecretRef) -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    match secret_ref {
        SecretRef::Env(var_name) => {
            let value = std::env::var(var_name)
                .map_err(|_| KeychainError::NotFound(format!("env var {} not set", var_name)))?;
            Ok(Zeroizing::new(value.into_bytes()))
        }
        SecretRef::Keychain(key_name) => {
            let keychain = Keychain::default();
            let entry = Entry::new(&keychain.service_name, key_name)
                .map_err(|e| KeychainError::Platform(e.to_string()))?;
            let secret = entry
                .get_password()
                .map_err(|e| KeychainError::NotFound(e.to_string()))?;
            Ok(Zeroizing::new(secret.into_bytes()))
        }
        SecretRef::Generated(length) => {
            let bytes: Vec<u8> = (0..*length as usize)
                .map(|_| rand::random::<u8>())
                .collect();
            Ok(Zeroizing::new(bytes))
        }
    }
}
