//! Inference configuration — multi-provider routing for DeepInfra, fal.ai, Together AI, OpenRouter, and KiloCode.
//!
//! # Environment Variables
//!
//! - `DI_BASE_URL` — DeepInfra base URL (default: <https://api.deepinfra.com>)
//! - `DI_API_KEY` — DeepInfra API key (required for DI provider)
//! - `FA_BASE_URL` — fal.ai base URL (default: <https://api.fal.ai>)
//! - `HKASK_FAL_API_KEY` — fal.ai API key (required for FA provider)
//! - `TG_BASE_URL` — Together AI base URL (default: <https://api.together.xyz>)
//! - `TOGETHER_API_KEY` — Together AI API key (required for TG provider)
//! - `OR_BASE_URL` — OpenRouter base URL (default: <https://openrouter.ai/api>)
//! - `OPENROUTER_API_KEY` — OpenRouter API key (required for OR provider)
//! - `KC_BASE_URL` — KiloCode base URL (default: <https://api.kilo.ai/api/gateway>)
//! - `KILOCODE_API_KEY` — KiloCode API key (required for KC provider)
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
    /// Baseten (cloud) — prefix `BT/`
    #[serde(rename = "BT")]
    Baseten,
    /// OpenRouter (cloud) — prefix `OR/`
    #[serde(rename = "OR")]
    OpenRouter,
    /// KiloCode (cloud) — prefix `KC/`
    #[serde(rename = "KC")]
    KiloCode,
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
            "KC" => Some((ProviderId::KiloCode, rest)),
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
    /// post: returns "DI", "FA", "TG", "RP", "BT", "OR", or "KC"
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::DeepInfra => "DI",
            ProviderId::Fal => "FA",
            ProviderId::Together => "TG",
            ProviderId::Runpod => "RP",
            ProviderId::Baseten => "BT",
            ProviderId::OpenRouter => "OR",
            ProviderId::KiloCode => "KC",
        }
    }
}

// Configuration for a single OpenRouter fusion group.
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

/// Judge deliberation mode for fusion orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FusionMode {
    /// Pick the single best panel response. No synthesis.
    #[serde(rename = "best-of-n")]
    BestOfN,
    /// Compose a unified response incorporating best elements from all panelists.
    #[serde(rename = "synthesis")]
    #[default]
    Synthesis,
    /// 2-round: draft → panel critique → revised final.
    #[serde(rename = "critique")]
    Critique,
    /// Multi-round deliberation with convergence check (up to max_rounds).
    #[serde(rename = "deliberation")]
    Deliberation,
    /// 2-phase Plan-Implement: Phase 1 synthesizes strategy, Phase 2 synthesizes execution plan.
    #[serde(rename = "pi")]
    PlanImplement,
}

impl FusionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            FusionMode::BestOfN => "best-of-n",
            FusionMode::Synthesis => "synthesis",
            FusionMode::Critique => "critique",
            FusionMode::Deliberation => "deliberation",
            FusionMode::PlanImplement => "pi",
        }
    }
}

impl std::str::FromStr for FusionMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "best-of-n" => Ok(FusionMode::BestOfN),
            "synthesis" => Ok(FusionMode::Synthesis),
            "critique" => Ok(FusionMode::Critique),
            "deliberation" => Ok(FusionMode::Deliberation),
            "pi" => Ok(FusionMode::PlanImplement),
            _ => Ok(FusionMode::Synthesis),
        }
    }
}

/// Skill bundle that anchors the judge's reasoning framework.
/// Each skill injects a compact methodology prompt into the judge's system context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionSkill {
    /// Pragmatic Semantics — classify statements by certainty, distinguish IS from OUGHT.
    #[serde(rename = "pragmatic-semantics")]
    PragmaticSemantics,
    /// Pragmatic Cybernetics — feedback loops, variety engineering, homeostasis.
    #[serde(rename = "pragmatic-cybernetics")]
    PragmaticCybernetics,
    /// Pragmatic Laziness — path of least action, procedural composition.
    #[serde(rename = "pragmatic-laziness")]
    PragmaticLaziness,
    /// Coding Guidelines — Karpathy's four principles: think first, simplicity, surgical, goal-driven.
    #[serde(rename = "coding-guidelines")]
    CodingGuidelines,
    /// Deep Module — deletion test, interface minimalism, dependency direction.
    #[serde(rename = "deep-module")]
    DeepModule,
    /// Essentialist — recursive eliminative interrogation, 3-gate challenge loop.
    #[serde(rename = "essentialist")]
    Essentialist,
    /// Superforecasting — calibrated probability, Fermi decomposition, Bayesian updating.
    #[serde(rename = "superforecasting")]
    Superforecasting,
    /// Multi-Criteria Decision Analysis — weighted scoring, sensitivity analysis.
    #[serde(rename = "mcda")]
    MCDA,
    /// TDD — red-green-refactor, vertical tracer-bullet, contract-first.
    #[serde(rename = "tdd")]
    TestDrivenDevelopment,
    /// Rust Expertise — type-driven design, ownership as architecture.
    #[serde(rename = "rust-expertise")]
    RustExpertise,
}

impl std::str::FromStr for FusionSkill {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "pragmatic-semantics" => Ok(FusionSkill::PragmaticSemantics),
            "pragmatic-cybernetics" => Ok(FusionSkill::PragmaticCybernetics),
            "pragmatic-laziness" => Ok(FusionSkill::PragmaticLaziness),
            "coding-guidelines" => Ok(FusionSkill::CodingGuidelines),
            "deep-module" => Ok(FusionSkill::DeepModule),
            "essentialist" => Ok(FusionSkill::Essentialist),
            "superforecasting" => Ok(FusionSkill::Superforecasting),
            "mcda" => Ok(FusionSkill::MCDA),
            "tdd" => Ok(FusionSkill::TestDrivenDevelopment),
            "rust-expertise" => Ok(FusionSkill::RustExpertise),
            _ => Err(()),
        }
    }
}

/// Configuration for fusion multi-model deliberation.
///
/// Provider-agnostic: hKask orchestrates the fusion itself by sending
/// the prompt to all panel models in parallel, collecting responses,
/// then having the judge operate in the configured mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    /// The judge/fuser model that orchestrates and synthesizes the fusion.
    /// Supports provider prefix routing (e.g., "DI/deepseek-v4-pro").
    pub judge: String,
    /// The panel of analysis models that answer in parallel.
    /// Each model supports provider prefix routing.
    pub panel: Vec<String>,
    /// Judge deliberation mode. Default: Synthesis.
    #[serde(default)]
    pub mode: FusionMode,
    /// Skills that anchor the judge's reasoning framework.
    /// Default: empty (no skill anchoring).
    #[serde(default)]
    pub skills: Vec<FusionSkill>,
    /// Max rounds for deliberation mode. Default: 5.
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
}

fn default_max_rounds() -> u32 {
    5
}

impl FusionConfig {
    /// The kask default panel models for multi-model deliberation.
    pub const KASK_PANEL: &[&str] = &["Kimi2.7", "Qwen3.7 Max", "GLM5.2", "Minimax3"];

    /// The kask default judge/fuser model.
    pub const KASK_JUDGE: &str = "deepseek-v4-pro";

    /// Return the kask default fusion configuration.
    pub fn kask_default() -> Self {
        Self {
            judge: Self::KASK_JUDGE.to_string(),
            panel: Self::KASK_PANEL.iter().map(|s| s.to_string()).collect(),
            mode: FusionMode::Synthesis,
            skills: Vec::new(),
            max_rounds: 5,
        }
    }

    /// The model ID to use when fusion is active (judge model).
    pub fn model_id(&self) -> String {
        self.judge.clone()
    }

    /// Human-readable description of the fusion setup.
    pub fn description(&self) -> String {
        let skills_str = if self.skills.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = self
                .skills
                .iter()
                .map(|s| match s {
                    FusionSkill::PragmaticSemantics => "pragmatic-semantics",
                    FusionSkill::PragmaticCybernetics => "pragmatic-cybernetics",
                    FusionSkill::PragmaticLaziness => "pragmatic-laziness",
                    FusionSkill::CodingGuidelines => "coding-guidelines",
                    FusionSkill::DeepModule => "deep-module",
                    FusionSkill::Essentialist => "essentialist",
                    FusionSkill::Superforecasting => "superforecasting",
                    FusionSkill::MCDA => "mcda",
                    FusionSkill::TestDrivenDevelopment => "tdd",
                    FusionSkill::RustExpertise => "rust-expertise",
                })
                .collect();
            format!(" [{}]", names.join(", "))
        };
        format!(
            "{} panel models judged by {} (mode: {}){}",
            self.panel.len(),
            self.judge,
            self.mode.as_str(),
            skills_str
        )
    }
}

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
    pub kilocode_base_url: String,
    pub kilocode_api_key: String,
    pub timeout_secs: u64,
    pub pool_max_idle: usize,
    pub default_model: String,

    /// Structured OpenRouter fusion group configuration.
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
            kilocode_base_url: "https://api.kilo.ai/api/gateway".to_string(),
            kilocode_api_key: String::new(),
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: "deepseek-v4-pro".to_string(),
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
        let deepinfra_base_url = std::env::var("DI_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepinfra.com".to_string());

        let deepinfra_api_key = resolve_api_key("DI_API_KEY");

        let fal_base_url =
            std::env::var("FA_BASE_URL").unwrap_or_else(|_| "https://api.fal.ai".to_string());

        let fal_media_base_url =
            std::env::var("FA_MEDIA_BASE_URL").unwrap_or_else(|_| "https://fal.run".to_string());

        let fal_queue_base_url = std::env::var("FA_QUEUE_BASE_URL")
            .unwrap_or_else(|_| "https://queue.fal.run".to_string());

        let fal_api_key = resolve_api_key("HKASK_FAL_API_KEY");

        let together_base_url =
            std::env::var("TG_BASE_URL").unwrap_or_else(|_| "https://api.together.xyz".to_string());

        let together_api_key = resolve_api_key("TOGETHER_API_KEY");

        let openrouter_base_url = std::env::var("OR_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api".to_string());

        let openrouter_api_key = resolve_api_key("OPENROUTER_API_KEY");

        let kilocode_base_url = std::env::var("KC_BASE_URL")
            .unwrap_or_else(|_| "https://api.kilo.ai/api/gateway".to_string());

        let kilocode_api_key = resolve_api_key("KILOCODE_API_KEY");

        let default_provider = resolve_default_provider();

        // Fusion: parse structured env vars.
        let fusion = parse_fusion_config();

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
            kilocode_base_url,
            kilocode_api_key,
            timeout_secs: 120,
            pool_max_idle: 5,
            default_model: std::env::var("HKASK_DEFAULT_MODEL")
                .unwrap_or_else(|_| "deepseek-v4-pro".to_string()),
            fusion,
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
/// Resolve an API key via OS keychain, then environment variable.
/// Returns empty string if no key is found — the backend will be unavailable.
fn resolve_api_key(env_name: &str) -> String {
    // Tier 1: OS keychain
    if let Ok(zeroizing) = hkask_keystore::resolve(&SecretRef::Keychain(env_name.to_string())) {
        let key = String::from_utf8_lossy(&zeroizing).into_owned();
        if !key.is_empty() {
            return key;
        }
    }

    // Tier 2: Environment variable
    std::env::var(env_name).unwrap_or_default()
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
        "BT" => ProviderId::Baseten,
        "OR" => ProviderId::OpenRouter,
        "KC" => ProviderId::KiloCode,
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
