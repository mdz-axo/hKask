//! Token-bucket per-tool rate limiting.
//!
//! Enforces a configurable number of requests per time window per tool name.
//! This is an external API boundary rate limiter — it protects the MCP server
//! from external client DoS, distinct from internal energy budget tracking.
//! On rate limit, returns McpToolError with RateLimited kind.

use std::collections::HashMap;
use std::sync::Mutex;

use hkask_mcp::server::McpToolError;
use hkask_types::McpErrorKind;

pub struct RateLimiter {
    windows: Mutex<HashMap<String, RateWindow>>,
    max_requests: u32,
    window_secs: u64,
}

struct RateWindow {
    count: u32,
    expires_at: std::time::Instant,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            max_requests,
            window_secs,
        }
    }

    /// Check whether a request for the given tool is allowed.
    /// Returns Ok(()) if allowed, or an McpToolError with RateLimited kind if exceeded.
    pub fn check(&self, tool_name: &str) -> Result<(), McpToolError> {
        let mut windows = self.windows.lock().expect("rate limiter lock poisoned");
        let now = std::time::Instant::now();
        let entry = windows.entry(tool_name.to_string()).or_insert(RateWindow {
            count: 0,
            expires_at: now + std::time::Duration::from_secs(self.window_secs),
        });
        if now >= entry.expires_at {
            entry.count = 0;
            entry.expires_at = now + std::time::Duration::from_secs(self.window_secs);
        }
        entry.count += 1;
        if entry.count > self.max_requests {
            Err(McpToolError::new(
                McpErrorKind::RateLimited,
                format!(
                    "Rate limit exceeded for {tool_name}: {} requests per {}s",
                    self.max_requests, self.window_secs
                ),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // P8 invariant: requests within limit are allowed
    #[test]
    fn rate_limiter_allows_requests_within_limit() {
        let limiter = RateLimiter::new(5, 60);
        for i in 0..5 {
            let result = limiter.check("test_tool");
            assert!(result.is_ok(), "request {i} within limit should be allowed");
        }
    }

    // P8 invariant: requests over limit are rejected with RateLimited kind
    #[test]
    fn rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, 60);
        assert!(limiter.check("test_tool").is_ok());
        assert!(limiter.check("test_tool").is_ok());
        assert!(limiter.check("test_tool").is_ok());
        let result = limiter.check("test_tool");
        assert!(result.is_err(), "request over limit must be rejected");
        let err = result.unwrap_err();
        assert_eq!(
            err.kind,
            McpErrorKind::RateLimited,
            "error kind must be RateLimited"
        );
    }

    // P8 invariant: different tools are tracked independently
    #[test]
    fn rate_limiter_tracks_tools_independently() {
        let limiter = RateLimiter::new(2, 60);
        assert!(limiter.check("tool_a").is_ok());
        assert!(limiter.check("tool_a").is_ok());
        assert!(limiter.check("tool_b").is_ok());
        assert!(limiter.check("tool_b").is_ok());
        // Both tools at limit, but different tools
        assert!(
            limiter.check("tool_a").is_err(),
            "tool_a should be rate limited"
        );
        assert!(
            limiter.check("tool_b").is_err(),
            "tool_b should be rate limited"
        );
    }

    // P8 invariant: error message includes limit and window
    #[test]
    fn rate_limiter_error_includes_limit_and_window() {
        let limiter = RateLimiter::new(2, 30);
        limiter.check("my_tool").unwrap();
        limiter.check("my_tool").unwrap();
        let err = limiter.check("my_tool").unwrap_err();
        let msg = err.message;
        assert!(
            msg.contains("2"),
            "error must mention max_requests, got: {msg}"
        );
        assert!(
            msg.contains("30"),
            "error must mention window_secs, got: {msg}"
        );
        assert!(
            msg.contains("my_tool"),
            "error must mention tool name, got: {msg}"
        );
    }
}
