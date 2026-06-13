//! Inference router — multi-provider `InferencePort` implementation.
//!
//! Routes requests to Ollama, Fireworks, DeepInfra, or fal.ai based on the
//! 2-letter provider prefix in the model name. Unprefixed model names
//! use the configured default provider.

use crate::RouterModelEntry;
use crate::chat_protocol::validate_prompt;
use crate::config::{InferenceConfig, ProviderId};
use crate::deepinfra_backend::DeepInfraBackend;
use crate::fal_backend::FalBackend;
use crate::fireworks_backend::FireworksBackend;
use crate::ollama_backend::OllamaBackend;
use hkask_types::LLMParameters;
use hkask_types::ports::{InferenceError, InferencePort, InferenceResult, InferenceStreamChunk};
use std::pin::Pin;
use tracing::warn;

/// Multi-provider inference router implementing `InferencePort`.
///
/// Parses the `XX/` prefix from model names and dispatches to the
/// appropriate backend. Each backend owns its own HTTP client, auth,
/// and model listing endpoint.
pub struct InferenceRouter {
    config: InferenceConfig,
    ollama: Option<OllamaBackend>,
    fireworks: Option<FireworksBackend>,
    deepinfra: Option<DeepInfraBackend>,
    fal: Option<FalBackend>,
}

impl InferenceRouter {
    /// Build the router from an `InferenceConfig`.
    ///
    /// Constructs backends lazily — a backend is only created if its
    /// configuration is valid (e.g., API key is present for cloud providers).
    /// Ollama is always attempted since it requires no auth.
    pub fn new(config: InferenceConfig) -> Self {
        let ollama = OllamaBackend::new(&config).ok();
        let fireworks = FireworksBackend::new(&config).ok();
        let deepinfra = DeepInfraBackend::new(&config).ok();
        let fal = FalBackend::new(&config).ok();

        if ollama.is_none() {
            warn!(target: "hkask.inference", "Ollama backend unavailable");
        }
        if fireworks.is_none() {
            warn!(target: "hkask.inference", "Fireworks backend unavailable (no API key)");
        }
        if deepinfra.is_none() {
            warn!(target: "hkask.inference", "DeepInfra backend unavailable (no API key)");
        }
        if fal.is_none() {
            warn!(target: "hkask.inference", "fal.ai backend unavailable (no API key)");
        }

        Self {
            config,
            ollama,
            fireworks,
            deepinfra,
            fal,
        }
    }

    /// Resolve which backend to use for a given model name.
    ///
    /// Returns `(provider, backend_model_name)` or an error if no backend
    /// is available for the requested provider.
    fn resolve<'a>(&self, model: &'a str) -> Result<(ProviderId, &'a str), InferenceError> {
        let (provider, stripped_model) =
            ProviderId::parse_from_model(model).unwrap_or((self.config.default_provider, model));

        let available = match provider {
            ProviderId::Ollama => self.ollama.is_some(),
            ProviderId::Fireworks => self.fireworks.is_some(),
            ProviderId::DeepInfra => self.deepinfra.is_some(),
            ProviderId::Fal => self.fal.is_some(),
        };

        if !available {
            return Err(InferenceError::Connection(format!(
                "Provider {} is not available (check configuration)",
                provider.as_str()
            )));
        }

        Ok((provider, stripped_model))
    }

    /// List all available models across all configured providers.
    ///
    /// Queries each backend concurrently and merges results with
    /// provider prefixes applied. Graceful degradation: if one
    /// provider fails, results from others are still returned.
    pub async fn list_models(&self) -> Vec<RouterModelEntry> {
        let mut entries = Vec::new();

        // Ollama models
        if let Some(ref backend) = self.ollama
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry {
                    prefixed_name: ProviderId::Ollama.prefix_model(&m.name),
                    provider: ProviderId::Ollama,
                    model: m.name.clone(),
                    supports_vision: RouterModelEntry::infer_vision_support(
                        &m.name,
                        m.details.as_ref().and_then(|d| d.family.as_deref()),
                    ),
                    family: m.details.as_ref().and_then(|d| d.family.clone()),
                    parameter_size: m.details.as_ref().and_then(|d| d.parameter_size.clone()),
                    quantization_level: m
                        .details
                        .as_ref()
                        .and_then(|d| d.quantization_level.clone()),
                    size_bytes: m.size,
                });
            }
        }

        // Fireworks models
        if let Some(ref backend) = self.fireworks
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry {
                    prefixed_name: ProviderId::Fireworks.prefix_model(&m.id),
                    provider: ProviderId::Fireworks,
                    model: m.id.clone(),
                    supports_vision: RouterModelEntry::infer_vision_support(&m.id, None),
                    family: None,
                    parameter_size: None,
                    quantization_level: None,
                    size_bytes: None,
                });
            }
        }

        // DeepInfra models
        if let Some(ref backend) = self.deepinfra
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry {
                    prefixed_name: ProviderId::DeepInfra.prefix_model(&m.id),
                    provider: ProviderId::DeepInfra,
                    model: m.id.clone(),
                    supports_vision: RouterModelEntry::infer_vision_support(&m.id, None),
                    family: None,
                    parameter_size: None,
                    quantization_level: None,
                    size_bytes: None,
                });
            }
        }

        // fal.ai models (static catalog)
        if let Some(ref backend) = self.fal
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry {
                    prefixed_name: ProviderId::Fal.prefix_model(&m.id),
                    provider: ProviderId::Fal,
                    model: m.id.clone(),
                    supports_vision: RouterModelEntry::infer_vision_support(&m.id, None),
                    family: None,
                    parameter_size: None,
                    quantization_level: None,
                    size_bytes: None,
                });
            }
        }

        entries
    }

    /// Search models by name across all providers (case-insensitive substring).
    pub async fn search_models(&self, query: &str) -> Vec<RouterModelEntry> {
        let all = self.list_models().await;
        if query.is_empty() {
            return all;
        }
        let lower = query.to_lowercase();
        all.into_iter()
            .filter(|m| m.model.to_lowercase().contains(&lower))
            .collect()
    }

    /// List only models that are likely vision-capable.
    ///
    /// Convenience filter over `list_models()` using the heuristic
    /// `supports_vision` flag. Useful for OCR model selection.
    pub async fn list_vision_models(&self) -> Vec<RouterModelEntry> {
        self.list_models()
            .await
            .into_iter()
            .filter(|m| m.supports_vision == Some(true))
            .collect()
    }

    /// Vision/multimodal inference — dispatch to the appropriate backend with base64 images.
    pub async fn generate_vision(
        &self,
        prompt: &str,
        images: &[String],
        params: &LLMParameters,
        model_override: Option<&str>,
    ) -> Result<InferenceResult, InferenceError> {
        let model_name = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.config.default_model.clone());
        let (provider, model) = self.resolve(&model_name)?;
        let model = model.to_string();
        let prompt = prompt.to_string();
        let params = params.clone();
        let images = images.to_vec();

        match provider {
            ProviderId::Ollama => {
                self.ollama
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Ollama backend unavailable".to_string())
                    })?
                    .generate_vision(&model, &prompt, &images, &params)
                    .await
            }
            ProviderId::Fireworks => {
                self.fireworks
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Fireworks backend unavailable".to_string())
                    })?
                    .generate_vision(&model, &prompt, &images, &params)
                    .await
            }
            ProviderId::DeepInfra => {
                self.deepinfra
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("DeepInfra backend unavailable".to_string())
                    })?
                    .generate_vision(&model, &prompt, &images, &params)
                    .await
            }
            ProviderId::Fal => {
                self.fal
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("fal.ai backend unavailable".to_string())
                    })?
                    .generate_vision(&model, &prompt, &images, &params)
                    .await
            }
        }
    }
}

impl InferencePort for InferenceRouter {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let (provider, model) = match self.resolve(&self.config.default_model) {
            Ok(r) => r,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let prompt = prompt.to_string();
        let parameters = parameters.clone();

        Box::pin(async move {
            validate_prompt(&prompt)?;
            match provider {
                ProviderId::Ollama => {
                    self.ollama
                        .as_ref()
                        .unwrap()
                        .generate(model, &prompt, &parameters)
                        .await
                }
                ProviderId::Fireworks => {
                    self.fireworks
                        .as_ref()
                        .unwrap()
                        .generate(model, &prompt, &parameters)
                        .await
                }
                ProviderId::DeepInfra => {
                    self.deepinfra
                        .as_ref()
                        .unwrap()
                        .generate(model, &prompt, &parameters)
                        .await
                }
                ProviderId::Fal => {
                    self.fal
                        .as_ref()
                        .unwrap()
                        .generate(model, &prompt, &parameters)
                        .await
                }
            }
        })
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let model_name = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.config.default_model.clone());
        let (provider, model) = match self.resolve(&model_name) {
            Ok(r) => r,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let model = model.to_string();
        let prompt = prompt.to_string();
        let parameters = parameters.clone();

        Box::pin(async move {
            validate_prompt(&prompt)?;
            match provider {
                ProviderId::Ollama => {
                    self.ollama
                        .as_ref()
                        .unwrap()
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::Fireworks => {
                    self.fireworks
                        .as_ref()
                        .unwrap()
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::DeepInfra => {
                    self.deepinfra
                        .as_ref()
                        .unwrap()
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::Fal => {
                    self.fal
                        .as_ref()
                        .unwrap()
                        .generate(&model, &prompt, &parameters)
                        .await
                }
            }
        })
    }

    fn generate_stream(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        self.generate_stream_with_model(prompt, parameters, None)
    }

    fn generate_stream_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        let model_name = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.config.default_model.clone());
        let (provider, model) = match self.resolve(&model_name) {
            Ok(r) => r,
            Err(e) => {
                return Box::pin(futures_util::stream::once(async move { Err(e) }));
            }
        };
        let model = model.to_string();
        let prompt = prompt.to_string();
        let parameters = parameters.clone();

        match provider {
            ProviderId::Ollama => {
                self.ollama
                    .as_ref()
                    .unwrap()
                    .generate_stream(&model, &prompt, &parameters)
            }
            ProviderId::Fireworks => {
                self.fireworks
                    .as_ref()
                    .unwrap()
                    .generate_stream(&model, &prompt, &parameters)
            }
            ProviderId::DeepInfra => {
                self.deepinfra
                    .as_ref()
                    .unwrap()
                    .generate_stream(&model, &prompt, &parameters)
            }
            ProviderId::Fal => {
                self.fal
                    .as_ref()
                    .unwrap()
                    .generate_stream(&model, &prompt, &parameters)
            }
        }
    }

    fn generate_vision(
        &self,
        prompt: &str,
        images: &[String],
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let prompt = prompt.to_string();
        let images = images.to_vec();
        let parameters = parameters.clone();
        let model_override = model_override.map(|s| s.to_string());
        Box::pin(async move {
            self.generate_vision(&prompt, &images, &parameters, model_override.as_deref())
                .await
        })
    }
}
