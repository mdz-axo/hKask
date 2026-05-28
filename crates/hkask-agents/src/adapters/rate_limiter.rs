//! Rate Limiter Adapter — Wraps unified hkask-cns RateLimiter
//!
//! Uses the unified RateLimiter from hkask-cns.

use crate::security::ValidationError;
use hkask_cns::rate_limit::{RateLimiter, StringRateLimiter};
use std::sync::Arc;

pub struct RateLimiterAdapter {
    inner: Arc<StringRateLimiter>,
}

impl RateLimiterAdapter {
    pub fn new(config: hkask_cns::rate_limit::RateLimitConfig) -> Self {
        Self {
            inner: Arc::new(RateLimiter::new(config)),
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            inner: Arc::new(StringRateLimiter::default()),
        }
    }

    pub fn acquire(&self, key: &str, _tokens: f64) -> Result<(), ValidationError> {
        if self.inner.check(&key.to_string()) {
            Ok(())
        } else {
            Err(ValidationError::RateLimitExceeded)
        }
    }

    pub fn available(&self, key: &str) -> f64 {
        self.inner.remaining(&key.to_string()) as f64
    }

    pub fn reset(&self, key: &str) {
        self.inner.reset(&key.to_string());
    }
}

impl Default for RateLimiterAdapter {
    fn default() -> Self {
        Self::with_defaults()
    }
}
