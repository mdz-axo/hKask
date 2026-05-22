//! Keystore Port
//!
//! Trait for secure key/value storage with OS keychain integration.

use serde::{Deserialize, Serialize};

/// Keystore Port — Secure storage for secrets
pub trait KeystorePort {
    /// Store a secret value
    fn set(&self, key: &str, value: &str, service: &str) -> Result<(), String>;

    /// Retrieve a secret value
    fn get(&self, key: &str, service: &str) -> Result<String, String>;

    /// Rotate a secret value
    fn rotate(&self, key: &str, new_value: &str, service: &str) -> Result<(), String>;

    /// Delete a secret value
    fn delete(&self, key: &str, service: &str) -> Result<(), String>;

    /// List all keys for a service
    fn list(&self, service: &str) -> Result<Vec<String>, String>;

    /// Prompt user for a secret value
    fn prompt(&self, prompt_text: &str) -> Result<String, String>;
}

/// Secret wrapper for type safety
#[derive(Clone, Serialize, Deserialize)]
pub struct Secret<T> {
    inner: T,
}

impl<T> Secret<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn get(&self) -> &T {
        &self.inner
    }
}

impl<T: AsRef<str>> std::fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret([REDACTED])")
    }
}
