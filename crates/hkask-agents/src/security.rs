//! Security Hardening — Input Validation and OCAP Enhancement
//!
//! This module provides:
//! - **Input Validation**: Schema-based validation for pod operations
//! - **OCAP Enhancement**: Attenuation history tracking and expiry enforcement

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Input validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Field too long: {field} (max {max} chars)")]
    FieldTooLong { field: String, max: usize },
    #[error("Invalid format: {field}")]
    InvalidFormat { field: String },
}

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationError>;

/// OCAP attenuation history tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttenuationHistory {
    pub root_nonce: String,
    pub chain: Vec<AttenuationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttenuationEntry {
    pub delegated_from: String,
    pub delegated_to: String,
    pub timestamp: i64,
    pub attenuation_level: u8,
}

impl AttenuationHistory {
    pub fn new(root_nonce: String) -> Self {
        Self {
            root_nonce,
            chain: vec![],
        }
    }

    pub fn add_entry(
        &mut self,
        delegated_from: String,
        delegated_to: String,
        timestamp: i64,
        attenuation_level: u8,
    ) {
        self.chain.push(AttenuationEntry {
            delegated_from,
            delegated_to,
            timestamp,
            attenuation_level,
        });
    }

    pub fn verify_chain(&self) -> bool {
        // Verify attenuation levels are sequential
        for i in 1..self.chain.len() {
            if self.chain[i].attenuation_level != self.chain[i - 1].attenuation_level + 1 {
                return false;
            }
        }
        true
    }

    pub fn chain_length(&self) -> usize {
        self.chain.len()
    }
}

/// Capability expiry enforcer
pub struct ExpiryEnforcer {
    max_lifetime: Duration,
}

impl ExpiryEnforcer {
    pub fn new(max_lifetime: Duration) -> Self {
        Self { max_lifetime }
    }

    pub fn calculate_expiry(&self, creation_time: i64) -> i64 {
        creation_time + self.max_lifetime.as_secs() as i64
    }

    pub fn is_expired(&self, expires_at: i64, current_time: i64) -> bool {
        current_time > expires_at
    }

    pub fn validate_expiry(&self, expires_at: i64, current_time: i64) -> ValidationResult<()> {
        if self.is_expired(expires_at, current_time) {
            Err(ValidationError::InvalidInput(
                "Capability token has expired".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn max_lifetime_secs(&self) -> u64 {
        self.max_lifetime.as_secs()
    }
}

impl Default for ExpiryEnforcer {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // 1 hour default
    }
}

/// Security context for pod operations
#[derive(Default)]
pub struct SecurityContext {
    pub expiry_enforcer: ExpiryEnforcer,
}

impl SecurityContext {
    pub fn new(expiry_enforcer: ExpiryEnforcer) -> Self {
        Self { expiry_enforcer }
    }
}
