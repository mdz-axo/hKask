//! Inference router — multi-provider `InferencePort` implementation.
//!
//! Routes requests to DeepInfra, fal.ai, or Together AI based on the
//! 2-letter provider prefix in the model name. Unprefixed model names
//! use the configured default provider.

use crate::RouterModelEntry;
use crate::chat_protocol::validate_prompt;
use crate::config::{InferenceConfig, ProviderId};
use crate::deepinfra_backend::DeepInfraBackend;
use crate::embedding_router::EmbeddingRouter;
use crate::fal_backend::FalBackend;
use crate::openrouter_backend::OpenRouterBackend;
use crate::together_backend::TogetherBackend;
use hkask_ports::{InferenceError, InferencePort, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use std::pin::Pin;
use std::sync::Arc;
use tracing::warn;

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
    embedding: EmbeddingRouter,
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
            embedding,
        }
    }

    /// Compute the effective model name, applying the fusion override when active.
    ///
    /// When `config.fusion_model` is Some AND `params.bypass_fusion` is false,
    /// the fusion model is used regardless of the explicit or default model.
    /// Otherwise, falls back to the explicit model or config.default_model.
    fn effective_model(&self, explicit: Option<&str>, params: &LLMParameters) -> String {
        if !params.bypass_fusion
            && let Some(ref fusion) = self.config.fusion_model
        {
            return fusion.clone();
        }
        explicit.unwrap_or(&self.config.default_model).to_string()
    }

    /// Resolve which backend to use for a given model name.
    ///
    /// Returns `(provider, backend_model_name)` or an error if no backend
    /// is available for the requested provider.
    fn resolve<'a>(&self, model: &'a str) -> Result<(ProviderId, &'a str), InferenceError> {
        let (provider, stripped_model) =
            ProviderId::parse_from_model(model).unwrap_or((self.config.default_provider, model));

        let available = match provider {
            ProviderId::DeepInfra => self.deepinfra.is_some(),
            ProviderId::Fal => self.fal.is_some(),
            ProviderId::Together => self.together.is_some(),
            ProviderId::OpenRouter => self.openrouter.is_some(),
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

    /// Dispatch a generate call to the resolved backend.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — shared dispatch for text generation
    /// pre:  provider is a resolved ProviderId with available backend
    /// pre:  model, prompt, params are validated and cloned
    /// post: returns Ok(InferenceResult) on success
    /// post: returns Err(Connection) if backend is None or provider is unsupported
    async fn dispatch_generate(
        &self,
        provider: ProviderId,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        match provider {
            ProviderId::DeepInfra => {
                self.deepinfra
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("DeepInfra backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params)
                    .await
            }
            ProviderId::Fal => {
                self.fal
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("fal.ai backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params)
                    .await
            }
            ProviderId::Together => {
                self.together
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Together backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params)
                    .await
            }
            ProviderId::OpenRouter => {
                self.openrouter
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("OpenRouter backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params)
                    .await
            }
            ProviderId::Runpod | ProviderId::Baseten => Err(InferenceError::Connection(
                "Runpod/Baseten are adapter providers".to_string(),
            )),
        }
    }

    /// List all available models across all configured providers.
    ///
    /// Queries each backend concurrently and merges results with
    /// provider prefixes applied. Graceful degradation: if one
    /// provider fails, results from others are still returned.
    ///
    /// expect: "I can discover available models across providers"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — aggregated model variety across providers
    /// pre:  backends are initialized (may be None)
    /// post: returns `Vec<RouterModelEntry>` with all available models across providers
    /// post: if a backend fails → its models are omitted (graceful degradation)
    pub async fn list_models(&self) -> Vec<RouterModelEntry> {
        let mut entries = Vec::new();

        // DeepInfra models
        if let Some(ref backend) = self.deepinfra
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry::from_model_entry(
                    ProviderId::DeepInfra,
                    &m.id,
                ));
            }
        }

        if let Some(ref backend) = self.fal
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry::from_model_entry(ProviderId::Fal, &m.id));
            }
        }

        if let Some(ref backend) = self.together
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry::from_model_entry(
                    ProviderId::Together,
                    &m.id,
                ));
            }
        }

        // OpenRouter models
        if let Some(ref backend) = self.openrouter
            && let Ok(models) = backend.list_models().await
        {
            for m in models {
                entries.push(RouterModelEntry::from_model_entry(
                    ProviderId::OpenRouter,
                    &m.id,
                ));
            }
        }

        entries
    }

    /// Search models by name across all providers (case-insensitive substring).
    ///
    /// expect: "I can discover available models across providers"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — searchable model catalog for routing
    /// pre:  query may be empty (returns all models)
    /// post: returns `Vec<RouterModelEntry>` filtered by case-insensitive substring match
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
    /// expect: "I can discover available models across providers"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — vision-capable model discovery
    /// pre:  none (delegates to list_models)
    /// post: returns `Vec<RouterModelEntry>` filtered to supports_vision == Some(true)
    pub async fn list_vision_models(&self) -> Vec<RouterModelEntry> {
        self.list_models()
            .await
            .into_iter()
            .filter(|m| m.supports_vision == Some(true))
            .collect()
    }

    /// Vision/multimodal inference — dispatch to the appropriate backend with base64 images.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated multimodal dispatch
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
            ProviderId::OpenRouter => {
                self.openrouter
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("OpenRouter backend unavailable".to_string())
                    })?
                    .generate_vision(&model, &prompt, &images, &params)
                    .await
            }
            ProviderId::Runpod | ProviderId::Baseten => Err(InferenceError::Connection(
                "Runpod/Baseten are adapter-composition providers; use AdapterRouter".to_string(),
            )),
        }
    }

    // ── Media generation dispatch ──────────────────────────────────────────

    /// Generate an image from a text prompt.
    /// Routes to fal.ai FLUX Schnell (default) or DeepInfra FLUX 2 Klein.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image generation dispatch
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image editing dispatch
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated background removal dispatch
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
                    tracing::warn!(target: "cns.inference", error = %e, "DeepInfra background removal failed, falling back to fal.ai");
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated upscaling dispatch
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated video generation dispatch
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated video generation dispatch
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated speech synthesis dispatch
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
                    tracing::warn!(target: "cns.inference", error = %e, "DeepInfra TTS failed, falling back to fal.ai");
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated segmentation dispatch
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
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated transcription dispatch
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
                    tracing::warn!(target: "cns.inference", error = %e, "DeepInfra STT failed, falling back to fal.ai");
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
    // pre:  prompt is non-empty; parameters are valid
    // post: Ok(InferenceResult) when resolved provider backend is configured;
    //       Err(Connection) when resolved provider backend is None
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
                self.dispatch_generate(provider, &model, &prompt, &parameters)
                    .await
            });
        }

        let model_name = self.effective_model(None, parameters);
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
                ProviderId::DeepInfra => {
                    self.deepinfra
                        .as_ref()
                        .ok_or_else(|| {
                            InferenceError::Connection("DeepInfra backend unavailable".to_string())
                        })?
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::Fal => {
                    self.fal
                        .as_ref()
                        .ok_or_else(|| {
                            InferenceError::Connection("fal.ai backend unavailable".to_string())
                        })?
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::Together => {
                    self.together
                        .as_ref()
                        .ok_or_else(|| {
                            InferenceError::Connection("Together backend unavailable".to_string())
                        })?
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::OpenRouter => {
                    self.openrouter
                        .as_ref()
                        .ok_or_else(|| {
                            InferenceError::Connection("OpenRouter backend unavailable".to_string())
                        })?
                        .generate(&model, &prompt, &parameters)
                        .await
                }
                ProviderId::Runpod | ProviderId::Baseten => Err(InferenceError::Connection(
                    "Runpod/Baseten are adapter providers".to_string(),
                )),
            }
        })
    }

    // pre:  prompt is non-empty; parameters are valid; model_override may be None
    // post: Ok(InferenceResult) when resolved provider backend is configured;
    //       Err(Connection) when resolved provider backend is None
    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let model_name = self.effective_model(model_override, parameters);
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
            self.dispatch_generate(provider, &model, &prompt, &parameters)
                .await
        })
    }

    // pre:  prompt is non-empty; parameters are valid
    // post: Stream of Ok(InferenceStreamChunk) when resolved provider backend is configured;
    //       Stream of Err(Connection) when resolved provider backend is None
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

    // pre:  prompt is non-empty; parameters are valid; model_override may be None
    // post: Stream of Ok(InferenceStreamChunk) when resolved provider backend is configured;
    //       Stream of Err(Connection) when resolved provider backend is None
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
        let model_name = self.effective_model(model_override, parameters);
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
            ProviderId::DeepInfra => {
                match self.deepinfra.as_ref().ok_or_else(|| {
                    InferenceError::Connection("DeepInfra backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &parameters),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Fal => {
                match self.fal.as_ref().ok_or_else(|| {
                    InferenceError::Connection("fal.ai backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &parameters),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Together => {
                match self.together.as_ref().ok_or_else(|| {
                    InferenceError::Connection("Together backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &parameters),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::OpenRouter => {
                match self.openrouter.as_ref().ok_or_else(|| {
                    InferenceError::Connection("OpenRouter backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &parameters),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Runpod | ProviderId::Baseten => {
                Box::pin(futures_util::stream::once(async move {
                    Err(InferenceError::Connection(
                        "Runpod/Baseten are adapter providers".to_string(),
                    ))
                }))
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
    /// Generate a text embedding vector via the embedding router.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated embedding dispatch
    /// pre:  text is a non-empty string
    /// post: delegates to EmbeddingRouter::embed_sentence with resolved model
    /// post: if embedding fails → Err(EmbeddingGenerationError)
    pub async fn embed_text(
        &self,
        text: &str,
        model_override: Option<&str>,
    ) -> Result<Vec<f32>, hkask_ports::EmbeddingGenerationError> {
        let model = model_override.unwrap_or(&self.config.default_model);
        self.embedding.embed_sentence(model, text).await
    }
}
