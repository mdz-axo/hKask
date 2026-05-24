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
    fn acquire(&self, key: &str, tokens: f64) -> Result<(), ValidationError> {
        let buckets = self.buckets.blocking_read();
        if let Some(bucket) = buckets.get(key) {
            if bucket.consume(tokens) {
                return Ok(());
            }
        } else {
            drop(buckets);
            let mut buckets = self.buckets.blocking_write();
            let bucket = buckets
                .entry(key.to_string())
                .or_insert_with(|| TokenBucket::new(self.default_max_tokens, self.default_refill_rate));
            if bucket.consume(tokens) {
                return Ok(());
            }
        }
        Err(ValidationError::RateLimitExceeded)
    }

    fn available(&self, key: &str) -> f64 {
        let buckets = self.buckets.blocking_read();
        buckets
            .get(key)
            .map(|b| b.available())
            .unwrap_or(self.default_max_tokens)
    }

    fn reset(&self, key: &str) {
        let mut buckets = self.buckets.blocking_write();
        buckets.remove(key);
    }
}

impl Default for RateLimiterAdapter {
    fn default() -> Self {
        Self::new(10.0, 1.0)
    }
}
