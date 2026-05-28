//! Token-bucket per-tool rate limiting.
//!
//! Enforces a configurable number of requests per time window per tool name.
//! This is the MCP boundary approximation of hKask's energy budget model.
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
