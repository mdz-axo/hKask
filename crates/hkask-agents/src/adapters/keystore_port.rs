//! Keystore Port
//!
//! Trait for secure key/value storage with OS keychain integration.

use hkask_keystore::KeystoreError;
use serde::{Deserialize, Serialize};

/// Keystore Port — Secure storage for secrets
pub trait KeystorePort: Send + Sync {
    fn set(&self, key: &str, value: &str, service: &str) -> Result<(), KeystoreError>;

    fn get(&self, key: &str, service: &str) -> Result<String, KeystoreError>;

    fn rotate(&self, key: &str, new_value: &str, service: &str) -> Result<(), KeystoreError>;

    fn delete(&self, key: &str, service: &str) -> Result<(), KeystoreError>;

    fn list(&self, service: &str) -> Result<Vec<String>, KeystoreError>;

    fn prompt(&self, prompt_text: &str) -> Result<String, KeystoreError>;
}

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
