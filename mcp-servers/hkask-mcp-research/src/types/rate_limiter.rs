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

    // REQ: CNS-WEB-RATELIMIT — RateLimiter allows requests within limit
    #[test]
    fn rate_limiter_allows_within_limit() {
        let rl = RateLimiter::new(5, 3600);
        for _ in 0..5 {
            assert!(rl.check("web_search").is_ok());
        }
    }

    // REQ: CNS-WEB-RATELIMIT — RateLimiter denies requests exceeding limit
    #[test]
    fn rate_limiter_denies_over_limit() {
        let rl = RateLimiter::new(2, 3600);
        assert!(rl.check("web_search").is_ok());
        assert!(rl.check("web_search").is_ok());
        assert!(rl.check("web_search").is_err());
    }

    // REQ: CNS-WEB-RATELIMIT — RateLimiter enforces per-tool counts independently
    #[test]
    fn rate_limiter_per_tool_independence() {
        let rl = RateLimiter::new(2, 3600);
        assert!(rl.check("web_search").is_ok());
        assert!(rl.check("web_search").is_ok());
        // web_search is now rate limited...
        assert!(rl.check("web_search").is_err());
        // ...but web_extract is independent
        assert!(rl.check("web_extract").is_ok());
    }

    // REQ: CNS-WEB-RATELIMIT — RateLimiter error message includes tool name and limit
    #[test]
    fn rate_limiter_error_message() {
        let rl = RateLimiter::new(2, 3600);
        for _ in 0..3 {
            let _ = rl.check("my_tool");
        }
        let err = rl.check("my_tool").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("my_tool"),
            "error should mention tool name: {msg}"
        );
        assert!(msg.contains("2"), "error should mention limit: {msg}");
    }
}
