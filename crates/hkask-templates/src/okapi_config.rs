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
pub(crate) enum OkapiConfigError {
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

    pub(crate) fn build_client(&self) -> Result<reqwest::Client, OkapiConfigError> {
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

// ---------------------------------------------------------------------------
// Okapi Model Catalog — /api/tags
// ---------------------------------------------------------------------------

/// A model entry from Okapi's `/api/tags` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiModelEntry {
    pub name: String,
    pub model: String,
    #[serde(default)]
    pub modified_at: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub details: Option<OkapiModelDetails>,
}

/// Model details from Okapi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiModelDetails {
    #[serde(default)]
    pub parent_model: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub families: Option<Vec<String>>,
    #[serde(default)]
    pub parameter_size: Option<String>,
    #[serde(default)]
    pub quantization_level: Option<String>,
}

/// Response from Okapi's `/api/tags` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OkapiTagsResponse {
    pub models: Vec<OkapiModelEntry>,
}

/// List available models from Okapi via the `/api/tags` endpoint.
///
/// Returns model entries with name, size, family, and parameter info.
/// If Okapi is unreachable, returns an empty list (graceful degradation).
pub async fn list_okapi_models(config: &OkapiConfig) -> Vec<OkapiModelEntry> {
    let client = match config.build_client() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut request = client.get(format!("{}/api/tags", config.base_url));
    if let Some(ref auth) = config.api_key {
        request = request.bearer_auth(auth);
    }

    match request.send().await {
        Ok(resp) => match resp.json::<OkapiTagsResponse>().await {
            Ok(tags) => tags.models,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// Fuzzy-search available models from Okapi.
///
/// Filters model names case-insensitively by the query string.
pub async fn search_okapi_models(config: &OkapiConfig, query: &str) -> Vec<OkapiModelEntry> {
    let models = list_okapi_models(config).await;
    if query.is_empty() {
        return models;
    }
    let lower = query.to_lowercase();
    models
        .into_iter()
        .filter(|m| m.name.to_lowercase().contains(&lower))
        .collect()
}
