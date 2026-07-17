//! Inference router — multi-provider `InferencePort` implementation.
//!
//! Routes requests to DeepInfra, fal.ai, Together AI, OpenRouter, or KiloCode based on the
//! 2-letter provider prefix in the model name. Unprefixed model names
//! use the configured default provider.

use crate::cline_backend::ClineBackend;
use crate::config::{FusionConfig, InferenceConfig, ProviderId};
use crate::deepinfra_backend::DeepInfraBackend;
use crate::embedding_router::EmbeddingRouter;
use crate::fal_backend::FalBackend;
use crate::kilocode_backend::KiloCodeBackend;
use crate::ollama_backend::OllamaBackend;
use crate::openrouter_backend::OpenRouterBackend;
use crate::runpod_backend::RunpodBackend;
use crate::together_backend::TogetherBackend;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult};
use hkask_types::template::LLMParameters;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

pub(crate) mod backend;
mod dispatch;
mod inference_port;
mod media;
mod models;
pub(crate) use backend::{ChatBackend, VisionBackend};

type HealCallback = Box<dyn Fn(&str, &str) + Send + Sync>;

/// Multi-provider inference router implementing `InferencePort`.
///
/// Parses the `XX/` prefix from model names and dispatches to the
/// appropriate backend via trait-object maps. Each backend owns its own HTTP
/// client, auth, and model listing endpoint. Adding a provider = implement
/// `ChatBackend`/`VisionBackend` + insert into the map(s) in `new` — no other
/// dispatch site (`resolve`, `dispatch_generate`, `dispatch_generate_stream`,
/// `media::generate_vision`, `models::list_models`) needs editing.
pub struct InferenceRouter {
    config: InferenceConfig,
    /// Chat-completion backends keyed by provider (7 chat-capable providers;
    /// RunPod is excluded — it is vision/OCR-only).
    chat_backends: HashMap<ProviderId, Arc<dyn ChatBackend>>,
    /// Vision/multimodal backends keyed by provider (all 8, including RunPod).
    vision_backends: HashMap<ProviderId, Arc<dyn VisionBackend>>,
    /// Fal.ai and DeepInfra are also held as typed fields: their specialist media
    /// methods (image generation, TTS, transcription, workflow, background
    /// removal with fallback) are Fal/DeepInfra-specific capabilities not
    /// covered by `ChatBackend`/`VisionBackend`. The same `Arc` is shared with
    /// the maps above, so there is no double-construction.
    fal: Option<Arc<FalBackend>>,
    deepinfra: Option<Arc<DeepInfraBackend>>,
    embedding: EmbeddingRouter,
    heal_error_cb: Option<HealCallback>,
}

impl InferenceRouter {
    /// Build the router from an `InferenceConfig`.
    ///
    /// Create a new inference router from config.
    ///
    /// Constructs backends lazily — a backend is only created if its
    /// configuration is valid (e.g., API key is present for cloud providers).
    ///
    /// # Availability gate (coding-guidelines + pragmatic-laziness)
    ///
    /// Gates on **configuration**, not reachability: cloud backends require a
    /// non-empty API key; Ollama requires a non-empty base URL (defaults to
    /// `localhost:11434`, so it is `Some` even when the daemon is down). A `None`
    /// backend is skipped by `list_models` (contributes no models) and rejected by
    /// `resolve` (clear "provider not available" error) — so unconfigured providers
    /// never appear in the model list and can't be used for inference.
    ///
    /// Reachability is detected **lazily at use**, not at construction: an Ollama
    /// daemon that is down yields an empty `list_models` and a connection error on
    /// dispatch. This avoids a blocking network probe in the sync constructor and
    /// keeps startup fast — the cheapest knowable signal (config presence) gates
    /// construction; the runtime signal (reachability) gates use.
    ///
    /// expect: "The system creates multi-provider membranes assembled from configured boundaries"
    /// \[P4\] Motivating: Clear Boundaries — multi-provider membrane assembled from configured boundaries
    /// pre:  config is a valid InferenceConfig
    /// post: returns InferenceRouter with backends for configured providers
    pub fn new(config: InferenceConfig) -> Self {
        let shared_client = config
            .build_client()
            .map(Arc::new)
            .map_err(|e| warn!(target: "cns.inference", "HTTP client build failed: {}", e))
            .ok();

        let mut chat_backends: HashMap<ProviderId, Arc<dyn ChatBackend>> = HashMap::new();
        let mut vision_backends: HashMap<ProviderId, Arc<dyn VisionBackend>> = HashMap::new();

        if let Some(ref c) = shared_client {
            // Chat-capable providers implement both ChatBackend and VisionBackend.
            // Fal and DeepInfra are ALSO held as typed fields (see struct docs) for
            // their specialist media methods — the same Arc is shared with the maps.
            let deepinfra = DeepInfraBackend::new(&config, Arc::clone(c))
                .ok()
                .map(Arc::new);
            match &deepinfra {
                Some(b) => {
                    chat_backends.insert(ProviderId::DeepInfra, Arc::clone(b));
                    vision_backends.insert(
                        ProviderId::DeepInfra,
                        Arc::clone(b) as Arc<dyn VisionBackend>,
                    );
                }
                None => {
                    warn!(target: "cns.inference", "DeepInfra backend unavailable (no API key)")
                }
            }
            let fal = FalBackend::new(&config, Arc::clone(c)).ok().map(Arc::new);
            match &fal {
                Some(b) => {
                    chat_backends.insert(ProviderId::Fal, Arc::clone(b));
                    vision_backends
                        .insert(ProviderId::Fal, Arc::clone(b) as Arc<dyn VisionBackend>);
                }
                None => {
                    warn!(target: "cns.inference", "fal.ai backend unavailable (no API key)")
                }
            }
            match TogetherBackend::new(&config, Arc::clone(c)) {
                Ok(b) => {
                    let b = Arc::new(b);
                    chat_backends.insert(ProviderId::Together, Arc::clone(&b));
                    vision_backends.insert(ProviderId::Together, b);
                }
                Err(_) => {
                    warn!(target: "cns.inference", "Together AI backend unavailable (no API key)")
                }
            }
            match OpenRouterBackend::new(&config, Arc::clone(c)) {
                Ok(b) => {
                    let b = Arc::new(b);
                    chat_backends.insert(ProviderId::OpenRouter, Arc::clone(&b));
                    vision_backends.insert(ProviderId::OpenRouter, b);
                }
                Err(_) => {
                    warn!(target: "cns.inference", "OpenRouter backend unavailable (no API key)")
                }
            }
            match KiloCodeBackend::new(&config, Arc::clone(c)) {
                Ok(b) => {
                    let b = Arc::new(b);
                    chat_backends.insert(ProviderId::KiloCode, Arc::clone(&b));
                    vision_backends.insert(ProviderId::KiloCode, b);
                }
                Err(_) => {
                    warn!(target: "cns.inference", "KiloCode backend unavailable (no API key)")
                }
            }
            // RunPod is vision/OCR-only — it is NOT a ChatBackend.
            match RunpodBackend::new(&config, Arc::clone(c)) {
                Ok(b) => {
                    vision_backends.insert(ProviderId::Runpod, Arc::new(b));
                }
                Err(_) => {
                    warn!(target: "cns.inference", "RunPod backend unavailable (no API key or template)")
                }
            }
            match OllamaBackend::new(&config, Arc::clone(c)) {
                Ok(b) => {
                    let b = Arc::new(b);
                    chat_backends.insert(ProviderId::Ollama, Arc::clone(&b));
                    vision_backends.insert(ProviderId::Ollama, b);
                }
                Err(_) => {
                    warn!(target: "cns.inference", "Ollama backend unavailable (no base URL)")
                }
            }
            match ClineBackend::new(&config, Arc::clone(c)) {
                Ok(b) => {
                    let b = Arc::new(b);
                    chat_backends.insert(ProviderId::Cline, Arc::clone(&b));
                    vision_backends.insert(ProviderId::Cline, b);
                }
                Err(_) => warn!(target: "cns.inference", "Cline backend unavailable (no API key)"),
            }
        }

        let embedding = shared_client
            .as_ref()
            .map(|c| EmbeddingRouter::with_client(&config, Arc::clone(c)))
            .unwrap_or_else(|| EmbeddingRouter::new(config.clone()));

        Self {
            config: config.clone(),
            chat_backends,
            vision_backends,
            fal,
            deepinfra,
            embedding,
            heal_error_cb: None,
        }
    }

    /// Attach a self-healing callback for automatic error recovery.
    /// The callback receives (error_string, operation_name) and should
    /// delegate to a SelfHealer instance.
    pub fn with_heal_cb(mut self, cb: HealCallback) -> Self {
        self.heal_error_cb = Some(cb);
        self
    }

    fn heal_error(&self, error: InferenceError, operation: &str) -> InferenceError {
        if let Some(ref cb) = self.heal_error_cb {
            cb(&error.to_string(), operation);
        }
        error
    }

    /// Compute the effective model name, applying the fusion override when active.
    ///
    /// Priority: per-call `params.fusion_config` > global `config.fusion` >
    /// explicit model > default model. When `params.bypass_fusion` is true,
    /// all fusion overrides are skipped.
    fn effective_model(&self, explicit: Option<&str>, params: &LLMParameters) -> String {
        if !params.bypass_fusion {
            if let Some(fusion) = &params.fusion_config {
                return fusion.model_id();
            }
            if let Some(ref fusion) = self.config.fusion {
                return fusion.model_id();
            }
        }
        explicit.unwrap_or(&self.config.default_model).to_string()
    }

    /// Parse a model name into `(provider, stripped_model)` with unknown-prefix rejection.
    ///
    /// Unprefixed names (no `XX/` shape) route to the configured default provider.
    /// A name with two uppercase letters followed by `/` that is not a recognized
    /// provider code is rejected explicitly (fail fast on unknown prefix).
    ///
    /// expect: "The system parses provider routing from model names"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — fail fast on unknown prefix
    /// pre:  model is non-empty
    /// post: returns Ok((ProviderId, stripped_model)) for recognized or unprefixed models
    /// post: returns Err(Connection) for unrecognized `XX/` prefix
    fn parse_provider<'a>(&self, model: &'a str) -> Result<(ProviderId, &'a str), InferenceError> {
        match ProviderId::parse_from_model(model) {
            Some(parsed) => Ok(parsed),
            None => {
                if ProviderId::looks_like_prefix(model) {
                    return Err(InferenceError::Connection(format!(
                        "Unknown provider prefix '{}/' in model '{}'",
                        &model[..2],
                        model
                    )));
                }
                Ok((self.config.default_provider, model))
            }
        }
    }

    /// Resolve which chat backend to use for a given model name.
    ///
    /// Returns `(provider, backend_model_name)` or an error if no chat backend
    /// is available for the requested provider, or if the model carries an
    /// unrecognized `XX/` prefix.
    ///
    /// Unprefixed names (no `XX/` shape) route to the configured default
    /// provider. A name with two uppercase letters followed by `/` that is
    /// not a recognized provider code is rejected explicitly — previously
    /// such names silently fell through to the default provider and were
    /// sent upstream as a garbage model identifier.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — fail fast on unknown prefix
    /// pre:  model is non-empty
    /// post: returns Ok((ProviderId, stripped_model)) for recognized or unprefixed models
    /// post: returns Err(Connection) for unrecognized `XX/` prefix or unavailable backend
    pub(crate) fn resolve<'a>(
        &self,
        model: &'a str,
    ) -> Result<(ProviderId, &'a str), InferenceError> {
        let (provider, stripped_model) = self.parse_provider(model)?;
        if !self.chat_backends.contains_key(&provider) {
            return Err(InferenceError::Connection(format!(
                "Provider {} is not available (check configuration)",
                provider.as_str()
            )));
        }
        Ok((provider, stripped_model))
    }

    // ── Embedding ──────────────────────────────────────────────────────────

    /// Generate a text embedding vector via the embedding router.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated embedding dispatch
    /// pre:  text is a non-empty string
    /// post: delegates to EmbeddingRouter::embed_sentence with resolved model
    /// post: if embedding fails → Err(EmbeddingGenerationError)
    #[must_use = "result must be used"]
    pub async fn embed_text(
        &self,
        text: &str,
        model_override: Option<&str>,
    ) -> Result<Vec<f32>, hkask_ports::EmbeddingGenerationError> {
        let model = model_override.unwrap_or(&self.config.default_model);
        self.embedding.embed_sentence(model, text).await
    }

    /// Provider-agnostic fusion orchestration.
    ///
    /// Delegates to the fusion orchestrator which dispatches to panel
    /// models in parallel, then routes to the configured fusion mode.
    ///
    /// expect: "Fusion orchestrates multi-model deliberation provider-agnostically"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — hKask-side fusion orchestration
    /// pre:  fusion.panel is non-empty, fusion.judge is valid
    /// post: returns judge output per the configured mode
    async fn orchestrate_fusion(
        &self,
        prompt: &str,
        params: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
        fusion: &FusionConfig,
    ) -> Result<InferenceResult, InferenceError> {
        crate::fusion_orchestrator::orchestrate(self, prompt, params, tools, fusion).await
    }

    /// Verify that the configured fusion judge model is *configured* (its
    /// provider backend is available).
    ///
    /// Resolves the judge model name to a provider and checks backend
    /// availability (config presence — not a network reachability probe).
    /// Returns Ok(true) if the backend is available, Ok(false) if not, or
    /// Err on resolution failure.
    ///
    /// Note: this is a configuration check, not a reachability check — it
    /// confirms the judge model's provider is configured but does not ping it.
    /// Reachability is detected lazily at use time.
    ///
    /// expect: "Fusion model is verified before use to prevent unexpected costs"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — proactive cost-safety check
    /// pre:  config.fusion may be None or Some
    /// post: if Some → resolves judge model to verify provider availability
    /// post: if None → returns Ok(true) immediately (nothing to verify)
    #[must_use = "result must be used"]
    pub async fn verify_fusion_model(&self) -> Result<bool, InferenceError> {
        let fusion = match &self.config.fusion {
            Some(f) => f,
            None => return Ok(true),
        };

        match self.resolve(&fusion.judge) {
            Ok((provider, _)) => {
                tracing::info!(
                    target: "cns.inference",
                    fusion_judge = %fusion.judge,
                    provider = %provider.as_str(),
                    panel_count = fusion.panel.len(),
                    "Fusion judge model provider is configured"
                );
                Ok(true)
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.inference",
                    fusion_judge = %fusion.judge,
                    error = %e,
                    "Fusion judge model provider NOT configured"
                );
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FusionConfig, FusionMode, InferenceConfig, NonEmptyVec};

    fn config_with_fusion(judge: Option<&str>, panel: Option<&[&str]>) -> InferenceConfig {
        InferenceConfig {
            fusion: judge.map(|j| FusionConfig {
                judge: j.to_string(),
                panel: NonEmptyVec::from_vec(
                    panel
                        .unwrap_or(&["default-panel"])
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                )
                .expect("config_with_fusion panel must not be empty"),
                mode: FusionMode::Synthesis,
                skills: Vec::new(),
                max_rounds: 5,
            }),
            ..Default::default()
        }
    }

    // ── C1: effective_model routing ────────────────────────────────────

    /// REQ: P9-inf-fusion-effective-model-routing
    /// expect: "Fusion model overrides default when configured and not bypassed" [P9]
    #[test]
    fn effective_model_routes_to_fusion() {
        let config = config_with_fusion(Some("kask"), Some(&["Kimi2.7", "Qwen3.7 Max"]));
        let router = InferenceRouter::new(config);
        let params = LLMParameters {
            bypass_fusion: false,
            ..Default::default()
        };
        assert_eq!(router.effective_model(None, &params), "kask");
    }

    /// REQ: P9-inf-fusion-bypass
    /// expect: "Bypass flag prevents fusion override" [P9]
    #[test]
    fn effective_model_bypasses_fusion() {
        let config = config_with_fusion(Some("kask"), Some(&["Kimi2.7"]));
        let default = config.default_model.clone();
        let router = InferenceRouter::new(config);
        let params = LLMParameters {
            bypass_fusion: true,
            ..Default::default()
        };
        assert_eq!(router.effective_model(None, &params), default);
    }

    /// REQ: P9-inf-fusion-effective-model-explicit
    /// expect: "Explicit model used when fusion is None" [P9]
    #[test]
    fn effective_model_uses_explicit_when_no_fusion() {
        let config = config_with_fusion(None, None);
        let router = InferenceRouter::new(config);
        let params = LLMParameters::default();
        assert_eq!(
            router.effective_model(Some("DI/custom-model"), &params),
            "DI/custom-model"
        );
    }

    /// REQ: P9-inf-fusion-default-fallback
    /// expect: "Default model used when nothing overrides" [P9]
    #[test]
    fn effective_model_falls_back_to_default() {
        let config = config_with_fusion(None, None);
        let default = config.default_model.clone();
        let router = InferenceRouter::new(config);
        let params = LLMParameters::default();
        assert_eq!(router.effective_model(None, &params), default);
    }

    // ── C2: Per-call fusion_config override ───────────────────────────

    /// REQ: P9-inf-fusion-per-call-override
    /// expect: "Per-call fusion_config overrides global config judge model" [P9]
    #[test]
    fn per_call_fusion_config_overrides_global() {
        let config = config_with_fusion(Some("global-judge"), Some(&["Kimi2.7"]));
        let router = InferenceRouter::new(config);
        let params = LLMParameters {
            bypass_fusion: false,
            fusion_config: Some(FusionConfig {
                judge: "manifest-judge".to_string(),
                panel: NonEmptyVec::one("Qwen3.7 Max".to_string()),
                mode: FusionMode::Critique,
                skills: Vec::new(),
                max_rounds: 3,
            }),
            system_prompt: None,
            ..Default::default()
        };
        assert_eq!(router.effective_model(None, &params), "manifest-judge");
    }

    /// REQ: P9-inf-fusion-per-call-no-global
    /// expect: "Per-call fusion_config used when no global config exists" [P9]
    #[test]
    fn per_call_fusion_config_without_global() {
        let config = config_with_fusion(None, None);
        let router = InferenceRouter::new(config);
        let params = LLMParameters {
            bypass_fusion: false,
            fusion_config: Some(FusionConfig {
                judge: "manifest-only-judge".to_string(),
                panel: NonEmptyVec::one("GLM5.2".to_string()),
                mode: FusionMode::Synthesis,
                skills: Vec::new(),
                max_rounds: 5,
            }),
            system_prompt: None,
            ..Default::default()
        };
        assert_eq!(router.effective_model(None, &params), "manifest-only-judge");
    }

    /// REQ: P9-inf-fusion-per-call-bypass
    /// expect: "Bypass flag overrides per-call fusion_config" [P9]
    #[test]
    fn per_call_fusion_config_bypassed() {
        let config = config_with_fusion(Some("global-judge"), Some(&["Kimi2.7"]));
        let default = config.default_model.clone();
        let router = InferenceRouter::new(config);
        let params = LLMParameters {
            bypass_fusion: true,
            fusion_config: Some(FusionConfig {
                judge: "manifest-judge".to_string(),
                panel: NonEmptyVec::one("Qwen3.7 Max".to_string()),
                mode: FusionMode::Synthesis,
                skills: Vec::new(),
                max_rounds: 5,
            }),
            system_prompt: None,
            ..Default::default()
        };
        assert_eq!(router.effective_model(None, &params), default);
    }

    // ── C2: Ollama provider routing ────────────────────────────────────

    /// REQ: P9-inf-ollama-prefix-routing
    /// expect: "OM/ prefix routes to the Ollama provider with the tag stripped" [P9]
    #[test]
    fn parse_from_model_routes_ollama_prefix() {
        assert_eq!(
            ProviderId::parse_from_model("OM/qwen3:8b"),
            Some((ProviderId::Ollama, "qwen3:8b"))
        );
    }

    /// REQ: P9-inf-ollama-prefix-routing
    /// expect: "Ollama provider formats models with the OM/ prefix" [P9]
    #[test]
    fn ollama_prefix_format() {
        assert_eq!(ProviderId::Ollama.prefix_model("qwen3:8b"), "OM/qwen3:8b");
        assert_eq!(ProviderId::Ollama.as_str(), "OM");
    }

    /// REQ: P9-inf-ollama-resolve
    /// expect: "Router resolves an OM/-prefixed model to the Ollama backend" [P9]
    #[test]
    fn resolve_routes_ollama_model() {
        let config = InferenceConfig::default();
        // Default config sets ollama_base_url, so the backend constructs.
        if config.ollama_base_url.is_empty() {
            // Environment stripped the default; skip rather than assert a false negative.
            return;
        }
        let router = InferenceRouter::new(config);
        let (provider, model) = router
            .resolve("OM/qwen3:8b")
            .expect("OM/-prefixed model should resolve");
        assert_eq!(provider, ProviderId::Ollama);
        assert_eq!(model, "qwen3:8b");
    }

    /// REQ: P9-inf-ollama-default-provider
    /// expect: "Unprefixed model routes to Ollama when it is the default provider" [P9]
    #[test]
    fn unprefixed_model_uses_ollama_default() {
        let config = InferenceConfig {
            default_provider: ProviderId::Ollama,
            ..InferenceConfig::default()
        };
        if config.ollama_base_url.is_empty() {
            return;
        }
        let router = InferenceRouter::new(config);
        let (provider, _model) = router
            .resolve("qwen3:8b")
            .expect("unprefixed model with Ollama default should resolve");
        assert_eq!(provider, ProviderId::Ollama);
    }

    // ── C3: Cline provider routing ─────────────────────────────────────

    /// REQ: P9-inf-cline-prefix-routing
    /// expect: "CL/ prefix routes to the Cline provider with the org/model stripped" [P9]
    #[test]
    fn parse_from_model_routes_cline_prefix() {
        assert_eq!(
            ProviderId::parse_from_model("CL/anthropic/claude-sonnet-4-6"),
            Some((ProviderId::Cline, "anthropic/claude-sonnet-4-6"))
        );
    }

    /// REQ: P9-inf-cline-prefix-routing
    /// expect: "Cline provider formats models with the CL/ prefix" [P9]
    #[test]
    fn cline_prefix_format() {
        assert_eq!(
            ProviderId::Cline.prefix_model("openai/gpt-4o"),
            "CL/openai/gpt-4o"
        );
        assert_eq!(ProviderId::Cline.as_str(), "CL");
    }

    /// REQ: P9-inf-cline-resolve
    /// expect: "Router resolves a CL/-prefixed model; unavailable without a key" [P9]
    #[test]
    fn resolve_cline_model_unavailable_without_key() {
        // Default config has no CLINE_API_KEY → cline backend is None → resolve errors.
        let config = InferenceConfig::default();
        let router = InferenceRouter::new(config);
        let err = router.resolve("CL/anthropic/claude-sonnet-4-6");
        assert!(
            err.is_err(),
            "CL/ model should not resolve without CLINE_API_KEY"
        );
    }

    /// REQ: P9-inf-unknown-prefix-reject
    /// expect: "An unrecognized XX/ prefix is rejected, not silently routed to the default provider" [P9]
    #[test]
    fn resolve_unknown_prefix_errors() {
        let config = InferenceConfig {
            default_provider: ProviderId::Ollama,
            ..InferenceConfig::default()
        };
        if config.ollama_base_url.is_empty() {
            return;
        }
        let router = InferenceRouter::new(config);
        assert!(
            router.resolve("qwen3:8b").is_ok(),
            "unprefixed model with Ollama default should resolve"
        );
        let err = router.resolve("BT/foo").unwrap_err();
        assert!(
            err.to_string().contains("Unknown provider prefix"),
            "BT/ should be rejected as unknown prefix"
        );
        let err = router.resolve("XX/model").unwrap_err();
        assert!(
            err.to_string().contains("Unknown provider prefix"),
            "XX/ should be rejected as unknown prefix"
        );
    }
}
