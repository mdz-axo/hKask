//! Rate Limiting for Manifest Operations
//!
//! Implements token bucket rate limiting to prevent DoS attacks and resource exhaustion.
//! Per architecture v0.21.0: rate limits are configurable per deployment.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Token bucket rate limiter
///
/// Allows burst of requests up to `max_tokens`, then refills at `refill_rate` tokens per second.
/// Thread-safe and lock-free for the common case (tokens available).
pub struct RateLimiter {
    /// Maximum tokens in bucket (burst capacity)
    max_tokens: u64,
    /// Tokens added per second
    refill_rate: u64,
    /// Current token count (atomic for lock-free reads)
    tokens: AtomicU64,
    /// Last refill time
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    /// Create new rate limiter
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum burst capacity
    /// * `refill_rate` - Tokens added per second
    pub fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            max_tokens,
            refill_rate,
            tokens: AtomicU64::new(max_tokens),
            last_refill: Mutex::new(Instant::now()),
        }
    }

    /// Create rate limiter with sensible defaults
    /// Default: 10 requests/second burst, 100 requests/second sustained
    pub fn with_defaults() -> Self {
        Self::new(10, 100)
    }

    /// Try to consume a token
    ///
    /// Returns `true` if token was consumed (request allowed),
    /// `false` if rate limit exceeded (request should be rejected).
    pub fn try_acquire(&self) -> bool {
        // First try lock-free path
        let current = self.tokens.load(Ordering::Relaxed);
        if current == 0 {
            // Need to refill - take lock
            self.refill();
            // Try again after refill
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
        }

        // Try to consume token
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
            if self
                .tokens
                .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
            // CAS failed, retry
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&self) {
        let mut last_refill = self.last_refill.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        // Calculate tokens to add
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate as f64) as u64;

        if tokens_to_add > 0 {
            let current = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current + tokens_to_add).min(self.max_tokens);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last_refill = now;
        }
    }

    /// Get current token count (for monitoring)
    pub fn tokens_available(&self) -> u64 {
        self.tokens.load(Ordering::Relaxed)
    }

    /// Get maximum tokens (burst capacity)
    pub fn max_tokens(&self) -> u64 {
        self.max_tokens
    }

    /// Get refill rate (tokens per second)
    pub fn refill_rate(&self) -> u64 {
        self.refill_rate
    }
}

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
    #[allow(dead_code)]
    rate_limiter: RateLimiter,
}

impl<R> RateLimitedRepository<R> {
    /// Create rate-limited repository
    pub fn new(inner: R, max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            inner,
            rate_limiter: RateLimiter::new(max_tokens, refill_rate),
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
    fn check_rate_limit(&self) -> Result<(), RateLimitExceededError> {
        if !self.rate_limiter.try_acquire() {
            // Calculate retry-after based on refill rate
            let retry_after = Duration::from_secs_f64(1.0 / self.rate_limiter.refill_rate() as f64);
            return Err(RateLimitExceededError::new(
                "Manifest operation rate limit exceeded",
                retry_after,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter_allows_burst() {
        let limiter = RateLimiter::new(5, 10);

        // Should allow 5 requests immediately (burst)
        for i in 0..5 {
            assert!(limiter.try_acquire(), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_after_burst() {
        let limiter = RateLimiter::new(2, 1);

        // Exhaust tokens
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());

        // Next request should be blocked
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(2, 100); // 100 tokens/second

        // Exhaust tokens
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());

        // Wait for refill (10ms should add ~1 token at 100/sec)
        thread::sleep(Duration::from_millis(20));

        // Should have tokens again
        assert!(limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_tokens_available() {
        let limiter = RateLimiter::new(10, 100);

        assert_eq!(limiter.tokens_available(), 10);

        limiter.try_acquire();
        assert_eq!(limiter.tokens_available(), 9);

        limiter.try_acquire();
        assert_eq!(limiter.tokens_available(), 8);
    }

    #[test]
    fn test_rate_limiter_max_cap() {
        let limiter = RateLimiter::new(5, 100);

        // Wait for potential over-refill
        thread::sleep(Duration::from_millis(100));

        // Should still be capped at max
        assert_eq!(limiter.tokens_available(), 5);
    }

    #[test]
    fn test_rate_limit_exceeded_error() {
        let err = RateLimitExceededError::new("test error", Duration::from_secs(1));

        assert_eq!(err.message, "test error");
        assert_eq!(err.retry_after, Duration::from_secs(1));
        assert!(err.to_string().contains("Rate limit exceeded"));
    }
}
