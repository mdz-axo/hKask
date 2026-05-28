//! Rate Limiter Adapter — Wraps unified hkask-cns RateLimiter for port interface
//!
//! This adapter implements the RateLimitPort trait using the unified RateLimiter
//! from hkask-cns, eliminating duplicate token bucket implementations.

use crate::ports::security_port::{RateLimitPort, ValidationError};
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
}

impl RateLimitPort for RateLimiterAdapter {
    fn acquire(&self, key: &str, _tokens: f64) -> Result<(), ValidationError> {
        if self.inner.check(&key.to_string()) {
            Ok(())
        } else {
            Err(ValidationError::RateLimitExceeded)
        }
    }

    fn available(&self, key: &str) -> f64 {
        self.inner.remaining(&key.to_string()) as f64
    }

    fn reset(&self, key: &str) {
        self.inner.reset(&key.to_string());
    }
}

impl Default for RateLimiterAdapter {
    fn default() -> Self {
        Self::with_defaults()
    }
}
