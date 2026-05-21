//! Rate Limiting Adapter — Token Bucket Implementation

use crate::ports::security_port::{RateLimitPort, ValidationResult, ValidationError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    pub fn consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }
}

pub struct RateLimiterAdapter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_max_tokens: f64,
    default_refill_rate: f64,
}

impl RateLimiterAdapter {
    pub fn new(default_max_tokens: f64, default_refill_rate: f64) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            default_max_tokens,
            default_refill_rate,
        }
    }

    pub async fn acquire_async(&self, key: &str, tokens: f64) -> Result<(), ValidationError> {
        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_max_tokens, self.default_refill_rate));

        if bucket.consume(tokens) {
            Ok(())
        } else {
            Err(ValidationError::RateLimitExceeded)
        }
    }
}

impl RateLimitPort for RateLimiterAdapter {
    fn acquire(&self, _key: &str, _tokens: f64) -> Result<(), ValidationError> {
        Err(ValidationError::RateLimitExceeded)
    }

    fn available(&self, _key: &str) -> f64 {
        self.default_max_tokens
    }

    fn reset(&self, _key: &str) {}
}

impl Default for RateLimiterAdapter {
    fn default() -> Self {
        Self::new(10.0, 1.0)
    }
}
