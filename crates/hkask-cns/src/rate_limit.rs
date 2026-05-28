//! Rate limiting for hKask — Unified implementation
//!
//! Provides token bucket rate limiting with multiple strategies:
//! - Per-key limiting (WebID, tool name, etc.)
//! - Configurable token counts and refill rates
//! - Thread-safe with async support
//!
//! This consolidates 5 duplicate rate limiter implementations across:
//! - hkask-cns (this file)
//! - hkask-agents/security.rs
//! - hkask-agents/adapters/rate_limiter.rs
//! - hkask-templates/rate_limiter.rs
//! - hkask-mcp-web/rate_limiter.rs

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
pub struct CnsTokenBucket {
    tokens: u32,
    last_refill: Instant,
    config: RateLimitConfig,
}

impl CnsTokenBucket {
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

/// Unified rate limiter for hKask
///
/// Generic over key type K (WebID, String, tool name, etc.)
/// Thread-safe with parking_lot::Mutex for performance.
pub struct RateLimiter<K = WebID> {
    buckets: Mutex<HashMap<K, CnsTokenBucket>>,
    config: RateLimitConfig,
}

impl<K: std::hash::Hash + Eq + Clone> RateLimiter<K> {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// Check if a key can proceed. Returns true if allowed.
    pub fn check(&self, key: &K) -> bool {
        let mut buckets = self.buckets.lock();

        let bucket = buckets
            .entry(key.clone())
            .or_insert_with(|| CnsTokenBucket::new(self.config.clone()));

        bucket.try_consume()
    }

    /// Get remaining tokens for a key
    pub fn remaining(&self, key: &K) -> u32 {
        let mut buckets = self.buckets.lock();

        let bucket = buckets
            .entry(key.clone())
            .or_insert_with(|| CnsTokenBucket::new(self.config.clone()));

        bucket.tokens()
    }

    /// Update rate limit config for a specific key
    pub fn configure(&self, key: &K, config: RateLimitConfig) {
        let mut buckets = self.buckets.lock();
        buckets.insert(key.clone(), CnsTokenBucket::new(config));
    }

    /// Remove a key's bucket (reset)
    pub fn reset(&self, key: &K) {
        let mut buckets = self.buckets.lock();
        buckets.remove(key);
    }
}

impl<K: std::hash::Hash + Eq + Clone> Default for RateLimiter<K> {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

/// Type alias for WebID-based rate limiter (most common case)
pub type WebIdRateLimiter = RateLimiter<WebID>;

/// Type alias for String-based rate limiter (tool names, etc.)
pub type StringRateLimiter = RateLimiter<String>;
