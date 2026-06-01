//! OS keychain integration

use hkask_types::SecretRef;
use hkask_types::WebID;
use hkask_types::derivation_contexts;
use keyring::{Entry, Error as KeyringError};
use thiserror::Error;
use tracing::warn;
use zeroize::Zeroizing;

#[derive(Error, Debug)]
pub enum KeychainError {
    #[error("Platform keychain error: {0}")]
    Platform(String),
    #[error("Secret not found: {0}")]
    NotFound(String),
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

    pub fn store_by_key(&self, key: &str, secret: &str) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .set_password(secret)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        Ok(())
    }

    pub fn retrieve_by_key(&self, key: &str) -> Result<String, KeychainError> {
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| KeychainError::NotFound(e.to_string()))
    }

    pub fn delete_by_key(&self, key: &str) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, key)
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

/// Get or create OCAP secret
///
/// Resolution chain:
/// 1. Deterministic derivation from master key (preferred — survives restarts)
/// 2. OS keychain (backward compat)
/// 3. Random generation (last resort — tokens will not survive restart)
pub fn get_or_create_ocap_secret() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    // Prefer deterministic derivation from master key
    let derived = resolve(&SecretRef::derived(
        derivation_contexts::MASTER_KEY_ENV,
        derivation_contexts::OCAP_SECRET,
    ));

    match derived {
        Ok(key) => Ok(key),
        Err(_) => {
            // Fallback to keychain for backward compat
            resolve(&SecretRef::Keychain("hkask-ocap-secret".to_string())).or_else(|_| {
                // Last resort: generate random (with warning)
                warn!(
                    "OCAP secret not available via derivation or keychain; \
                     generating random secret. Tokens will not survive restart."
                );
                let secret: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
                Ok(Zeroizing::new(secret))
            })
        }
    }
}

/// Resolve a SecretRef to actual secret bytes.
///
/// Resolution priority:
/// 1. `Env` — read from environment variable
/// 2. `Keychain` — read from OS keychain
/// 3. `Derived` — look up master key (env → keychain), then HKDF-SHA256 derive sub-key
/// 4. `Generated` — random bytes (⚠️ not reproducible; debug builds only)
///
/// For `Derived`, the master key is resolved first (env var → keychain),
/// then HKDF-SHA256 is applied with the given context string to produce
/// a deterministic 256-bit sub-key.
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
        SecretRef::Derived {
            master_key_env,
            context,
        } => {
            // Resolve master key: env var first, then keychain
            let master_key_bytes = resolve(&SecretRef::Env(master_key_env.clone()))
                .or_else(|_| resolve(&SecretRef::Keychain(master_key_env.clone())))
                .map_err(|_| {
                    KeychainError::NotFound(format!(
                        "Master key '{}' not found in environment or keychain; \
                     set {} or run `kask init` to derive secrets from a master passphrase",
                        master_key_env, master_key_env
                    ))
                })?;

            // HKDF-SHA256 derive sub-key
            let sub_key = crate::master_key::derive_sub_key(&master_key_bytes, context);
            Ok(sub_key)
        }
        #[cfg(debug_assertions)]
        SecretRef::Generated(length) => {
            let bytes: Vec<u8> = (0..*length as usize)
                .map(|_| rand::random::<u8>())
                .collect();
            Ok(Zeroizing::new(bytes))
        }
    }
}
