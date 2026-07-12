//! Retry configuration for CNS operations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,
    #[serde(default)]
    pub retryable_status: Vec<u16>,
}

fn default_multiplier() -> f64 {
    2.0
}

impl RetryConfig {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  attempt >= 0; self.initial_delay_ms, self.multiplier, self.max_delay_ms are valid
    /// post: returns the exponential backoff delay in ms, capped at self.max_delay_ms
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32);
        (delay as u64).min(self.max_delay_ms)
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  status is a valid HTTP status code (u16)
    /// post: returns true if status is in the retryable_status list
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_status.contains(&status)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            multiplier: 2.0,
            retryable_status: vec![408, 429, 500, 502, 503, 504],
        }
    }
}

#[cfg(test)]
mod retry_config_tests {
    use super::*;

    fn test_config() -> RetryConfig {
        RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            multiplier: 2.0,
            retryable_status: vec![429, 503],
        }
    }

    #[test]
    fn first_attempt_is_initial_delay() {
        let cfg = test_config();
        assert_eq!(cfg.delay_for_attempt(0), 100);
    }

    #[test]
    fn delay_doubles_each_attempt() {
        let cfg = test_config();
        assert_eq!(cfg.delay_for_attempt(1), 200);
        assert_eq!(cfg.delay_for_attempt(2), 400);
        assert_eq!(cfg.delay_for_attempt(3), 800);
    }

    #[test]
    fn delay_capped_at_max() {
        let cfg = test_config();
        let delay = cfg.delay_for_attempt(10); // 100 * 2^10 = 102400
        assert_eq!(delay, cfg.max_delay_ms);
    }

    #[test]
    fn delay_with_multiplier_one_is_constant() {
        let cfg = RetryConfig {
            multiplier: 1.0,
            ..test_config()
        };
        assert_eq!(cfg.delay_for_attempt(0), 100);
        assert_eq!(cfg.delay_for_attempt(5), 100);
    }

    #[test]
    fn default_config_is_reasonable() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert!(cfg.initial_delay_ms > 0);
        assert!(cfg.max_delay_ms > cfg.initial_delay_ms);
        assert!(!cfg.retryable_status.is_empty());
    }

    #[test]
    fn is_retryable_status_matches() {
        let cfg = test_config();
        assert!(cfg.is_retryable_status(429));
        assert!(cfg.is_retryable_status(503));
        assert!(!cfg.is_retryable_status(200));
        assert!(!cfg.is_retryable_status(404));
    }

    #[test]
    fn default_retryable_statuses() {
        let cfg = RetryConfig::default();
        // Standard retryable HTTP status codes
        assert!(cfg.is_retryable_status(429)); // Too Many Requests
        assert!(cfg.is_retryable_status(503)); // Service Unavailable
        assert!(!cfg.is_retryable_status(200)); // OK
    }

    // ── Regression: mutation corruption guard ────────────────────────────
    // 2026-06-19: cargo-mutants --in-place replaced delay_for_attempt body
    // with `1 /* ~ changed by cargo-mutants ~ */`. This test catches that
    // specific corruption — if the function ever returns a constant, it fails.

    #[test]
    fn regression_delay_for_attempt_is_exponential_not_constant() {
        let cfg = test_config();
        // The mutation changed the body to `1`. Verify it actually computes.
        let d0 = cfg.delay_for_attempt(0); // 100
        let d1 = cfg.delay_for_attempt(1); // 200
        let d2 = cfg.delay_for_attempt(2); // 400
        let d3 = cfg.delay_for_attempt(3); // 800
        assert_eq!(d0, 100, "attempt 0 must be initial_delay_ms, not constant");
        assert_ne!(d1, d0, "delay must change between attempts, not constant");
        assert!(d2 > d1, "delay must grow exponentially");
        assert!(d3 > d2, "delay must grow exponentially");
    }

    // ── Proptest: RetryConfig serialization round-trip ──────

    proptest::proptest! {
        #[test]
        fn retry_config_to_json_round_trip(
            max_retries in 0u32..10u32,
            initial_delay_ms in 0u64..10000u64,
            max_delay_ms in 1000u64..60000u64,
            multiplier in 1.0f64..5.0f64,
        ) {
            let cfg = RetryConfig {
                max_retries,
                initial_delay_ms,
                max_delay_ms,
                multiplier,
                retryable_status: vec![429, 503],
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let parsed: RetryConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.max_retries, cfg.max_retries);
            assert_eq!(parsed.initial_delay_ms, cfg.initial_delay_ms);
            assert_eq!(parsed.max_delay_ms, cfg.max_delay_ms);
        }
    }
}
