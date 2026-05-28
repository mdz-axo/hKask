//! Keychain Adapter — Concrete implementation using hkask-keystore

use hkask_keystore::KeystoreError;
use hkask_keystore::keychain::Keychain;
use hkask_types::WebID;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct KeychainAdapter {
    keychain: Keychain,
}

impl KeychainAdapter {
    pub fn new(service_name: &str) -> Self {
        Self {
            keychain: Keychain::new(service_name),
        }
    }

    pub fn default_service() -> Self {
        Self {
            keychain: Keychain::default(),
        }
    }

    fn key_to_webid(key: &str, service: &str) -> WebID {
        let combined = format!("{}:{}", service, key);
        let hash = Sha256::digest(combined.as_bytes());
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes.copy_from_slice(&hash[..16]);
        uuid_bytes[6] = (uuid_bytes[6] & 0x0f) | 0x40;
        uuid_bytes[8] = (uuid_bytes[8] & 0x3f) | 0x80;
        WebID(Uuid::from_bytes(uuid_bytes))
    }

    pub fn set(&self, key: &str, value: &str, service: &str) -> Result<(), KeystoreError> {
        let webid = Self::key_to_webid(key, service);
        self.keychain
            .store(&webid, value)
            .map_err(KeystoreError::from)
    }

    pub fn get(&self, key: &str, service: &str) -> Result<String, KeystoreError> {
        let webid = Self::key_to_webid(key, service);
        self.keychain.retrieve(&webid).map_err(KeystoreError::from)
    }

    pub fn rotate(&self, key: &str, new_value: &str, service: &str) -> Result<(), KeystoreError> {
        self.set(key, new_value, service)
    }

    pub fn delete(&self, key: &str, service: &str) -> Result<(), KeystoreError> {
        let webid = Self::key_to_webid(key, service);
        self.keychain.delete(&webid).map_err(KeystoreError::from)
    }

    pub fn list(&self, _service: &str) -> Result<Vec<String>, KeystoreError> {
        Err(KeystoreError::NotSupported(
            "Keychain does not support listing keys".to_string(),
        ))
    }

    pub fn prompt(&self, prompt_text: &str) -> Result<String, KeystoreError> {
        eprint!("{}: ", prompt_text);
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| KeystoreError::Io(e.to_string()))?;
        Ok(input.trim().to_string())
    }
}

impl Default for KeychainAdapter {
    fn default() -> Self {
        Self::default_service()
    }
}
