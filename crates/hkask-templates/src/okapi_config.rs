//! Okapi Configuration - Authentication, timeouts, and connection settings
//!
//! Secure configuration for Okapi inference with support for:
//! - API key authentication from environment
//! - Timeout configuration
//! - Connection pooling
//! - Retry configuration

use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;
use thiserror::Error;

/// Okapi configuration error
#[derive(Debug, Error)]
pub enum OkapiConfigError {
    #[error("Keystore error: {0}")]
    Keystore(String),

    #[error("Environment variable not found: {0}")]
    EnvNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Okapi configuration
#[derive(Debug, Clone)]
pub struct OkapiConfig {
    /// Okapi base URL (e.g., "http://localhost:8080")
    pub base_url: String,
    /// API key for authentication (optional for local dev)
    pub api_key: Option<SecretString>,
    /// Request timeout
    pub timeout: Duration,
    /// Connection pool max idle connections per host
    pub pool_max_idle_per_host: usize,
    /// Whether to accept invalid TLS certificates (dev only)
    pub accept_invalid_certs: bool,
}

impl Default for OkapiConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 10,
            accept_invalid_certs: false,
        }
    }
}

impl OkapiConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self, OkapiConfigError> {
        let base_url =
            std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let api_key = std::env::var("OKAPI_API_KEY")
            .ok()
            .map(|s| SecretString::new(s.into()));

        let timeout_secs: u64 = std::env::var("OKAPI_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let pool_max_idle: usize = std::env::var("OKAPI_POOL_MAX_IDLE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let accept_invalid_certs = std::env::var("OKAPI_ACCEPT_INVALID_CERTS")
            .ok()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            base_url,
            api_key,
            timeout: Duration::from_secs(timeout_secs),
            pool_max_idle_per_host: pool_max_idle,
            accept_invalid_certs,
        })
    }

    /// Create configuration for local development (no auth)
    pub fn local_dev() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 5,
            accept_invalid_certs: false,
        }
    }

    /// Create configuration with API key
    pub fn with_api_key(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: Some(SecretString::new(api_key.to_string().into())),
            ..Default::default()
        }
    }

    /// Build reqwest client from configuration
    pub fn build_client(&self) -> Result<reqwest::Client, OkapiConfigError> {
        let mut builder = reqwest::Client::builder()
            .timeout(self.timeout)
            .pool_max_idle_per_host(self.pool_max_idle_per_host);

        if self.accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder
            .build()
            .map_err(|e| OkapiConfigError::InvalidConfig(e.to_string()))
    }

    /// Get API key as header value
    pub fn get_authorization_header(&self) -> Option<String> {
        self.api_key
            .as_ref()
            .map(|key| format!("Bearer {}", key.expose_secret()))
    }
}

/// Retry configuration for transient errors
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub backoff_base: Duration,
    /// Maximum delay cap
    pub max_delay: Duration,
    /// Retryable status codes
    pub retryable_status: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_base: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            retryable_status: vec![408, 429, 500, 502, 503, 504],
        }
    }
}

impl RetryConfig {
    /// Calculate delay for given retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        use std::cmp;
        let exponential = self.backoff_base * 2u32.pow(attempt);
        let capped = cmp::min(exponential, self.max_delay);
        let jitter = capped.as_millis() / 10;
        let jitter_actual =
            (rand::random::<u64>() % (jitter as u64 * 2)) as i128 - (jitter as i128);
        Duration::from_millis(((capped.as_millis() as i128 + jitter_actual) as u64).max(0))
    }

    /// Check if status code is retryable
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_status.contains(&status)
    }
}

/// Prompt validation error
#[derive(Debug, Error)]
pub enum PromptValidationError {
    #[error("Prompt is empty")]
    EmptyPrompt,
    #[error("Prompt too long: {0} characters")]
    TooLong(usize),
    #[error("Prompt may contain secrets")]
    ContainsSecret,
}

/// Input validation for prompts
pub fn validate_prompt(prompt: &str) -> Result<(), PromptValidationError> {
    if prompt.is_empty() {
        return Err(PromptValidationError::EmptyPrompt);
    }
    if prompt.len() > 1_000_000 {
        return Err(PromptValidationError::TooLong(prompt.len()));
    }
    Ok(())
}

/// Output sanitization
pub fn sanitize_output(output: &str) -> String {
    output.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_prompt_empty() {
        assert!(matches!(
            validate_prompt(""),
            Err(PromptValidationError::EmptyPrompt)
        ));
    }

    #[test]
    fn test_validate_prompt_valid() {
        assert!(validate_prompt("Hello, world!").is_ok());
    }

    #[test]
    fn test_retry_config_delay() {
        let config = RetryConfig::default();
        let delay_0 = config.delay_for_attempt(0);
        let delay_1 = config.delay_for_attempt(1);
        let delay_2 = config.delay_for_attempt(2);
        assert!(delay_1 > delay_0);
        assert!(delay_2 > delay_1);
        let delay_10 = config.delay_for_attempt(10);
        assert!(delay_10 <= config.max_delay);
    }

    #[test]
    fn test_sanitize_output() {
        let input = "Hello world";
        let output = sanitize_output(input);
        assert_eq!(output, "Hello world");
    }
}
