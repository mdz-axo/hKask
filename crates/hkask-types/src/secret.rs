use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretRef {
    Env(String),
    Keychain(String),
    Generated(u32),
}

impl SecretRef {
    pub fn env(name: &str) -> Self {
        Self::Env(name.to_string())
    }

    pub fn keychain(service: &str) -> Self {
        Self::Keychain(service.to_string())
    }

    pub fn generated(length: u32) -> Self {
        Self::Generated(length)
    }
}
