//! Inference configuration — multi-provider routing for DeepInfra, fal.ai, Together AI, OpenRouter, and KiloCode.
//!
//! # Environment Variables
//!
//! - `DI_BASE_URL` — DeepInfra base URL (default: <https://api.deepinfra.com>)
//! - `DI_API_KEY` — DeepInfra API key (required for DI provider)
//! - `FA_BASE_URL` — fal.ai base URL (default: <https://api.fal.ai>)
//! - `FA_API_KEY` — fal.ai API key (required for FA provider)
//! - `TG_BASE_URL` — Together AI base URL (default: <https://api.together.xyz>)
//! - `TG_API_KEY` — Together AI API key (required for TG provider)
//! - `OR_BASE_URL` — OpenRouter base URL (default: <https://openrouter.ai/api>)
//! - `OR_API_KEY` — OpenRouter API key (required for OR provider)
//! - `KC_BASE_URL` — KiloCode base URL (default: <https://api.kilo.ai/api/gateway>)
//! - `KC_API_KEY` — KiloCode API key (required for KC provider)
//! - `HKASK_DEFAULT_PROVIDER` — default provider for unprefixed models (DI, FA, TG, OR, KC; default: DI)
//!
//! # API Key Resolution
//!
//! Provider API keys resolve through a 2-tier chain:
//! Keys are resolved:
//! 1. OS keychain (encrypted at rest) — preferred for cloud deployments
//! 2. Environment variables (auto-loaded from .env via dotenvy)
//!
//! # Model Naming Convention
//!
//! Models use a 2-letter provider prefix:
//! - `DI/meta-llama/Llama-3.3-70B-Instruct` → DeepInfra (cloud)
//! - `FA/paddleocr` → fal.ai (cloud)
//! - `TG/Qwen/Qwen2.5-7B-Instruct-Turbo` → Together AI (cloud)
//! - `OR/openai/gpt-4o` → OpenRouter (cloud)
//! - `KC/anthropic/claude-sonnet-4.5` → KiloCode (cloud)
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
    /// OpenRouter (cloud) — prefix `OR/`
    #[serde(rename = "OR")]
    OpenRouter,
    /// KiloCode (cloud) — prefix `KC/`
    #[serde(rename = "KC")]
    KiloCode,
    /// Ollama (local) — prefix `OM/`. No API key required; the OpenAI-compatible
    /// endpoint at `/v1/chat/completions` ignores the `Authorization` header.
    #[serde(rename = "OM")]
    Ollama,
    /// Cline (cloud) — prefix `CL/`. OpenAI-compatible gateway at `api.cline.bot`
    /// routing to Anthropic/OpenAI/Google/DeepSeek/xAI models behind one key.
    /// Env: `CLINE_API_KEY`, `CLINE_BASE_URL` (default `https://api.cline.bot/api`).
    #[serde(rename = "CL")]
    Cline,
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
    /// post: returns Some((ProviderId, stripped_model)) for DI/, FA/, TG/, RP/, BT/, OR/, KC/ prefixes
    /// post: returns None for unrecognized or missing prefix
    #[must_use]
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
            "OR" => Some((ProviderId::OpenRouter, rest)),
            "KC" => Some((ProviderId::KiloCode, rest)),
            "OM" => Some((ProviderId::Ollama, rest)),
            "CL" => Some((ProviderId::Cline, rest)),
            _ => None,
        }
    }

    /// Format a model name with this provider's prefix.
    ///
    /// expect: "The system normalizes provider responses for monitoring"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — canonical provider-prefixed model naming
    /// pre:  model is non-empty
    /// post: returns "{prefix}/{model}" string
    #[must_use]
    pub fn prefix_model(&self, model: &str) -> String {
        format!("{}/{}", self.as_str(), model)
    }

    /// Two-letter code for this provider.
    ///
    /// expect: "The system normalizes provider responses for monitoring"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — stable provider code for routing
    /// post: returns "DI", "FA", "TG", "RP", "BT", "OR", or "KC"
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::DeepInfra => "DI",
            ProviderId::Fal => "FA",
            ProviderId::Together => "TG",
            ProviderId::Runpod => "RP",
            ProviderId::OpenRouter => "OR",
            ProviderId::KiloCode => "KC",
            ProviderId::Ollama => "OM",
            ProviderId::Cline => "CL",
        }
    }
}

// Configuration for provider-agnostic multi-model fusion.
//
// When set, all text generation calls route through the fusion group by default.
// Individual calls can bypass with `LLMParameters.bypass_fusion = true`.
//
// # Environment Variables
//
// - `HKASK_FUSION_JUDGE_MODEL` — judge model for fusion (e.g., "DI/deepseek-v4-pro")
// - `HKASK_FUSION_PANEL_MODELS` — comma-separated panel models (e.g., "OR/auto,KC/anthropic/claude-sonnet-4.5")
// - `HKASK_FUSION_MODE` — judge deliberation mode (default: synthesis)
// - `HKASK_FUSION_SKILLS` — comma-separated skill anchors for the judge
// - `HKASK_FUSION_MAX_ROUNDS` — max rounds for deliberation mode (default: 5)
// - `HKASK_FUSION_DISABLED` — set to "1" to disable fusion

/// Fusion types moved to hkask-types::fusion — re-exported for back-compat.
pub use hkask_types::fusion::{FusionConfig, FusionMode, FusionSkill};

/// Configuration for the inference router.
///
/// Holds connection settings for DeepInfra, fal.ai, Together AI, and OpenRouter.
/// The router uses this config to construct backends and decide
/// the default provider for unprefixed model names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Default provider for model names without a prefix.
    /// Default: DeepInfra (cloud-first).
    pub default_provider: ProviderId,

    pub deepinfra_base_url: String,
    pub deepinfra_api_key: String,
    pub fal_base_url: String,
    pub fal_media_base_url: String,
    pub fal_queue_base_url: String,
    pub fal_api_key: String,
    pub together_base_url: String,
    pub together_api_key: String,
    pub openrouter_base_url: String,
    pub openrouter_api_key: String,
    /// OpenRouter onboarding thresholds (used by CLI model discovery).
    pub openrouter_max_prompt_price_per_m: f64,
    pub openrouter_min_intelligence_index: f64,
    pub kilocode_base_url: String,
    pub kilocode_api_key: String,
    /// Ollama local inference — defaults to `http://localhost:11434`. The API key
    /// is optional (Ollama ignores it) but kept as `String` for consistency with the
    /// other backends and to support remote Ollama instances that require auth.
    pub ollama_base_url: String,
    pub ollama_api_key: String,
    /// Cline cloud gateway — OpenAI-compatible router at `api.cline.bot`.
    /// Env: `CLINE_API_KEY`, `CLINE_BASE_URL` (default `https://api.cline.bot/api`).
    pub cline_base_url: String,
    pub cline_api_key: String,
    pub timeout_secs: u64,
    pub pool_max_idle: usize,
    pub default_model: String,

    /// Structured fusion configuration (provider-agnostic).
    /// When set, all text generation calls route through fusion by default.
    /// Calls with `LLMParameters.bypass_fusion = true` bypass the override.
    /// Default: None (fusion disabled).
    pub fusion: Option<FusionConfig>,
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
            openrouter_max_prompt_price_per_m: 1.0,
            openrouter_min_intelligence_index: 40.0,
            kilocode_base_url: "https://api.kilo.ai/api/gateway".to_string(),
            kilocode_api_key: String::new(),
            ollama_base_url: "http://localhost:11434".to_string(),
            ollama_api_key: String::new(),
            cline_base_url: "https://api.cline.bot/api".to_string(),
            cline_api_key: String::new(),
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: "KC/z-ai/glm-5.2".to_string(),
            fusion: None,
        }
    }
}

impl InferenceConfig {
    /// Resolve from environment variables and OS keychain.
    ///
    /// API keys resolve keychain-first, then fall back to environment variables.
    ///
    /// expect: "The system resolves inference configuration from the environment"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — inference configuration resolved from environment
    /// post: returns InferenceConfig resolved from env vars and keychain
    /// post: defaults to DeepInfra cloud if env vars unset
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let di = ProviderConfig::from_env("DI", "https://api.deepinfra.com");
        let tg = ProviderConfig::from_env("TG", "https://api.together.xyz");
        let or = ProviderConfig::from_env("OR", "https://openrouter.ai/api");
        let kc = ProviderConfig::from_env("KC", "https://api.kilo.ai/api/gateway");
        let om = ProviderConfig::from_env("OM", "http://localhost:11434");
        // Cline uses `CLINE_` env vars (not `CL_`) per the documented API key name.
        let cline_base_url = std::env::var("CLINE_BASE_URL")
            .unwrap_or_else(|_| "https://api.cline.bot/api".to_string());
        let cline_api_key = resolve_api_key("CLINE_API_KEY");

        let fal_base_url =
            std::env::var("FA_BASE_URL").unwrap_or_else(|_| "https://api.fal.ai".to_string());

        let fal_media_base_url =
            std::env::var("FA_MEDIA_BASE_URL").unwrap_or_else(|_| "https://fal.run".to_string());

        let fal_queue_base_url = std::env::var("FA_QUEUE_BASE_URL")
            .unwrap_or_else(|_| "https://queue.fal.run".to_string());

        let fal_api_key = resolve_api_key("FA_API_KEY");

        let openrouter_max_prompt_price_per_m = env_f64("HKASK_OR_MAX_PRICE", 1.0);
        let openrouter_min_intelligence_index = env_f64("HKASK_OR_MIN_INTELLIGENCE_INDEX", 40.0);

        // Fusion: parse structured env vars.
        let fusion = parse_fusion_config();

        Self {
            default_provider: resolve_default_provider(),
            deepinfra_base_url: di.base_url,
            deepinfra_api_key: di.api_key,
            fal_base_url,
            fal_media_base_url,
            fal_queue_base_url,
            fal_api_key,
            together_base_url: tg.base_url,
            together_api_key: tg.api_key,
            openrouter_base_url: or.base_url,
            openrouter_api_key: or.api_key,
            openrouter_max_prompt_price_per_m,
            openrouter_min_intelligence_index,
            kilocode_base_url: kc.base_url,
            kilocode_api_key: kc.api_key,
            ollama_base_url: om.base_url,
            ollama_api_key: om.api_key,
            cline_base_url,
            cline_api_key,
            timeout_secs: std::env::var("HKASK_HTTP_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
            pool_max_idle: std::env::var("HKASK_HTTP_POOL_MAX_IDLE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(256),
            default_model: std::env::var("HKASK_DEFAULT_MODEL")
                .unwrap_or_else(|_| "KC/z-ai/glm-5.2".to_string()),
            fusion,
        }
    }

    /// Build a reqwest HTTP client with the configured timeout and pool settings.
    ///
    /// expect: "The system resolves inference configuration from the environment"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — bounded HTTP client for regulated requests
    /// post: returns reqwest::Client with timeout and pool settings from config
    #[must_use = "result must be used"]
    pub fn build_client(&self) -> anyhow::Result<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .pool_max_idle_per_host(self.pool_max_idle)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
    }

    /// Provider config for DeepInfra (Bearer auth, /v1/chat/completions).
    pub fn deepinfra_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.deepinfra_base_url.clone(),
            api_key: self.deepinfra_api_key.clone(),
        }
    }

    /// Provider config for Together AI (Bearer auth, /v1/chat/completions).
    pub fn together_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.together_base_url.clone(),
            api_key: self.together_api_key.clone(),
        }
    }

    /// Provider config for OpenRouter (Bearer auth, /v1/chat/completions).
    pub fn openrouter_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.openrouter_base_url.clone(),
            api_key: self.openrouter_api_key.clone(),
        }
    }

    /// Provider config for KiloCode (Bearer auth, /chat/completions — no /v1).
    pub fn kilocode_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.kilocode_base_url.clone(),
            api_key: self.kilocode_api_key.clone(),
        }
    }

    /// Provider config for Ollama (Bearer auth ignored, /v1/chat/completions).
    ///
    /// Unlike cloud providers, Ollama does not require an API key. The key is
    /// passed through anyway (sent as `Authorization: Bearer <key>`, ignored by
    /// the server) so remote authenticated Ollama instances keep working.
    pub fn ollama_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.ollama_base_url.clone(),
            api_key: self.ollama_api_key.clone(),
        }
    }

    /// Provider config for Cline (Bearer auth, /v1/chat/completions).
    pub fn cline_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.cline_base_url.clone(),
            api_key: self.cline_api_key.clone(),
        }
    }
}

// ── Private resolution helpers ──────────────────────────────────────────────

/// Resolve a provider API key through the 2-tier chain: keychain → env var.
///
/// Tier 1: OS keychain (encrypted at rest, preferred for cloud deployments).
/// Resolve an API key via OS keychain, then environment variable.
/// Returns empty string if no key is found — the backend will be unavailable.
fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}

fn resolve_api_key(env_name: &str) -> String {
    // Tier 1: Environment variable (fast path — .env is loaded by dotenvy)
    if let Ok(val) = std::env::var(env_name)
        && !val.is_empty()
    {
        return val;
    }

    // Tier 2: OS keychain (guarded against concurrent-access SIGABRT from libdbus)
    let keychain_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        hkask_keystore::resolve(&SecretRef::Keychain(env_name.to_string()))
    }));
    if let Ok(Ok(zeroizing)) = keychain_result {
        let key = String::from_utf8_lossy(&zeroizing).into_owned();
        if !key.is_empty() {
            return key;
        }
    }

    // Tier 3: Empty string (provider not configured)
    String::new()
}

/// Resolve the default provider from env var or keychain.
///
/// Reads `HKASK_DEFAULT_PROVIDER` from OS keychain first, then environment
/// variable. Accepted values: DI, FA, TG. Defaults to DeepInfra.
fn resolve_default_provider() -> ProviderId {
    let raw = resolve_api_key("HKASK_DEFAULT_PROVIDER");
    parse_provider_code(&raw)
}

/// Parse a provider code string to a ProviderId.
///
/// Accepted values: DI, FA, TG, RP, BT, OR, KC. Anything else (including empty) → DeepInfra.
fn parse_provider_code(raw: &str) -> ProviderId {
    match raw {
        "DI" => ProviderId::DeepInfra,
        "FA" => ProviderId::Fal,
        "TG" => ProviderId::Together,
        "RP" => ProviderId::Runpod,
        "OR" => ProviderId::OpenRouter,
        "KC" => ProviderId::KiloCode,
        "OM" => ProviderId::Ollama,
        "CL" => ProviderId::Cline,
        _ => ProviderId::DeepInfra,
    }
}

/// Parse fusion configuration from environment variables.
///
/// Returns `None` if no fusion is configured.
fn parse_fusion_config() -> Option<FusionConfig> {
    // Explicit disable: HKASK_FUSION_DISABLED=1
    if std::env::var("HKASK_FUSION_DISABLED")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return None;
    }

    // Parse shared optional fields
    let mode = std::env::var("HKASK_FUSION_MODE")
        .ok()
        .and_then(|m| m.parse().ok())
        .unwrap_or_default();
    let skills: Vec<FusionSkill> = std::env::var("HKASK_FUSION_SKILLS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let max_rounds = std::env::var("HKASK_FUSION_MAX_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    // Structured config: HKASK_FUSION_JUDGE_MODEL + HKASK_FUSION_PANEL_MODELS
    if let Ok(judge) = std::env::var("HKASK_FUSION_JUDGE_MODEL") {
        let panel: Vec<String> = std::env::var("HKASK_FUSION_PANEL_MODELS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !judge.is_empty() && !panel.is_empty() {
            return Some(FusionConfig {
                judge,
                panel,
                mode,
                skills,
                max_rounds,
            });
        }
    }

    None
}

// ── Provider configuration ───────────────────────────────────────────────────

/// Per-provider connection config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub base_url: String,
    pub api_key: String,
}

impl ProviderConfig {
    /// Resolve base URL and API key from environment using a 2-letter provider prefix.
    ///
    /// Reads `{prefix}_BASE_URL` (falls back to `default_base_url` if unset)
    /// and `{prefix}_API_KEY` (keychain-first, then env).
    pub fn from_env(prefix: &str, default_base_url: &str) -> Self {
        Self {
            base_url: std::env::var(format!("{}_BASE_URL", prefix))
                .unwrap_or_else(|_| default_base_url.to_string()),
            api_key: resolve_api_key(&format!("{}_API_KEY", prefix)),
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
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
            resolve_api_key("HKASK_TEST_KEY_010"),
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX"
        );
        // SAFETY: Test cleanup — see above.
        unsafe { std::env::remove_var("HKASK_TEST_KEY_010") };
    }

    /// expect: "Inference API key missing handling works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates missing key handling
    #[test]
    fn resolve_api_key_empty_when_missing() {
        // SAFETY: Test cleanup — removing environment variables is safe in single-threaded test context.
        unsafe {
            std::env::remove_var("HKASK_TEST_KEY_012");
        }
        assert_eq!(resolve_api_key("HKASK_TEST_KEY_012"), "");
    }
}
