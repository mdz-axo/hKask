//! Keychain Adapter — Concrete implementation of KeystorePort using hkask-keystore

use crate::adapters::keystore_port::KeystorePort;
use hkask_keystore::keychain::Keychain;
use hkask_types::WebID;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Keychain Adapter — Bridges KeystorePort to OS keychain via hkask-keystore
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
}

impl Default for KeychainAdapter {
    fn default() -> Self {
        Self::default_service()
    }
}

impl KeystorePort for KeychainAdapter {
    fn set(&self, key: &str, value: &str, service: &str) -> Result<(), String> {
        let webid = Self::key_to_webid(key, service);
        self.keychain
            .store(&webid, value)
            .map_err(|e| e.to_string())
    }

    fn get(&self, key: &str, service: &str) -> Result<String, String> {
        let webid = Self::key_to_webid(key, service);
        self.keychain.retrieve(&webid).map_err(|e| e.to_string())
    }

    fn rotate(&self, key: &str, new_value: &str, service: &str) -> Result<(), String> {
        self.set(key, new_value, service)
    }

    fn delete(&self, key: &str, service: &str) -> Result<(), String> {
        let webid = Self::key_to_webid(key, service);
        self.keychain.delete(&webid).map_err(|e| e.to_string())
    }

    fn list(&self, _service: &str) -> Result<Vec<String>, String> {
        Err("Keychain does not support listing keys".to_string())
    }

    fn prompt(&self, prompt_text: &str) -> Result<String, String> {
        eprint!("{}: ", prompt_text);
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;
        Ok(input.trim().to_string())
    }
}
