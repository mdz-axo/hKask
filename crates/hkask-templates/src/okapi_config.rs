//! Okapi Configuration
//!
//! Configuration for Okapi LLM inference with authentication, timeouts, and connection pooling.
//!
//! # Environment Variables
//!
//! - `OKAPI_BASE_URL` - Okapi API base URL (default: http://127.0.0.1:11435)
//! - `OKAPI_API_KEY` - API key for authentication (optional)
//! - `OKAPI_TIMEOUT_SECS` - Request timeout in seconds (default: 30)
//! - `OKAPI_POOL_MAX_IDLE` - Max idle connections per host (default: 10)
//!
//! # Example
//!
//! ```rust
//! use hkask_templates::OkapiConfig;
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
            base_url: "http://127.0.0.1:11435".to_string(),
            api_key: None,
            timeout_secs: 30,
            pool_max_idle: 10,
        }
    }
}

impl OkapiConfig {
    pub fn local_dev() -> Self {
        Self {
            base_url: "http://127.0.0.1:11435".to_string(),
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
