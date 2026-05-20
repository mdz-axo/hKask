//! Rate limiting for template dispatch
//!
//! Implements token bucket rate limiting per bot/WebID.
//! Algedonic alert on rate limit exceeded.

use hkask_types::WebID;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum tokens (requests) allowed in the bucket
    pub max_tokens: u32,
    /// Time to refill one token
    pub refill_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        // Default: 100 requests per minute
        Self {
            max_tokens: 100,
            refill_interval: Duration::from_millis(600), // 60000ms / 100 = 600ms per token
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug)]
pub struct TokenBucket {
    tokens: u32,
    last_refill: Instant,
    config: RateLimitConfig,
}

impl TokenBucket {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: config.max_tokens,
            last_refill: Instant::now(),
            config,
        }
    }

    /// Try to consume a token. Returns true if successful.
    pub fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        let tokens_to_add = (elapsed.as_millis() / self.config.refill_interval.as_millis()) as u32;

        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.config.max_tokens);
            self.last_refill = now;
        }
    }

    /// Get current token count
    pub fn tokens(&self) -> u32 {
        self.tokens
    }
}

/// Rate limiter for template dispatch
pub struct RateLimiter {
    buckets: Mutex<HashMap<WebID, TokenBucket>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// Check if a bot can dispatch. Returns true if allowed.
    pub fn check(&self, bot_id: &WebID) -> bool {
        let mut buckets = self.buckets.lock();

        let bucket = buckets
            .entry(*bot_id)
            .or_insert_with(|| TokenBucket::new(self.config.clone()));

        bucket.try_consume()
    }

    /// Get remaining tokens for a bot
    pub fn remaining(&self, bot_id: &WebID) -> u32 {
        let mut buckets = self.buckets.lock();

        let bucket = buckets
            .entry(*bot_id)
            .or_insert_with(|| TokenBucket::new(self.config.clone()));

        bucket.tokens()
    }

    /// Update rate limit config for a specific bot
    pub fn configure_bot(&self, bot_id: &WebID, config: RateLimitConfig) {
        let mut buckets = self.buckets.lock();
        buckets.insert(*bot_id, TokenBucket::new(config));
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

