//! Inference router — multi-provider `InferencePort` implementation.
//!
//! Routes requests to Ollama, Fireworks, DeepInfra, or fal.ai based on the
//! 2-letter provider prefix in the model name. Unprefixed model names
//! use the configured default provider.

use crate::RouterModelEntry;
use crate::chat_protocol::validate_prompt;
use crate::config::{InferenceConfig, ProviderId};
use crate::deepinfra_backend::DeepInfraBackend;
use crate::embedding_router::EmbeddingRouter;
use crate::fal_backend::FalBackend;
use crate::ollama_backend::OllamaBackend;
use crate::together_backend::TogetherBackend;
use hkask_types::ports::{InferenceError, InferencePort, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
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
    deepinfra: Option<DeepInfraBackend>,
    fal: Option<FalBackend>,
    together: Option<TogetherBackend>,
    #[allow(dead_code)]
    embedding: Option<EmbeddingRouter>,
}

impl InferenceRouter {
    /// Build the router from an `InferenceConfig`.
    ///
    /// Create a new inference router from config.
    ///
    /// Constructs backends lazily — a backend is only created if its
    /// configuration is valid (e.g., API key is present for cloud providers).
    /// Ollama is always attempted since it requires no auth.
    ///
    /// REQ: INFER-019
    /// pre:  config is a valid InferenceConfig
    /// post: returns InferenceRouter with backends for configured providers
    pub fn new(config: InferenceConfig) -> Self {
        let ollama = OllamaBackend::new(&config).ok();
        let deepinfra = DeepInfraBackend::new(&config).ok();
        let fal = FalBackend::new(&config).ok();
        let together = TogetherBackend::new(&config).ok();

        if ollama.is_none() {
            warn!(target: "hkask.inference", "Ollama backend unavailable");
        }
        if deepinfra.is_none() {
            warn!(target: "hkask.inference", "DeepInfra backend unavailable (no API key)");
        }
        if fal.is_none() {
            warn!(target: "hkask.inference", "fal.ai backend unavailable (no API key)");
        }
        if together.is_none() {
            warn!(target: "hkask.inference", "Together AI backend unavailable (no API key)");
        }

        Self {
            config,
            ollama,
            deepinfra,
            fal,
            together,
            embedding: None,
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
            ProviderId::DeepInfra => self.deepinfra.is_some(),
            ProviderId::Fal => self.fal.is_some(),
            ProviderId::Together => self.together.is_some(),
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
    ///
    /// REQ: INFER-059
    /// pre:  backends are initialized (may be None)
    /// post: returns Vec<RouterModelEntry> with all available models across providers
    /// post: if a backend fails → its models are omitted (graceful degradation)
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

        // Together AI models
        if let Some(ref backend) = self.together
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry {
                    prefixed_name: ProviderId::Together.prefix_model(&m.id),
                    provider: ProviderId::Together,
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
    ///
    /// REQ: INFER-060
    /// pre:  query may be empty (returns all models)
    /// post: returns Vec<RouterModelEntry> filtered by case-insensitive substring match
    /// post: if query is empty → returns all models (delegates to list_models)
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
    ///
    /// REQ: INFER-061
    /// pre:  none (delegates to list_models)
    /// post: returns Vec<RouterModelEntry> filtered to supports_vision == Some(true)
    pub async fn list_vision_models(&self) -> Vec<RouterModelEntry> {
        self.list_models()
            .await
            .into_iter()
            .filter(|m| m.supports_vision == Some(true))
            .collect()
    }

    /// Vision/multimodal inference — dispatch to the appropriate backend with base64 images.
    ///
    /// REQ: INFER-062
    /// pre:  prompt is non-empty
    /// pre:  images is non-empty
    /// pre:  params is a valid LLMParameters
    /// post: dispatches to provider-resolved backend's generate_vision
    /// post: returns Ok(InferenceResult) on success
    /// post: if provider resolution fails → Err(InferenceError)
    /// post: if backend call fails → Err(InferenceError)
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
            ProviderId::Together => {
                self.together
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Together AI backend unavailable".to_string())
                    })?
                    .generate_vision(&model, &prompt, &images, &params)
                    .await
            }
        }
    }

    // ── Media generation dispatch ──────────────────────────────────────────

    /// Generate an image from a text prompt.
    /// Routes to fal.ai FLUX Schnell (default) or DeepInfra FLUX 2 Klein.
    ///
    /// REQ: INFER-063
    /// pre:  prompt is a non-empty text description
    /// post: returns Ok(serde_json::Value) with generated image data
    /// post: if fal backend unavailable → Err(InferenceError::Connection)
    pub async fn generate_image(
        &self,
        prompt: &str,
        image_size: Option<&str>,
        num_images: Option<u32>,
    ) -> Result<serde_json::Value, InferenceError> {
        // Default to fal.ai for image generation
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection(
                "fal.ai backend unavailable for image generation".to_string(),
            )
        })?;
        backend.generate_image(prompt, image_size, num_images).await
    }

    /// Transform an existing image with a prompt (image-to-image).
    /// Routes to fal.ai Flux dev img2img (default) or DeepInfra Qwen Image Edit.
    ///
    /// REQ: INFER-064
    /// pre:  image_url is a valid, accessible image URL
    /// pre:  prompt is a non-empty transformation instruction
    /// post: returns Ok(serde_json::Value) with transformed image data
    /// post: if fal backend unavailable → Err(InferenceError::Connection)
    pub async fn image_to_image(
        &self,
        image_url: &str,
        prompt: &str,
        strength: Option<f32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection("fal.ai backend unavailable for image-to-image".to_string())
        })?;
        backend.image_to_image(image_url, prompt, strength).await
    }

    /// Remove background from an image.
    /// Routes to DeepInfra Bria RMBG 2.0 (cheapest) with fal.ai Birefnet fallback.
    ///
    /// REQ: INFER-065
    /// pre:  image_url is a valid, accessible image URL
    /// post: tries DeepInfra first, falls back to fal.ai on failure
    /// post: returns Ok(serde_json::Value) with background-removed image data
    /// post: if no backend available → Err(InferenceError::Connection)
    pub async fn remove_background(
        &self,
        image_url: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        // Try DeepInfra first (cheapest at $0.018/image)
        if let Some(ref di) = self.deepinfra {
            match di.remove_background(image_url).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(target: "hkask.inference", error = %e, "DeepInfra background removal failed, falling back to fal.ai");
                }
            }
        }
        // Fallback to fal.ai Birefnet
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection("No backend available for background removal".to_string())
        })?;
        backend.remove_background(image_url).await
    }

    /// Upscale an image.
    /// Routes to fal.ai SeedVR2 (queue).
    ///
    /// REQ: INFER-066
    /// pre:  image_url is a valid, accessible image URL
    /// post: returns Ok(serde_json::Value) with upscaled image data
    /// post: if fal backend unavailable → Err(InferenceError::Connection)
    pub async fn upscale(
        &self,
        image_url: &str,
        scale: Option<u32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection("fal.ai backend unavailable for upscaling".to_string())
        })?;
        backend.upscale(image_url, scale).await
    }

    /// Generate a video from a text prompt.
    /// Routes to fal.ai MiniMax video-01-live (queue).
    ///
    /// REQ: INFER-067
    /// pre:  prompt is a non-empty text description
    /// post: returns Ok(serde_json::Value) with generated video data
    /// post: if fal backend unavailable → Err(InferenceError::Connection)
    pub async fn generate_video(
        &self,
        prompt: &str,
        duration: Option<f32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection(
                "fal.ai backend unavailable for video generation".to_string(),
            )
        })?;
        backend.generate_video(prompt, duration).await
    }

    /// Animate a still image into a video.
    /// Routes to fal.ai Seedance 2.0 image-to-video (queue).
    ///
    /// REQ: INFER-068
    /// pre:  image_url is a valid, accessible image URL
    /// post: returns Ok(serde_json::Value) with generated video data
    /// post: if fal backend unavailable → Err(InferenceError::Connection)
    pub async fn image_to_video(
        &self,
        image_url: &str,
        prompt: Option<&str>,
        duration: Option<f32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection("fal.ai backend unavailable for image-to-video".to_string())
        })?;
        backend.image_to_video(image_url, prompt, duration).await
    }

    /// Generate speech from text with a voice preset.
    /// Routes to DeepInfra ElevenLabs-compatible API (default) with fal.ai fallback.
    /// Default voice: "Rachel" (ElevenLabs default, available on both providers).
    /// Default model on DeepInfra: hexgrad/Kokoro-82M.
    ///
    /// REQ: INFER-069
    /// pre:  text is non-empty
    /// pre:  voice is a valid voice preset name
    /// post: tries DeepInfra first, falls back to fal.ai on failure
    /// post: returns Ok(serde_json::Value) with generated speech audio data
    /// post: if no backend available → Err(InferenceError::Connection)
    pub async fn generate_speech(
        &self,
        text: &str,
        voice: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        // Try DeepInfra first (ElevenLabs-compatible API)
        if let Some(ref di) = self.deepinfra {
            match di.generate_speech(text, voice, None).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(target: "hkask.inference", error = %e, "DeepInfra TTS failed, falling back to fal.ai");
                }
            }
        }
        // Fallback to fal.ai ElevenLabs
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection("No backend available for speech generation".to_string())
        })?;
        backend.generate_speech(text, voice).await
    }

    /// Segment/extract a specific object from an image.
    /// Routes to fal.ai Florence-2 segmentation.
    ///
    /// REQ: INFER-070
    /// pre:  image_url is a valid, accessible image URL
    /// pre:  object_description is a non-empty description of the object to segment
    /// post: returns Ok(serde_json::Value) with segmented object data
    /// post: if fal backend unavailable → Err(InferenceError::Connection)
    pub async fn segment_object(
        &self,
        image_url: &str,
        object_description: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection(
                "fal.ai backend required for object segmentation".to_string(),
            )
        })?;
        backend.segment_object(image_url, object_description).await
    }

    /// Transcribe speech audio to text.
    /// Routes to DeepInfra Whisper (default) with fal.ai Whisper fallback.
    ///
    /// REQ: INFER-071
    /// pre:  audio_url is a valid, accessible audio file URL
    /// post: tries DeepInfra first, falls back to fal.ai on failure
    /// post: returns Ok(serde_json::Value) with transcription data
    /// post: if no backend available → Err(InferenceError::Connection)
    pub async fn transcribe(
        &self,
        audio_url: &str,
        language: Option<&str>,
    ) -> Result<serde_json::Value, InferenceError> {
        // Try DeepInfra first
        if let Some(ref di) = self.deepinfra {
            match di.transcribe(audio_url, language).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(target: "hkask.inference", error = %e, "DeepInfra STT failed, falling back to fal.ai");
                }
            }
        }
        // Fallback to fal.ai Whisper
        let backend = self.fal.as_ref().ok_or_else(|| {
            InferenceError::Connection("No backend available for speech transcription".to_string())
        })?;
        backend.transcribe(audio_url).await
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
        // LoRA adapter overrides the model entirely (includes base model).
        // Format: "Qwen3.5-9B#constraint-forces-v3" — adapter IS the full model identifier.
        if let Some(ref adapter) = parameters.adapter {
            let (provider, model) = match self.resolve(adapter) {
                Ok(r) => r,
                Err(e) => return Box::pin(async move { Err(e) }),
            };
            let model = model.to_string();
            let prompt = prompt.to_string();
            let parameters = parameters.clone();
            return Box::pin(async move {
                validate_prompt(&prompt)?;
                match provider {
                    ProviderId::Ollama => {
                        self.ollama
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
                    ProviderId::Together => {
                        self.together
                            .as_ref()
                            .unwrap()
                            .generate(&model, &prompt, &parameters)
                            .await
                    }
                }
            });
        }

        let (provider, model) = match self.resolve(&self.config.default_model) {
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
                ProviderId::Together => {
                    self.together
                        .as_ref()
                        .unwrap()
                        .generate(&model, &prompt, &parameters)
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
        // LoRA adapter overrides the model entirely (includes base model).
        // When adapter is set, it replaces model_override/default_model completely.
        let effective_model = parameters.adapter.as_deref().unwrap_or(&model_name);
        let (provider, model) = match self.resolve(effective_model) {
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
                ProviderId::Together => {
                    self.together
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
            ProviderId::Together => {
                self.together
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

// Non-trait methods (not part of InferencePort)
impl InferenceRouter {
    /// Generate a text embedding vector — not yet implemented.
    ///
    /// REQ: INFER-072
    /// pre:  _text may be any string (currently ignored)
    /// post: always returns Err(EmbeddingGenerationError::Connection) — not yet implemented
    pub async fn embed_text(
        &self,
        _text: &str,
        _model_override: Option<&str>,
    ) -> Result<Vec<f32>, hkask_types::ports::EmbeddingGenerationError> {
        Err(hkask_types::ports::EmbeddingGenerationError::Connection(
            "Embedding router not yet implemented".into(),
        ))
    }
}
