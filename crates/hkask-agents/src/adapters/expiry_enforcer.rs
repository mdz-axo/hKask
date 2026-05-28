//! Expiry Enforcement Adapter

use crate::ports::security_port::{ExpiryPort, ValidationResult, ValidationError};
use std::time::Duration;

pub struct ExpiryEnforcerAdapter {
    max_lifetime: Duration,
}

impl ExpiryEnforcerAdapter {
    pub fn new(max_lifetime: Duration) -> Self {
        Self { max_lifetime }
    }
}

impl ExpiryPort for ExpiryEnforcerAdapter {
    fn calculate_expiry(&self, creation_time: i64) -> i64 {
        creation_time + self.max_lifetime.as_secs() as i64
    }

    fn is_expired(&self, expires_at: i64, current_time: i64) -> bool {
        current_time > expires_at
    }

    fn validate_expiry(&self, expires_at: i64, current_time: i64) -> ValidationResult<()> {
        if self.is_expired(expires_at, current_time) {
            Err(ValidationError::InvalidInput(
                "Capability token has expired".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn max_lifetime_secs(&self) -> u64 {
        self.max_lifetime.as_secs()
    }
}

impl Default for ExpiryEnforcerAdapter {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600))
    }
}
