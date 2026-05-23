//! Okapi Configuration
//!
//! Configuration for Okapi LLM inference with authentication, timeouts, and connection pooling.
//!
//! # Environment Variables
//!
//! - `OKAPI_BASE_URL` - Okapi API base URL (default: http://localhost:8080)
//! - `OKAPI_API_KEY` - API key for authentication (optional)
//! - `OKAPI_TIMEOUT_SECS` - Request timeout in seconds (default: 30)
//! - `OKAPI_POOL_MAX_IDLE` - Max idle connections per host (default: 10)
//!
//! # Example
//!
//! ```rust
//! use hkask_templates::{OkapiConfig, RetryConfig};
//!
//! // Local development (no auth)
//! let config = OkapiConfig::local_dev();
//!
//! // Production with API key
//! let config = OkapiConfig {
//!     base_url: "https://okapi.example.com".to_string(),
//!     api_key: Some("your-api-key".to_string()),
//!     timeout_secs: 60,
//!     pool_max_idle: 20,
//! };
//!
//! // Retry configuration
//! let retry_config = RetryConfig::default();
//! assert_eq!(retry_config.max_retries, 3);
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OkapiConfigError {
    #[error("Config error: {0}")]
    Config(String),
}

/// Okapi configuration
///
/// Contains connection settings for Okapi API access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    pub pool_max_idle: usize,
}

impl Default for OkapiConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout_secs: 30,
            pool_max_idle: 10,
        }
    }
}

impl OkapiConfig {
    pub fn local_dev() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout_secs: 30,
            pool_max_idle: 5,
        }
    }

    pub fn build_client(&self) -> Result<reqwest::Client, OkapiConfigError> {
        reqwest::Client::builder()
            .build()
            .map_err(|e| OkapiConfigError::Config(e.to_string()))
    }

    pub fn get_authorization_header(&self) -> Option<String> {
        self.api_key.as_ref().map(|key| format!("Bearer {}", key))
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub backoff_base_ms: u64,
    pub max_delay_ms: u64,
    pub retryable_status: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_base_ms: 500,
            max_delay_ms: 30000,
            retryable_status: vec![408, 429, 500, 502, 503, 504],
        }
    }
}

impl RetryConfig {
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_status.contains(&status)
    }

    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        use std::cmp;
        let exponential = self.backoff_base_ms * 2u64.pow(attempt);
        let capped = cmp::min(exponential, self.max_delay_ms);
        std::time::Duration::from_millis(capped)
    }
}

/// Prompt validation
pub fn validate_prompt(prompt: &str) -> Result<(), String> {
    if prompt.is_empty() {
        return Err("Prompt is empty".to_string());
    }
    if prompt.len() > 1_000_000 {
        return Err("Prompt too long".to_string());
    }
    Ok(())
}
