//! ThrottleBucket — per-agent token-bucket rate limiting (Loop 6)
//!
//! CNS owns throttling as a cybernetic concern: resource governance that
//! prevents any single agent from monopolising shared capacity.
//!
//! The token-bucket algorithm gives each agent `burst` tokens max, refilling
//! at `tokens_per_minute / 60` per second. `check_and_consume` is atomic:
//! it refills elapsed tokens then deducts one, returning `false` when empty.

use hkask_types::WebID;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::debug;

/// Per-agent bucket state (not shared across agents).
#[derive(Debug)]
struct TokenBucketState {
    /// Current token count (fractional for smooth refill).
    tokens: f64,
    /// Timestamp of last refill.
    last_refill: Instant,
}

impl TokenBucketState {
    fn new(burst: u32) -> Self {
        Self {
            tokens: burst as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time, then try to consume one.
    /// Returns `true` if a token was consumed, `false` if rate-limited.
    fn refill_and_consume(&mut self, refill_per_sec: f64, max_tokens: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * refill_per_sec).min(max_tokens);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Configuration for a `ThrottleBucket`.
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    /// Maximum tokens an agent can accumulate (burst capacity).
    pub burst: u32,
    /// Tokens added per minute (refill rate).
    pub tokens_per_minute: u32,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            burst: 10,
            tokens_per_minute: 60,
        }
    }
}

/// Per-agent token-bucket throttle keyed by `WebID`.
///
/// Thread-safe: uses `tokio::sync::Mutex` for async-compatible interior
/// mutability. Each `check_and_consume` call locks, refills, and consumes
/// atomically.
///
/// # Cybernetic placement
///
/// Throttling is a Loop 6 (regulation) concern. The CNS owns the canonical
/// throttle primitive; MCP servers running in separate processes maintain
/// local `RateBucket` proxies as fallback for process-isolated throttling.
#[derive(Debug, Clone)]
pub struct ThrottleBucket {
    inner: Arc<Mutex<HashMap<WebID, TokenBucketState>>>,
    config: ThrottleConfig,
}

impl ThrottleBucket {
    /// Create a new `ThrottleBucket` with the given configuration.
    pub fn with_config(tokens_per_minute: u32, burst: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            config: ThrottleConfig {
                burst,
                tokens_per_minute,
            },
        }
    }

    /// Check the rate limit for an agent and consume a token if available.
    ///
    /// Returns `true` if the request is allowed (token consumed), `false`
    /// if the agent is rate-limited.
    pub async fn check_and_consume(&self, agent: WebID) -> bool {
        let refill_per_sec = self.config.tokens_per_minute as f64 / 60.0;
        let max_tokens = self.config.burst as f64;

        let mut buckets = self.inner.lock().await;
        let bucket = buckets
            .entry(agent)
            .or_insert_with(|| TokenBucketState::new(self.config.burst));

        let allowed = bucket.refill_and_consume(refill_per_sec, max_tokens);
        if !allowed {
            debug!(
                target: "cns.throttle",
                agent = ?agent,
                "Rate limit exceeded"
            );
        }
        allowed
    }

    /// Get the current token count for an agent (observability).
    ///
    /// Does not consume or refill — snapshot only.
    pub async fn tokens_for(&self, agent: WebID) -> Option<f64> {
        let buckets = self.inner.lock().await;
        buckets.get(&agent).map(|b| b.tokens)
    }

    /// Reset an agent's bucket (e.g. after admin override).
    pub async fn reset(&self, agent: WebID) {
        let mut buckets = self.inner.lock().await;
        buckets.remove(&agent);
    }
}

impl Default for ThrottleBucket {
    fn default() -> Self {
        Self::with_config(60, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;

    fn webid() -> WebID {
        WebID::new()
    }

    #[tokio::test]
    async fn consume_within_burst() {
        let bucket = ThrottleBucket::with_config(60, 10);
        let agent = webid();

        for i in 0..10 {
            assert!(
                bucket.check_and_consume(agent).await,
                "Token {i} should be consumed"
            );
        }
    }

    #[tokio::test]
    async fn consume_exceeds_burst() {
        let bucket = ThrottleBucket::with_config(60, 3);
        let agent = webid();

        assert!(bucket.check_and_consume(agent).await);
        assert!(bucket.check_and_consume(agent).await);
        assert!(bucket.check_and_consume(agent).await);
        assert!(
            !bucket.check_and_consume(agent).await,
            "Should be rate-limited"
        );
    }

    #[tokio::test]
    async fn refill_after_time() {
        let bucket = ThrottleBucket::with_config(60, 3);
        let agent = webid();

        // Exhaust the bucket
        for _ in 0..3 {
            bucket.check_and_consume(agent).await;
        }
        assert!(!bucket.check_and_consume(agent).await);

        // Simulate time passing by directly manipulating bucket state
        let mut buckets = bucket.inner.lock().await;
        let state = buckets.get_mut(&agent).unwrap();
        state.last_refill = Instant::now() - std::time::Duration::from_secs(2);
        drop(buckets);

        // After 2 seconds, should have ~2 tokens refilled (60/60 = 1/sec)
        assert!(bucket.check_and_consume(agent).await);
    }

    #[tokio::test]
    async fn different_agents_independent() {
        let bucket = ThrottleBucket::with_config(60, 2);
        let agent_a = webid();
        let agent_b = webid();

        // Exhaust agent A
        assert!(bucket.check_and_consume(agent_a).await);
        assert!(bucket.check_and_consume(agent_a).await);
        assert!(!bucket.check_and_consume(agent_a).await);

        // Agent B should still have full bucket
        assert!(bucket.check_and_consume(agent_b).await);
        assert!(bucket.check_and_consume(agent_b).await);
    }

    #[tokio::test]
    async fn reset_clears_bucket() {
        let bucket = ThrottleBucket::with_config(60, 2);
        let agent = webid();

        assert!(bucket.check_and_consume(agent).await);
        bucket.reset(agent).await;
        // After reset, bucket is recreated with full tokens on next access
        assert!(bucket.check_and_consume(agent).await);
    }

    #[tokio::test]
    async fn tokens_for_snapshot() {
        let bucket = ThrottleBucket::with_config(60, 10);
        let agent = webid();

        assert_eq!(bucket.tokens_for(agent).await, None); // Not yet created
        bucket.check_and_consume(agent).await;
        assert_eq!(bucket.tokens_for(agent).await, Some(9.0));
    }

    #[tokio::test]
    async fn default_config() {
        let bucket = ThrottleBucket::default();
        let agent = webid();

        // Default: 60 tokens/min, burst 10
        for i in 0..10 {
            assert!(
                bucket.check_and_consume(agent).await,
                "Token {i} should be consumed"
            );
        }
        assert!(!bucket.check_and_consume(agent).await);
    }
}
