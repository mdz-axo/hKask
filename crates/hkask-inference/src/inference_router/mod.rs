//! Inference router — multi-provider `InferencePort` implementation.
//!
//! Routes requests to DeepInfra, fal.ai, Together AI, OpenRouter, or KiloCode based on the
//! 2-letter provider prefix in the model name. Unprefixed model names
//! use the configured default provider.

use crate::config::{FusionConfig, InferenceConfig, ProviderId};
use crate::deepinfra_backend::DeepInfraBackend;
use crate::embedding_router::EmbeddingRouter;
use crate::fal_backend::FalBackend;
use crate::kilocode_backend::KiloCodeBackend;
use crate::openrouter_backend::OpenRouterBackend;
use crate::together_backend::TogetherBackend;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult};
use hkask_types::template::LLMParameters;
use std::sync::Arc;
use tracing::warn;

mod dispatch;
mod inference_port;
mod media;
mod models;

/// Error healing callback: (error_string, operation_name).
type HealCallback = Box<dyn Fn(&str, &str) + Send + Sync>;

/// Multi-provider inference router implementing `InferencePort`.
///
/// Parses the `XX/` prefix from model names and dispatches to the
/// appropriate backend. Each backend owns its own HTTP client, auth,
/// and model listing endpoint.
pub struct InferenceRouter {
    config: InferenceConfig,
    deepinfra: Option<DeepInfraBackend>,
    fal: Option<FalBackend>,
    together: Option<TogetherBackend>,
    openrouter: Option<OpenRouterBackend>,
    kilocode: Option<KiloCodeBackend>,
    embedding: EmbeddingRouter,
    /// Optional error healing callback. Takes (error_string, operation_name).
    /// Wire this to a SelfHealer instance at construction.
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
    /// expect: "The system creates multi-provider membranes assembled from configured boundaries"
    /// \[P4\] Motivating: Clear Boundaries — multi-provider membrane assembled from configured boundaries
    /// pre:  config is a valid InferenceConfig
    /// post: returns InferenceRouter with backends for configured providers
    pub fn new(config: InferenceConfig) -> Self {
        let deepinfra = DeepInfraBackend::new(&config).ok();
        let fal = FalBackend::new(&config).ok();
        let together = TogetherBackend::new(&config).ok();
        let openrouter = OpenRouterBackend::new(&config).ok();
        let kilocode = KiloCodeBackend::new(&config).ok();

        if deepinfra.is_none() {
            warn!(target: "cns.inference", "DeepInfra backend unavailable (no API key)");
        }
        if fal.is_none() {
            warn!(target: "cns.inference", "fal.ai backend unavailable (no API key)");
        }
        if together.is_none() {
            warn!(target: "cns.inference", "Together AI backend unavailable (no API key)");
        }
        if openrouter.is_none() {
            warn!(target: "cns.inference", "OpenRouter backend unavailable (no API key)");
        }
        if kilocode.is_none() {
            warn!(target: "cns.inference", "KiloCode backend unavailable (no API key)");
        }

        let shared_client = config.build_client().map(Arc::new).ok();
        let embedding = shared_client
            .as_ref()
            .map(|c| EmbeddingRouter::with_client(&config, Arc::clone(c)))
            .unwrap_or_else(|| EmbeddingRouter::new(config.clone()));

        Self {
            config: config.clone(),
            deepinfra,
            fal,
            together,
            openrouter,
            kilocode,
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
    /// When `config.fusion` is Some AND `params.bypass_fusion` is false,
    /// the fusion model ID is used regardless of the explicit or default model.
    /// Otherwise, falls back to the explicit model or config.default_model.
    fn effective_model(&self, explicit: Option<&str>, params: &LLMParameters) -> String {
        if !params.bypass_fusion
            && let Some(ref fusion) = self.config.fusion
        {
            return fusion.model_id();
        }
        explicit.unwrap_or(&self.config.default_model).to_string()
    }

    /// Resolve which backend to use for a given model name.
    ///
    /// Returns `(provider, backend_model_name)` or an error if no backend
    /// is available for the requested provider.
    pub(crate) fn resolve<'a>(
        &self,
        model: &'a str,
    ) -> Result<(ProviderId, &'a str), InferenceError> {
        let (provider, stripped_model) =
            ProviderId::parse_from_model(model).unwrap_or((self.config.default_provider, model));

        let available = match provider {
            ProviderId::DeepInfra => self.deepinfra.is_some(),
            ProviderId::Fal => self.fal.is_some(),
            ProviderId::Together => self.together.is_some(),
            ProviderId::OpenRouter => self.openrouter.is_some(),
            ProviderId::KiloCode => self.kilocode.is_some(),
            ProviderId::Runpod | ProviderId::Baseten => false,
        };

        if !available {
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

    /// Verify that the configured fusion judge model is reachable.
    ///
    /// Resolves the judge model name to a provider and checks that
    /// the backend is available. Returns Ok(true) if reachable,
    /// Ok(false) if not, or Err on resolution failure.
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
                    "Fusion judge model reachable"
                );
                Ok(true)
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.inference",
                    fusion_judge = %fusion.judge,
                    error = %e,
                    "Fusion judge model NOT reachable"
                );
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FusionConfig, FusionMode, InferenceConfig};

    fn config_with_fusion(judge: Option<&str>, panel: Option<&[&str]>) -> InferenceConfig {
        InferenceConfig {
            fusion: judge.map(|j| FusionConfig {
                judge: j.to_string(),
                panel: panel.unwrap_or(&[]).iter().map(|s| s.to_string()).collect(),
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
}
