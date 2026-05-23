//! Integration Tests for Inference Pipeline
//!
//! Tests for rate limiting and basic inference operations.

use hkask_cns::{RateLimitConfig, RateLimiter};
use hkask_types::WebID;
use std::time::Duration;

/// Test rate limiting blocks excess requests
#[test]
fn test_rate_limiting_blocks_excess_requests() {
    // Arrange
    let rate_limiter = RateLimiter::new(RateLimitConfig {
        max_tokens: 5,
        refill_interval: Duration::from_secs(60),
    });
    let webid = WebID::new();

    // Act: Make 6 requests (5 should pass, 6th should fail)
    let mut results = Vec::new();
    for _ in 0..6 {
        results.push(rate_limiter.check(&webid));
    }

    // Assert
    assert!(
        results[..5].iter().all(|&r| r),
        "First 5 requests should pass"
    );
    assert!(!results[5], "6th request should be rate limited");
}

/// Test rate limiter refills over time
#[test]
fn test_rate_limiter_refills() {
    // Arrange
    let rate_limiter = RateLimiter::new(RateLimitConfig {
        max_tokens: 2,
        refill_interval: Duration::from_millis(100),
    });
    let webid = WebID::new();

    // Act: Use all tokens
    assert!(rate_limiter.check(&webid));
    assert!(rate_limiter.check(&webid));
    assert!(!rate_limiter.check(&webid)); // Should be blocked

    // Wait for refill
    std::thread::sleep(Duration::from_millis(150));

    // Assert: Should have tokens again
    assert!(rate_limiter.check(&webid), "Should have refilled");
}

/// Test rate limiter with different WebIDs
#[test]
fn test_rate_limiter_per_webid() {
    // Arrange
    let rate_limiter = RateLimiter::new(RateLimitConfig {
        max_tokens: 2,
        refill_interval: Duration::from_secs(60),
    });
    let webid1 = WebID::new();
    let webid2 = WebID::new();

    // Act: webid1 uses all tokens
    assert!(rate_limiter.check(&webid1));
    assert!(rate_limiter.check(&webid1));
    assert!(!rate_limiter.check(&webid1));

    // Assert: webid2 should have its own tokens
    assert!(rate_limiter.check(&webid2));
    assert!(rate_limiter.check(&webid2));
}
