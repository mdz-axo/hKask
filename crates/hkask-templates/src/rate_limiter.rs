//! Rate Limiting for Manifest Operations
//!
//! Re-exports unified rate limiter from hkask-cns.
//! Provides rate-limited repository wrapper for manifest operations.

pub use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter, StringRateLimiter};
use std::time::Duration;

/// Rate limit exceeded error
#[derive(Debug, thiserror::Error)]
#[error("Rate limit exceeded: {message}")]
pub struct RateLimitExceededError {
    pub message: String,
    pub retry_after: Duration,
}

impl RateLimitExceededError {
    pub fn new(message: &str, retry_after: Duration) -> Self {
        Self {
            message: message.to_string(),
            retry_after,
        }
    }
}

/// Rate-limited manifest repository wrapper
pub struct RateLimitedRepository<R> {
    inner: R,
    rate_limiter: StringRateLimiter,
}

impl<R> RateLimitedRepository<R> {
    /// Create rate-limited repository
    pub fn new(inner: R, config: RateLimitConfig) -> Self {
        Self {
            inner,
            rate_limiter: RateLimiter::new(config),
        }
    }

    /// Get reference to inner repository
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Get mutable reference to inner repository
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Check rate limit before operation
    pub fn check_rate_limit(&self, operation: &str) -> Result<(), RateLimitExceededError> {
        if !self.rate_limiter.check(&operation.to_string()) {
            let retry_after = Duration::from_millis(
                self.rate_limiter.remaining(&operation.to_string()).max(1) as u64 * 100,
            );
            return Err(RateLimitExceededError::new(
                "Manifest operation rate limit exceeded",
                retry_after,
            ));
        }
        Ok(())
    }
}
