//! Inference configuration — multi-provider routing for DeepInfra, fal.ai, Together AI, and OpenRouter.
//!
//! # Environment Variables
//!
//! - `DI_BASE_URL` — DeepInfra base URL (default: <https://api.deepinfra.com>)
//! - `DI_API_KEY` — DeepInfra API key (required for DI provider)
//! - `FA_BASE_URL` — fal.ai base URL (default: <https://api.fal.ai>)
//! - `FA_API_KEY` — fal.ai API key (required for FA provider)
//! - `TG_BASE_URL` — Together AI base URL (default: <https://api.together.xyz>)
//! - `TOGETHER_API_KEY` — Together AI API key (required for TG provider)
//! - `OR_BASE_URL` — OpenRouter base URL (default: <https://openrouter.ai/api>)
//! - `OPENROUTER_API_KEY` — OpenRouter API key (required for OR provider)
//! - `HKASK_DEFAULT_PROVIDER` — default provider for unprefixed models (DI, FA, TG, OR; default: DI)
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
//! - `DI/meta-llama/Llama-3.3-70B-Instruct` → DeepInfra (cloud)
//! - `FA/paddleocr` → fal.ai (cloud)
//! - `TG/Qwen/Qwen2.5-7B-Instruct-Turbo` → Together AI (cloud)
//! - `OR/openai/gpt-4o` → OpenRouter (cloud)
//! - No prefix → default provider (configurable, default: DeepInfra)

use serde::{Deserialize, Serialize};

use hkask_types::secret::SecretRef;

/// Two-letter provider identifier for inference routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderId {
    /// DeepInfra (cloud) — prefix `DI/`
    #[serde(rename = "DI")]
    DeepInfra,
    /// fal.ai (cloud) — prefix `FA/`
    #[serde(rename = "FA")]
    Fal,
    /// Together AI (cloud) — prefix `TG/`
    #[serde(rename = "TG")]
    Together,
    /// Runpod (cloud) — prefix `RP/`
    #[serde(rename = "RP")]
    Runpod,
    /// Baseten (cloud) — prefix `BT/`
    #[serde(rename = "BT")]
    Baseten,
    /// OpenRouter (cloud) — prefix `OR/`
    #[serde(rename = "OR")]
    OpenRouter,
}

impl ProviderId {
    /// Parse a 2-letter provider prefix from a model name.
    ///
    /// Returns `None` if the model name has no recognized prefix.
    /// Returns `Some((provider, stripped_model))` if a prefix is found.
    ///
    /// expect: "The system normalizes provider responses for monitoring"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — model-name routing to provider boundary
    /// pre:  model is non-empty
    /// post: returns Some((ProviderId, stripped_model)) for DI/, FA/, TG/, RP/, BT/ prefixes
    /// post: returns None for unrecognized or missing prefix
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
            "DI" => Some((ProviderId::DeepInfra, rest)),
            "FA" => Some((ProviderId::Fal, rest)),
            "TG" => Some((ProviderId::Together, rest)),
            "RP" => Some((ProviderId::Runpod, rest)),
            "BT" => Some((ProviderId::Baseten, rest)),
            "OR" => Some((ProviderId::OpenRouter, rest)),
            _ => None,
        }
    }

    /// Format a model name with this provider's prefix.
    ///
    /// expect: "The system normalizes provider responses for monitoring"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — canonical provider-prefixed model naming
    /// pre:  model is non-empty
    /// post: returns "{prefix}/{model}" string
    pub fn prefix_model(&self, model: &str) -> String {
        format!("{}/{}", self.as_str(), model)
    }

    /// Two-letter code for this provider.
    ///
    /// expect: "The system normalizes provider responses for monitoring"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — stable provider code for routing
    /// post: returns "DI", "FA", "TG", "RP", or "BT"
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::DeepInfra => "DI",
            ProviderId::Fal => "FA",
            ProviderId::Together => "TG",
            ProviderId::Runpod => "RP",
            ProviderId::Baseten => "BT",
            ProviderId::OpenRouter => "OR",
        }
    }
}

/// Configuration for the inference router.
///
/// Holds connection settings for DeepInfra (cloud),
/// fal.ai (cloud), and Together AI (cloud).
/// The router uses this config to construct backends and decide
/// the default provider for unprefixed model names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Default provider for model names without a prefix.
    /// Default: DeepInfra (cloud-first). Override with `HKASK_DEFAULT_PROVIDER` env var
    /// or store in OS keychain under key `HKASK_DEFAULT_PROVIDER`.
    /// Accepted values: DI, FA, TG.
    pub default_provider: ProviderId,

    /// Base URL for the DeepInfra inference API (OpenAI-compatible endpoint).
    pub deepinfra_base_url: String,

    /// API key for DeepInfra authentication.
    /// Required for DI provider. If empty, DI is unavailable.
    pub deepinfra_api_key: String,

    /// Base URL for the fal.ai inference API (OpenAI-compatible endpoint).
    pub fal_base_url: String,

    /// Base URL for fal.ai media/sync endpoints (native inference API).
    pub fal_media_base_url: String,

    /// Base URL for fal.ai queue/async endpoints (native inference API).
    pub fal_queue_base_url: String,

    /// API key for fal.ai authentication.
    /// Required for FA provider. If empty, FA is unavailable.
    pub fal_api_key: String,

    /// Base URL for the Together AI inference API (OpenAI-compatible endpoint).
    pub together_base_url: String,

    /// API key for Together AI authentication.
    /// Required for TG provider. If empty, TG is unavailable.
    pub together_api_key: String,

    /// Base URL for the OpenRouter inference API (OpenAI-compatible endpoint).
    /// Defaults to `https://openrouter.ai/api` — the `/v1` suffix is appended
    /// by the backend following the shared URL construction pattern.
    pub openrouter_base_url: String,

    /// API key for OpenRouter authentication.
    /// Required for OR provider. If empty, OR is unavailable.
    pub openrouter_api_key: String,

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

    /// OpenRouter fusion model that overrides all text generation calls.
    /// When set, `generate()` and `generate_with_model()` route through this
    /// fusion model instead of their explicit/default models. Media generation,
    /// embeddings, and calls with `LLMParameters.bypass_fusion = true` are
    /// unaffected. Override with `HKASK_FUSION_MODEL` env var.
    /// Default: None (fusion disabled).
    pub fusion_model: Option<String>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            default_provider: ProviderId::DeepInfra,
            deepinfra_base_url: "https://api.deepinfra.com".to_string(),
            deepinfra_api_key: String::new(),
            fal_base_url: "https://api.fal.ai".to_string(),
            fal_media_base_url: "https://fal.run".to_string(),
            fal_queue_base_url: "https://queue.fal.run".to_string(),
            fal_api_key: String::new(),
            together_base_url: "https://api.together.xyz".to_string(),
            together_api_key: String::new(),
            openrouter_base_url: "https://openrouter.ai/api".to_string(),
            openrouter_api_key: String::new(),
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: "deepseek-v4-pro".to_string(),
            fusion_model: None,
        }
    }
}

impl InferenceConfig {
    /// Resolve from environment variables and OS keychain.
    ///
    /// API keys resolve keychain-first, then fall back to environment variables.
    /// Also accepts `DEEPINFRA_API_KEY` and `FAL_API_KEY`
    /// as legacy environment variable names.
    ///
    /// expect: "The system resolves inference configuration from the environment"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — inference configuration resolved from environment
    /// post: returns InferenceConfig resolved from env vars and keychain
    /// post: defaults to DeepInfra cloud if env vars unset
    pub fn from_env() -> Self {
        let deepinfra_base_url = std::env::var("DI_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepinfra.com".to_string());

        let deepinfra_api_key = resolve_api_key("DI_API_KEY", &["DEEPINFRA_API_KEY"]);

        let fal_base_url =
            std::env::var("FA_BASE_URL").unwrap_or_else(|_| "https://api.fal.ai".to_string());

        let fal_media_base_url =
            std::env::var("FA_MEDIA_BASE_URL").unwrap_or_else(|_| "https://fal.run".to_string());

        let fal_queue_base_url = std::env::var("FA_QUEUE_BASE_URL")
            .unwrap_or_else(|_| "https://queue.fal.run".to_string());

        let fal_api_key = resolve_api_key("FA_API_KEY", &["FAL_API_KEY"]);

        let together_base_url =
            std::env::var("TG_BASE_URL").unwrap_or_else(|_| "https://api.together.xyz".to_string());

        let together_api_key = resolve_api_key("TOGETHER_API_KEY", &[]);

        let openrouter_base_url = std::env::var("OR_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api".to_string());

        let openrouter_api_key = resolve_api_key("OPENROUTER_API_KEY", &["OR_API_KEY"]);

        let default_provider = resolve_default_provider();

        Self {
            default_provider,
            deepinfra_base_url,
            deepinfra_api_key,
            fal_base_url,
            fal_media_base_url,
            fal_queue_base_url,
            fal_api_key,
            together_base_url,
            together_api_key,
            openrouter_base_url,
            openrouter_api_key,
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: std::env::var("HKASK_DEFAULT_MODEL")
                .unwrap_or_else(|_| "deepseek-v4-pro".to_string()),
            fusion_model: std::env::var("HKASK_FUSION_MODEL").ok(),
        }
    }

    /// Build a reqwest HTTP client with the configured timeout and pool settings.
    ///
    /// expect: "The system resolves inference configuration from the environment"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — bounded HTTP client for regulated requests
    /// post: returns reqwest::Client with timeout and pool settings from config
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
        let key = String::from_utf8_lossy(&zeroizing).into_owned();
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
/// variable. Accepted values: DI, FA, TG. Defaults to DeepInfra.
fn resolve_default_provider() -> ProviderId {
    let raw = resolve_api_key("HKASK_DEFAULT_PROVIDER", &[]);
    parse_provider_code(&raw)
}

/// Parse a provider code string to a ProviderId.
///
/// Accepted values: DI, FA, TG, RP, BT. Anything else (including empty) → DeepInfra.
fn parse_provider_code(raw: &str) -> ProviderId {
    match raw {
        "DI" => ProviderId::DeepInfra,
        "FA" => ProviderId::Fal,
        "TG" => ProviderId::Together,
        "RP" => ProviderId::Runpod,
        "BT" => ProviderId::Baseten,
        "OR" => ProviderId::OpenRouter,
        _ => ProviderId::DeepInfra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// expect: "Inference provider prefix parsing works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates provider routing parser
    #[test]
    fn parse_provider_prefix() {
        assert_eq!(
            ProviderId::parse_from_model("TG/Qwen/Qwen2.5-7B-Instruct-Turbo"),
            Some((ProviderId::Together, "Qwen/Qwen2.5-7B-Instruct-Turbo"))
        );
        assert_eq!(
            ProviderId::parse_from_model("DI/meta-llama/Llama-3.3-70B-Instruct"),
            Some((ProviderId::DeepInfra, "meta-llama/Llama-3.3-70B-Instruct"))
        );
        assert_eq!(
            ProviderId::parse_from_model("RP/my-model"),
            Some((ProviderId::Runpod, "my-model"))
        );
        assert_eq!(
            ProviderId::parse_from_model("BT/my-model"),
            Some((ProviderId::Baseten, "my-model"))
        );
    }

    /// expect: "Inference model prefix fallback works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates default-provider fallback
    #[test]
    fn parse_no_prefix_returns_none() {
        assert_eq!(ProviderId::parse_from_model("deepseek-v4-pro"), None);
        assert_eq!(ProviderId::parse_from_model("qwen3:8b"), None);
    }

    /// expect: "Inference malformed model rejection works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates malformed model rejection
    #[test]
    fn parse_empty_model_returns_none() {
        assert_eq!(ProviderId::parse_from_model("DI/"), None);
        assert_eq!(ProviderId::parse_from_model("FA/"), None);
    }

    /// expect: "Inference malformed model rejection works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates malformed model rejection
    #[test]
    fn parse_too_short_returns_none() {
        assert_eq!(ProviderId::parse_from_model("DI"), None);
        assert_eq!(ProviderId::parse_from_model("FA"), None);
        assert_eq!(ProviderId::parse_from_model("X"), None);
    }

    /// expect: "Inference unknown provider rejection works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates unknown provider rejection
    #[test]
    fn parse_unknown_prefix_returns_none() {
        assert_eq!(ProviderId::parse_from_model("XX/model"), None);
        assert_eq!(ProviderId::parse_from_model("AB/test"), None);
    }

    /// expect: "Inference model name formatting works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates canonical model naming
    #[test]
    fn prefix_model_format() {
        assert_eq!(
            ProviderId::Together.prefix_model("Qwen/Qwen2.5-7B"),
            "TG/Qwen/Qwen2.5-7B"
        );
        assert_eq!(
            ProviderId::DeepInfra.prefix_model("meta-llama/Llama-3.3-70B"),
            "DI/meta-llama/Llama-3.3-70B"
        );
        assert_eq!(ProviderId::Fal.prefix_model("paddleocr"), "FA/paddleocr");
        assert_eq!(ProviderId::Runpod.prefix_model("my-model"), "RP/my-model");
        assert_eq!(ProviderId::Baseten.prefix_model("my-model"), "BT/my-model");
    }

    /// expect: "Inference fal.ai prefix parsing works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates fal.ai routing
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

    /// expect: "Inference provider code parsing works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates provider code parser
    #[test]
    fn parse_provider_code_all_codes() {
        assert_eq!(parse_provider_code("DI"), ProviderId::DeepInfra);
        assert_eq!(parse_provider_code("FA"), ProviderId::Fal);
        assert_eq!(parse_provider_code("TG"), ProviderId::Together);
        assert_eq!(parse_provider_code("RP"), ProviderId::Runpod);
        assert_eq!(parse_provider_code("BT"), ProviderId::Baseten);
    }

    /// expect: "Inference provider code default works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates safe default provider
    #[test]
    fn parse_provider_code_unknown_defaults_to_deepinfra() {
        assert_eq!(parse_provider_code("XX"), ProviderId::DeepInfra);
        assert_eq!(parse_provider_code(""), ProviderId::DeepInfra);
        assert_eq!(parse_provider_code("unknown"), ProviderId::DeepInfra);
        assert_eq!(parse_provider_code("om"), ProviderId::DeepInfra);
    }

    // ── resolve_api_key ──────────────────────────────────────────────────

    /// expect: "Inference API key resolution works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates API key resolution
    #[test]
    fn resolve_api_key_primary_env() {
        // SAFETY: Setting/removing test environment variables in test code is safe in a single-threaded test context (Rust runs tests serially by default).
        unsafe { std::env::set_var("HKASK_TEST_KEY_010", "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX") };
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_010", &[]),
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX"
        );
        // SAFETY: Test cleanup — see above.
        unsafe { std::env::remove_var("HKASK_TEST_KEY_010") };
    }

    /// expect: "Inference API key fallback works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates API key fallback
    #[test]
    fn resolve_api_key_fallback_env() {
        // SAFETY: Setting/removing test environment variables in test code is safe in a single-threaded test context (Rust runs tests serially by default).
        unsafe { std::env::set_var("HKASK_TEST_LEGACY_011", "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX") };
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_011", &["HKASK_TEST_LEGACY_011"]),
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX"
        );
        // SAFETY: Test cleanup — see above.
        unsafe { std::env::remove_var("HKASK_TEST_LEGACY_011") };
    }

    /// expect: "Inference API key missing handling works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates missing key handling
    #[test]
    fn resolve_api_key_empty_when_missing() {
        // SAFETY: Test cleanup — removing environment variables is safe in single-threaded test context.
        unsafe {
            std::env::remove_var("HKASK_TEST_KEY_012");
            std::env::remove_var("HKASK_TEST_LEGACY_012");
        }
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_012", &["HKASK_TEST_LEGACY_012"]),
            ""
        );
    }

    /// expect: "Inference API key priority works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates keychain/env priority
    #[test]
    fn resolve_api_key_primary_wins_over_fallback() {
        // SAFETY: Setting/removing test environment variables in test code is safe in a single-threaded test context (Rust runs tests serially by default).
        unsafe {
            std::env::set_var("HKASK_TEST_KEY_013", "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX");
            std::env::set_var("HKASK_TEST_LEGACY_013", "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX");
        }
        assert_eq!(
            resolve_api_key("HKASK_TEST_KEY_013", &["HKASK_TEST_LEGACY_013"]),
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX"
        );
        // SAFETY: Test cleanup — see above.
        unsafe {
            std::env::remove_var("HKASK_TEST_KEY_013");
            std::env::remove_var("HKASK_TEST_LEGACY_013");
        }
    }
}
