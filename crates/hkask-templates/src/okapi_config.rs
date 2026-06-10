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

/// Prompt validation (internal)
pub(crate) fn validate_prompt(prompt: &str) -> Result<(), String> {
    if prompt.is_empty() {
        return Err("Prompt is empty".to_string());
    }
    if prompt.len() > 1_000_000 {
        return Err("Prompt too long".to_string());
    }
    Ok(())
}

// Okapi Model Catalog — /api/tags

/// A model entry from Okapi's `/api/tags` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiModelEntry {
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub details: Option<OkapiModelDetails>,
}

/// Model details from Okapi — populated from Ollama's `/api/tags` response.
/// Fields are all optional with `#[serde(default)]` so we gracefully handle
/// models from any inference backend that may not provide every field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiModelDetails {
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub parameter_size: Option<String>,
    #[serde(default)]
    pub quantization_level: Option<String>,
    /// Context length in tokens. Provided by Ollama as details.context_length.
    #[serde(default)]
    pub context_length: Option<u32>,
    /// Model capabilities. Example: ["completion", "chat", "json", "tools"].
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
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

/// Per-model detail from Ollama's `/api/show` endpoint (proxied through Okapi).
///
/// Returns detailed model info including `model_info` (architecture, context_length,
/// embedding dimensions) and `capabilities` (completion, chat, tools, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiModelShow {
    #[serde(default)]
    pub modelfile: Option<String>,
    #[serde(default)]
    pub parameters: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    /// Raw model_info map — keys are architecture-prefixed (e.g. "qwen3.context_length").
    #[serde(default)]
    pub model_info: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Model capabilities — e.g. ["completion", "chat", "json", "tools"].
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
}

impl OkapiModelShow {
    /// Extract context_length from model_info, searching for any key ending in `.context_length`.
    pub fn context_length(&self) -> Option<u32> {
        self.model_info.as_ref()?.iter().find_map(|(k, v)| {
            if k.ends_with(".context_length") {
                v.as_u64().map(|n| n as u32)
            } else {
                None
            }
        })
    }

    /// Check whether the model supports extended thinking / reasoning.
    /// Looks for relevant keys in model_info (e.g. "qwen3.reasoning.enable")
    /// or the presence of "reasoning" in capabilities.
    pub fn supports_thinking(&self) -> bool {
        if let Some(ref caps) = self.capabilities {
            if caps.iter().any(|c| c == "reasoning" || c == "thinking") {
                return true;
            }
        }
        if let Some(ref info) = self.model_info {
            if info.iter().any(|(k, v)| {
                (k.contains("reasoning") || k.contains("thinking")) && v.as_bool().unwrap_or(false)
            }) {
                return true;
            }
        }
        false
    }
}

/// Fetch per-model detail from Ollama via Okapi's `/api/show` endpoint.
///
/// Returns `None` if the endpoint is unreachable, the model is not found,
/// or the response cannot be parsed (graceful degradation).
pub async fn fetch_model_show(config: &OkapiConfig, model: &str) -> Option<OkapiModelShow> {
    let client = config.build_client().ok()?;

    let mut request = client.get(format!("{}/api/show", config.base_url));
    request = request.query(&[("name", model)]);
    if let Some(ref auth) = config.api_key {
        request = request.bearer_auth(auth);
    }

    match request.send().await {
        Ok(resp) => resp.json::<OkapiModelShow>().await.ok(),
        Err(_) => None,
    }
}
