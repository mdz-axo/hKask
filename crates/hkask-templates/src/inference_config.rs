//! Inference configuration — multi-provider routing for Ollama, Fireworks, and DeepInfra.
//!
//! # Environment Variables
//!
//! - `OM_BASE_URL` — Ollama base URL (default: http://127.0.0.1:11434)
//! - `FW_BASE_URL` — Fireworks base URL (default: https://api.fireworks.ai/inference)
//! - `FW_API_KEY` — Fireworks API key (required for FW provider)
//! - `DI_BASE_URL` — DeepInfra base URL (default: https://api.deepinfra.com/v1/openai)
//! - `DI_API_KEY` — DeepInfra API key (required for DI provider)
//! - `OKAPI_BASE_URL` — Legacy; maps to `OM_BASE_URL` if `OM_BASE_URL` is unset
//!
//! # Model Naming Convention
//!
//! Models use a 2-letter provider prefix:
//! - `OM/qwen3:8b` → Ollama (local)
//! - `FW/llama-v3p1-70b-instruct` → Fireworks.ai (cloud)
//! - `DI/meta-llama/Llama-3.3-70B-Instruct` → DeepInfra (cloud)
//! - No prefix → default provider (configurable, default: Ollama)

use serde::{Deserialize, Serialize};

/// Two-letter provider identifier for inference routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderId {
    /// Ollama (local) — prefix `OM/`
    #[serde(rename = "OM")]
    Ollama,
    /// Fireworks.ai (cloud) — prefix `FW/`
    #[serde(rename = "FW")]
    Fireworks,
    /// DeepInfra (cloud) — prefix `DI/`
    #[serde(rename = "DI")]
    DeepInfra,
}

impl ProviderId {
    /// Parse a 2-letter provider prefix from a model name.
    ///
    /// Returns `None` if the model name has no recognized prefix.
    /// Returns `Some((provider, stripped_model))` if a prefix is found.
    pub fn parse_from_model(model: &str) -> Option<(Self, &str)> {
        if model.len() < 4 {
            return None;
        }
        let bytes = model.as_bytes();
        if bytes.get(2) != Some(&b'/') {
            return None;
        }
        let prefix = &model[..2];
        let rest = &model[3..];
        if rest.is_empty() {
            return None;
        }
        match prefix {
            "OM" => Some((ProviderId::Ollama, rest)),
            "FW" => Some((ProviderId::Fireworks, rest)),
            "DI" => Some((ProviderId::DeepInfra, rest)),
            _ => None,
        }
    }

    /// Format a model name with this provider's prefix.
    pub fn prefix_model(&self, model: &str) -> String {
        let prefix = self.as_str();
        format!("{}/{}", prefix, model)
    }

    /// Two-letter code for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Ollama => "OM",
            ProviderId::Fireworks => "FW",
            ProviderId::DeepInfra => "DI",
        }
    }
}

/// Configuration for the inference router.
///
/// Holds connection settings for Ollama (local), Fireworks (cloud),
/// and DeepInfra (cloud). The router uses this config to construct
/// backends and decide the default provider for unprefixed model names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Default provider for model names without a prefix.
    /// Default: Ollama (local-first).
    pub default_provider: ProviderId,

    /// Base URL for the Ollama inference server.
    pub ollama_base_url: String,

    /// Base URL for the Fireworks inference API.
    pub fireworks_base_url: String,

    /// API key for Fireworks authentication.
    /// Required for FW provider. If empty, FW is unavailable.
    pub fireworks_api_key: String,

    /// Base URL for the DeepInfra inference API (OpenAI-compatible endpoint).
    pub deepinfra_base_url: String,

    /// API key for DeepInfra authentication.
    /// Required for DI provider. If empty, DI is unavailable.
    pub deepinfra_api_key: String,

    /// Request timeout in seconds for inference calls.
    /// Default: 120 (accommodates model cold-start).
    pub timeout_secs: u64,

    /// Max idle connections per host for the HTTP client pool.
    /// Default: 5.
    pub pool_max_idle: usize,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            default_provider: ProviderId::Ollama,
            ollama_base_url: "http://127.0.0.1:11434".to_string(),
            fireworks_base_url: "https://api.fireworks.ai/inference".to_string(),
            fireworks_api_key: String::new(),
            deepinfra_base_url: "https://api.deepinfra.com/v1/openai".to_string(),
            deepinfra_api_key: String::new(),
            timeout_secs: 120,
            pool_max_idle: 5,
        }
    }
}

impl InferenceConfig {
    /// Resolve from environment variables.
    ///
    /// Reads `OM_BASE_URL`, `FW_BASE_URL`, `FW_API_KEY`, `DI_BASE_URL`, `DI_API_KEY`.
    /// Falls back to `OKAPI_BASE_URL` for `OM_BASE_URL` if unset (legacy migration).
    pub fn from_env() -> Self {
        let ollama_base_url = std::env::var("OM_BASE_URL")
            .or_else(|_| std::env::var("OKAPI_BASE_URL"))
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        let fireworks_base_url = std::env::var("FW_BASE_URL")
            .unwrap_or_else(|_| "https://api.fireworks.ai/inference".to_string());

        let fireworks_api_key = std::env::var("FW_API_KEY").unwrap_or_default();

        let deepinfra_base_url = std::env::var("DI_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepinfra.com/v1/openai".to_string());

        let deepinfra_api_key = std::env::var("DI_API_KEY").unwrap_or_default();

        Self {
            default_provider: ProviderId::Ollama,
            ollama_base_url,
            fireworks_base_url,
            fireworks_api_key,
            deepinfra_base_url,
            deepinfra_api_key,
            timeout_secs: 120,
            pool_max_idle: 5,
        }
    }

    /// Build a reqwest HTTP client with the configured timeout and pool settings.
    pub fn build_client(&self) -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .pool_max_idle_per_host(self.pool_max_idle)
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ: inf-cfg-001 — ProviderId::parse_from_model parses all three prefixes
    #[test]
    fn parse_provider_prefix() {
        assert_eq!(
            ProviderId::parse_from_model("OM/qwen3:8b"),
            Some((ProviderId::Ollama, "qwen3:8b"))
        );
        assert_eq!(
            ProviderId::parse_from_model("FW/llama-v3p1-70b-instruct"),
            Some((ProviderId::Fireworks, "llama-v3p1-70b-instruct"))
        );
        assert_eq!(
            ProviderId::parse_from_model("DI/meta-llama/Llama-3.3-70B-Instruct"),
            Some((ProviderId::DeepInfra, "meta-llama/Llama-3.3-70B-Instruct"))
        );
    }

    /// REQ: inf-cfg-002 — unprefixed model names return None
    #[test]
    fn parse_no_prefix_returns_none() {
        assert_eq!(ProviderId::parse_from_model("deepseek-v4-pro"), None);
        assert_eq!(ProviderId::parse_from_model("qwen3:8b"), None);
    }

    /// REQ: inf-cfg-003 — empty model after prefix returns None
    #[test]
    fn parse_empty_model_returns_none() {
        assert_eq!(ProviderId::parse_from_model("OM/"), None);
        assert_eq!(ProviderId::parse_from_model("FW/"), None);
        assert_eq!(ProviderId::parse_from_model("DI/"), None);
    }

    /// REQ: inf-cfg-004 — too-short strings return None
    #[test]
    fn parse_too_short_returns_none() {
        assert_eq!(ProviderId::parse_from_model("OM"), None);
        assert_eq!(ProviderId::parse_from_model("DI"), None);
        assert_eq!(ProviderId::parse_from_model("X"), None);
    }

    /// REQ: inf-cfg-005 — unknown prefix returns None
    #[test]
    fn parse_unknown_prefix_returns_none() {
        assert_eq!(ProviderId::parse_from_model("XX/model"), None);
        assert_eq!(ProviderId::parse_from_model("AB/test"), None);
    }

    /// REQ: inf-cfg-006 — prefix_model formats correctly for all providers
    #[test]
    fn prefix_model_format() {
        assert_eq!(ProviderId::Ollama.prefix_model("qwen3:8b"), "OM/qwen3:8b");
        assert_eq!(
            ProviderId::Fireworks.prefix_model("llama-v3p1"),
            "FW/llama-v3p1"
        );
        assert_eq!(
            ProviderId::DeepInfra.prefix_model("meta-llama/Llama-3.3-70B"),
            "DI/meta-llama/Llama-3.3-70B"
        );
    }
}
