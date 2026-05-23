//! Rate Limiting Adapter — Token Bucket Implementation

use crate::ports::security_port::{RateLimitPort, ValidationError};
use hkask_types::TokenBucket;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
