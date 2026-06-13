//! Inference configuration — multi-provider routing for Ollama, Fireworks, DeepInfra, and fal.ai.
//!
//! # Environment Variables
//!
//! - `OM_BASE_URL` — Ollama base URL (default: http://127.0.0.1:11434)
//! - `FW_BASE_URL` — Fireworks base URL (default: https://api.fireworks.ai/inference)
//! - `FW_API_KEY` — Fireworks API key (required for FW provider)
//! - `DI_BASE_URL` — DeepInfra base URL (default: https://api.deepinfra.com)
//! - `DI_API_KEY` — DeepInfra API key (required for DI provider)
//! - `FA_BASE_URL` — fal.ai base URL (default: https://api.fal.ai)
//! - `FA_API_KEY` — fal.ai API key (required for FA provider)
//! - `HKASK_DEFAULT_PROVIDER` — default provider for unprefixed models (OM, FW, DI, FA; default: OM)
//!
//! # API Key Resolution
//!
//! Provider API keys resolve through a 2-tier chain:
//! 1. OS keychain (encrypted at rest) — preferred for cloud deployments
//! 2. Environment variable (backward compat, SSH session convenience)
//!
//! Use `kask keystore load --path providers.env --shred` to load keys into the
//! keychain and securely delete the plaintext file.
//!
//! # Model Naming Convention
//!
//! Models use a 2-letter provider prefix:
//! - `OM/qwen3:8b` → Ollama (local)
//! - `FW/llama-v3p1-70b-instruct` → Fireworks.ai (cloud)
//! - `DI/meta-llama/Llama-3.3-70B-Instruct` → DeepInfra (cloud)
//! - `FA/paddleocr` → fal.ai (cloud)
//! - No prefix → default provider (configurable, default: Ollama)

use serde::{Deserialize, Serialize};

use hkask_types::SecretRef;

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
    /// fal.ai (cloud) — prefix `FA/`
    #[serde(rename = "FA")]
    Fal,
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
            "FA" => Some((ProviderId::Fal, rest)),
            _ => None,
        }
    }

    /// Format a model name with this provider's prefix.
    pub fn prefix_model(&self, model: &str) -> String {
        format!("{}/{}", self.as_str(), model)
    }

    /// Two-letter code for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Ollama => "OM",
            ProviderId::Fireworks => "FW",
            ProviderId::DeepInfra => "DI",
            ProviderId::Fal => "FA",
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
    /// Default: Ollama (local-first). Override with `HKASK_DEFAULT_PROVIDER` env var
    /// or store in OS keychain under key `HKASK_DEFAULT_PROVIDER`.
    /// Accepted values: OM, FW, DI, FA.
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

    /// Base URL for the fal.ai inference API (OpenAI-compatible endpoint).
    pub fal_base_url: String,

    /// API key for fal.ai authentication.
    /// Required for FA provider. If empty, FA is unavailable.
    pub fal_api_key: String,

    /// Request timeout in seconds for inference calls.
    /// Default: 120 (accommodates model cold-start).
    pub timeout_secs: u64,

    /// Max idle connections per host for the HTTP client pool.
    /// Default: 5.
    pub pool_max_idle: usize,

    /// Default model name used when no model is specified at inference time.
    /// Supports provider prefixes (OM/, FW/, DI/) or unprefixed names.
    /// Default: "deepseek-v4-pro". Override with `HKASK_DEFAULT_MODEL` env var.
    pub default_model: String,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            default_provider: ProviderId::Ollama,
            ollama_base_url: "http://127.0.0.1:11434".to_string(),
            fireworks_base_url: "https://api.fireworks.ai/inference".to_string(),
            fireworks_api_key: String::new(),
            deepinfra_base_url: "https://api.deepinfra.com".to_string(),
            deepinfra_api_key: String::new(),
            fal_base_url: "https://api.fal.ai".to_string(),
            fal_api_key: String::new(),
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: "deepseek-v4-pro".to_string(),
        }
    }
}

impl InferenceConfig {
    /// Resolve from environment variables and OS keychain.
    ///
    /// API keys resolve keychain-first, then fall back to environment variables.
    /// Also accepts `FIREWORKS_API_KEY`, `DEEPINFRA_API_KEY`, and `FAL_API_KEY`
    /// as legacy environment variable names.
    pub fn from_env() -> Self {
        let ollama_base_url =
            std::env::var("OM_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        let fireworks_base_url = std::env::var("FW_BASE_URL")
            .unwrap_or_else(|_| "https://api.fireworks.ai/inference".to_string());

        let fireworks_api_key = resolve_api_key("FW_API_KEY", &["FIREWORKS_API_KEY"]);

        let deepinfra_base_url = std::env::var("DI_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepinfra.com".to_string());

        let deepinfra_api_key = resolve_api_key("DI_API_KEY", &["DEEPINFRA_API_KEY"]);

        let fal_base_url =
            std::env::var("FA_BASE_URL").unwrap_or_else(|_| "https://api.fal.ai".to_string());

        let fal_api_key = resolve_api_key("FA_API_KEY", &["FAL_API_KEY"]);

        let default_provider = resolve_default_provider();

        Self {
            default_provider,
            ollama_base_url,
            fireworks_base_url,
            fireworks_api_key,
            deepinfra_base_url,
            deepinfra_api_key,
            fal_base_url,
            fal_api_key,
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: std::env::var("HKASK_DEFAULT_MODEL")
                .unwrap_or_else(|_| "deepseek-v4-pro".to_string()),
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

// ── Private resolution helpers ──────────────────────────────────────────────

/// Resolve a provider API key through the 2-tier chain: keychain → env var.
///
/// Tier 1: OS keychain (encrypted at rest, preferred for cloud deployments).
/// Tier 2: Environment variable (primary name, then fallback names).
/// Returns empty string if no key is found — the backend will be unavailable.
fn resolve_api_key(primary_env: &str, fallback_envs: &[&str]) -> String {
    // Tier 1: OS keychain
    if let Ok(zeroizing) = hkask_keystore::resolve(&SecretRef::Keychain(primary_env.to_string())) {
        let key = String::from_utf8_lossy(&*zeroizing).into_owned();
        if !key.is_empty() {
            return key;
        }
    }

    // Tier 2: Environment variable (primary name)
    if let Ok(key) = std::env::var(primary_env)
        && !key.is_empty()
    {
        return key;
    }

    // Tier 2 (fallback): Legacy environment variable names
    for fallback in fallback_envs {
        if let Ok(key) = std::env::var(fallback)
            && !key.is_empty()
        {
            return key;
        }
    }

    String::new()
}

/// Resolve the default provider from env var or keychain.
///
/// Reads `HKASK_DEFAULT_PROVIDER` from OS keychain first, then environment
/// variable. Accepted values: OM, FW, DI, FA. Defaults to Ollama.
fn resolve_default_provider() -> ProviderId {
    let raw = resolve_api_key("HKASK_DEFAULT_PROVIDER", &[]);
    parse_provider_code(&raw)
}

/// Parse a provider code string to a ProviderId.
///
/// Accepted values: OM, FW, DI, FA. Anything else (including empty) → Ollama.
fn parse_provider_code(raw: &str) -> ProviderId {
    match raw {
        "OM" => ProviderId::Ollama,
        "FW" => ProviderId::Fireworks,
        "DI" => ProviderId::DeepInfra,
        "FA" => ProviderId::Fal,
        _ => ProviderId::Ollama,
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
        assert_eq!(ProviderId::Fal.prefix_model("paddleocr"), "FA/paddleocr");
    }

    /// REQ: inf-cfg-007 — FA/ prefix parses correctly
    #[test]
    fn parse_fal_prefix() {
        assert_eq!(
            ProviderId::parse_from_model("FA/paddleocr"),
            Some((ProviderId::Fal, "paddleocr"))
        );
        assert_eq!(
            ProviderId::parse_from_model("FA/nemotron-parse"),
            Some((ProviderId::Fal, "nemotron-parse"))
        );
    }

    // ── parse_provider_code ────────────────────────────────────────────

    /// REQ: inf-cfg-008 — parse_provider_code parses all four provider codes
    #[test]
    fn parse_provider_code_all_codes() {
        assert_eq!(parse_provider_code("OM"), ProviderId::Ollama);
        assert_eq!(parse_provider_code("FW"), ProviderId::Fireworks);
        assert_eq!(parse_provider_code("DI"), ProviderId::DeepInfra);
        assert_eq!(parse_provider_code("FA"), ProviderId::Fal);
    }

    /// REQ: inf-cfg-009 — unknown or empty provider code defaults to Ollama
    #[test]
    fn parse_provider_code_unknown_defaults_to_ollama() {
        assert_eq!(parse_provider_code("XX"), ProviderId::Ollama);
        assert_eq!(parse_provider_code(""), ProviderId::Ollama);
        assert_eq!(parse_provider_code("ollama"), ProviderId::Ollama);
        assert_eq!(parse_provider_code("om"), ProviderId::Ollama); // case-sensitive
    }

    // ── resolve_api_key ──────────────────────────────────────────────────

    /// REQ: inf-cfg-010 — resolve_api_key reads from primary env var
    #[test]
    fn resolve_api_key_primary_env() {
        unsafe { std::env::set_var("HKASK_TEST_KEY_010", "sk-test-primary") };
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_010", &[]),
            "sk-test-primary"
        );
        unsafe { std::env::remove_var("HKASK_TEST_KEY_010") };
    }

    /// REQ: inf-cfg-011 — resolve_api_key falls back to legacy env var names
    #[test]
    fn resolve_api_key_fallback_env() {
        unsafe { std::env::set_var("HKASK_TEST_LEGACY_011", "sk-test-legacy") };
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_011", &["HKASK_TEST_LEGACY_011"]),
            "sk-test-legacy"
        );
        unsafe { std::env::remove_var("HKASK_TEST_LEGACY_011") };
    }

    /// REQ: inf-cfg-012 — resolve_api_key returns empty when no key found
    #[test]
    fn resolve_api_key_empty_when_missing() {
        unsafe { std::env::remove_var("HKASK_TEST_KEY_012") };
        unsafe { std::env::remove_var("HKASK_TEST_LEGACY_012") };
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_012", &["HKASK_TEST_LEGACY_012"]),
            ""
        );
    }

    /// REQ: inf-cfg-013 — resolve_api_key prefers primary over fallback
    #[test]
    fn resolve_api_key_primary_wins_over_fallback() {
        unsafe { std::env::set_var("HKASK_TEST_KEY_013", "sk-primary") };
        unsafe { std::env::set_var("HKASK_TEST_LEGACY_013", "sk-legacy") };
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_013", &["HKASK_TEST_LEGACY_013"]),
            "sk-primary"
        );
        unsafe { std::env::remove_var("HKASK_TEST_KEY_013") };
        unsafe { std::env::remove_var("HKASK_TEST_LEGACY_013") };
    }
}
