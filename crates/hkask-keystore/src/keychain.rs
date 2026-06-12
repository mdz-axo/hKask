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
        use KeyringError::*;
        match err {
            NoEntry => KeychainError::NotFound("secret not found in keychain".into()),
            other => KeychainError::Platform(other.to_string()),
        }
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
        let entry = Entry::new(&self.service_name, &webid.as_uuid().to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .set_password(secret)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        Ok(())
    }

    pub fn retrieve(&self, webid: &WebID) -> Result<String, KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.as_uuid().to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry.get_password().map_err(KeychainError::from)
    }

    pub fn delete(&self, webid: &WebID) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.as_uuid().to_string())
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

        entry.get_password().map_err(KeychainError::from)
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

//
// These functions encapsulate the standard 3-tier resolution chain
// (derived → env → keychain) for each well-known secret. Every call site
// that previously hand-rolled its own chain should use these instead.
//
// Benefits:
//   - Eliminates copy-paste drift (10+ independent copies collapsed to 1 implementation)
//   - Fixes the ACP env var inconsistency (HKASK_ACP_SECRET vs HKASK_ACP_SECRET_KEY)
//   - Single place to audit secret resolution behavior

/// Resolve a secret through the standard 3-tier chain:
/// 1. Master key derivation (HKDF-SHA256)
/// 2. Direct environment variable
/// 3. OS keychain lookup
///
/// This is the canonical resolution pattern for all hKask secrets.
/// Domain-specific functions (`resolve_acp_secret`, etc.) call this with
/// the appropriate parameters.
pub fn resolve_secret_chain(
    derivation_context: (&str, &str),
    env_var: &str,
    keychain_key: &str,
) -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve(&SecretRef::derived(
        derivation_context.0,
        derivation_context.1,
    ))
    .or_else(|_| resolve(&SecretRef::env(env_var)))
    .or_else(|_| resolve(&SecretRef::keychain(keychain_key)))
}

/// Resolve the ACP (Agent Capability Protocol) HMAC signing secret.
///
/// Chain: master key derivation → env var → OS keychain.
/// Tries both `HKASK_ACP_SECRET` (canonical) and `HKASK_ACP_SECRET_KEY` (legacy)
/// environment variables for backward compatibility.
pub fn resolve_acp_secret() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::ACP_SECRET,
        ),
        "HKASK_ACP_SECRET",
        "acp-secret",
    )
    .or_else(|_| resolve(&SecretRef::env("HKASK_ACP_SECRET_KEY")))
}

/// Resolve the MCP dispatch and tool invocation signing key.
///
/// Chain: master key derivation → env var → OS keychain → ACP fallback.
/// Falls back to the ACP secret if MCP-specific key is unavailable,
/// since they share the same authority chain.
pub fn resolve_mcp_secret() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::MCP_SECRET,
        ),
        "HKASK_MCP_SECRET",
        "mcp-secret",
    )
    .or_else(|_| resolve_acp_secret())
}

/// Resolve the MCP security gateway HMAC key (used for API auth).
///
/// Chain: master key derivation → env var → OS keychain.
pub fn resolve_mcp_security_key() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::MCP_SECURITY_KEY,
        ),
        "HKASK_MCP_SECURITY_KEY",
        "mcp-security-key",
    )
}

/// Resolve the capability token signing key (used for SOAP/capability tokens).
///
/// Chain: master key derivation → env var → OS keychain.
pub fn resolve_capability_key() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::CAPABILITY_KEY,
        ),
        "HKASK_CAPABILITY_KEY",
        "capability-key",
    )
}

/// Resolve the database encryption passphrase.
///
/// Chain: env var → OS keychain.
/// Note: no master-key derivation for the DB passphrase — it must be
/// explicitly set via env var or keychain to avoid accidentally encrypting
/// the database with a derived key that the user didn't consent to.
pub fn resolve_db_passphrase() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve(&SecretRef::env("HKASK_DB_PASSPHRASE"))
        .or_else(|_| resolve(&SecretRef::keychain("hkask-db-passphrase")))
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
            let secret = entry.get_password().map_err(KeychainError::from)?;
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
