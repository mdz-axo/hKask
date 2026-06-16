//! DeepInfra backend — cloud inference via OpenAI-compatible API.
//!
//! DeepInfra exposes `/v1/chat/completions` and `/v1/models` at
//! `https://api.deepinfra.com`. Requires Bearer token
//! authentication via `DI_API_KEY`.
//!
//! DeepInfra has the broadest open-source model catalog and the
//! lowest per-token pricing among GPU cloud providers.

use crate::chat_protocol::{
    build_chat_request, chat_response_to_result, parse_sse_stream, validate_prompt,
};
use crate::config::InferenceConfig;
use futures_util::StreamExt;
use hkask_types::ports::{InferenceError, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// DeepInfra backend for chat completions and model listing.
pub struct DeepInfraBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl DeepInfraBackend {
    /// Create a new DeepInfra backend from inference config.
    ///
    /// Returns an error if `deepinfra_api_key` is empty.
    ///
    /// REQ: INFER-010
    /// pre:  config.deepinfra_api_key is set
    /// post: returns DeepInfraBackend with configured HTTP client
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.deepinfra_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "DeepInfra API key not configured (set DI_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(InferenceError::Connection)?;
        Ok(Self {
            base_url: config.deepinfra_base_url.clone(),
            api_key: config.deepinfra_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to DeepInfra.
    ///
    /// REQ: INFER-033
    /// pre:  model is a valid DeepInfra model name
    /// pre:  prompt is non-empty (validated by validate_prompt)
    /// pre:  params is a valid LLMParameters
    /// post: returns Ok(InferenceResult) with generated text, model, usage stats
    /// post: if connection fails → Err(InferenceError::Connection)
    /// post: if prompt is empty → Err(InferenceError::Generation)
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        let request = build_chat_request(model, prompt, None, params, Some(false), Some(5));

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "DeepInfra status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("DeepInfra JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "DI",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "DeepInfra inference completed"
        );
        Ok(result)
    }

    /// Vision/multimodal inference with base64-encoded images.
    ///
    /// REQ: INFER-034
    /// pre:  model is a valid DeepInfra vision-capable model name
    /// pre:  prompt is non-empty
    /// pre:  images is non-empty (at least one base64-encoded image)
    /// pre:  params is a valid LLMParameters
    /// post: returns Ok(InferenceResult) with vision-generated text
    /// post: if images is empty → Err(InferenceError::Generation("No images provided"))
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn generate_vision(
        &self,
        model: &str,
        prompt: &str,
        images: &[String],
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        if images.is_empty() {
            return Err(InferenceError::Generation("No images provided".into()));
        }
        let request = build_chat_request(
            model,
            prompt,
            Some(images.to_vec()),
            params,
            Some(false),
            Some(5),
        );

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "DeepInfra vision status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("DeepInfra JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "hkask.inference",
            provider = "DI",
            model = %result.model,
            tokens = result.usage.total_tokens,
            "DeepInfra vision inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from DeepInfra via SSE.
    /// Generate a streaming completion from DeepInfra.
    ///
    /// REQ: INFER-011
    /// pre:  model is a valid DeepInfra model name
    /// post: returns stream of inference chunks
    pub fn generate_stream(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        let model = model.to_string();
        let prompt = prompt.to_string();
        let params = params.clone();
        let client = Arc::clone(&self.client);
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        Box::pin(
            futures_util::stream::once(async move {
                let request = build_chat_request(&model, &prompt, None, &params, Some(true), None);

                let response = match client
                    .post(format!("{}/v1/chat/completions", base_url))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| InferenceError::Connection(e.to_string()))
                {
                    Ok(r) => r,
                    Err(e) => return vec![Err(e)],
                };

                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return vec![Err(InferenceError::Connection(format!(
                        "DeepInfra streaming status {}: {}",
                        status, error_text
                    )))];
                }

                let body = match response
                    .text()
                    .await
                    .map_err(|e| InferenceError::Connection(e.to_string()))
                {
                    Ok(b) => b,
                    Err(e) => return vec![Err(e)],
                };

                parse_sse_stream(&body, &model)
            })
            .map(futures_util::stream::iter)
            .flatten(),
        )
    }

    /// List models from DeepInfra via `/v1/models`, filtered to last 6 months.
    ///
    /// REQ: INFER-035
    /// pre:  self.client and self.base_url are initialized
    /// post: returns Ok(Vec<DeepInfraModelEntry>) with models updated in last 180 days
    /// post: if API returns non-success → Ok(Vec::new()) (graceful degradation)
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn list_models(&self) -> Result<Vec<DeepInfraModelEntry>, InferenceError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(Vec::new()); // Graceful degradation
        }

        let list: DeepInfraModelList = response
            .json()
            .await
            .map_err(|_| InferenceError::Json("DeepInfra models parse error".into()))?;

        // Filter to models updated in the last 6 months
        let cutoff = chrono::Utc::now() - chrono::Duration::days(180);
        let filtered: Vec<DeepInfraModelEntry> = list
            .data
            .into_iter()
            .filter(|m| {
                m.created_at
                    .as_ref()
                    .and_then(|ts| {
                        chrono::DateTime::parse_from_rfc3339(ts)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    })
                    .map(|dt| dt >= cutoff)
                    .unwrap_or(false)
            })
            .collect();

        Ok(filtered)
    }

    // ── Media generation methods ───────────────────────────────────────────

    /// Call a DeepInfra inference endpoint for image generation.
    /// DeepInfra image models use POST /v1/inference/{model} with custom bodies.
    async fn di_inference_post(
        &self,
        model: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, InferenceError> {
        let url = format!("{}/v1/inference/{}", self.base_url, model);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(format!("DeepInfra request failed: {}", e)))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(InferenceError::Connection(format!(
                "DeepInfra {} status {}: {}",
                model, status, text
            )));
        }
        serde_json::from_str(&text)
            .map_err(|e| InferenceError::Json(format!("DeepInfra JSON parse: {}", e)))
    }

    /// Remove background from an image using Bria RMBG 2.0.
    /// Model: Bria/remove_background — $0.018/image, commercial-ready.
    ///
    /// REQ: INFER-036
    /// pre:  image_url is a valid, accessible image URL
    /// post: returns Ok(serde_json::Value) with background-removed image data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn remove_background(
        &self,
        image_url: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({"image_url": image_url});
        self.di_inference_post("Bria/remove_background", body).await
    }

    /// Generate an image from a text prompt using FLUX 2 Klein.
    /// Model: black-forest-labs/FLUX-2-klein-4b — fast 4B param FLUX.
    ///
    /// REQ: INFER-037
    /// pre:  prompt is a non-empty text description
    /// post: returns Ok(serde_json::Value) with generated image data (1024x1024)
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn generate_image(
        &self,
        prompt: &str,
        _image_size: Option<&str>,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({
            "prompt": prompt,
            "width": 1024,
            "height": 1024,
        });
        self.di_inference_post("black-forest-labs/FLUX-2-klein-4b", body)
            .await
    }

    /// Edit/transform an image using Qwen Image Edit.
    /// Model: Qwen/Qwen-Image-Edit — style transfer, precise edits.
    ///
    /// REQ: INFER-038
    /// pre:  image_url is a valid, accessible image URL
    /// pre:  prompt is a non-empty edit instruction
    /// post: returns Ok(serde_json::Value) with edited image data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn image_to_image(
        &self,
        image_url: &str,
        prompt: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({
            "image_url": image_url,
            "prompt": prompt,
        });
        self.di_inference_post("Qwen/Qwen-Image-Edit", body).await
    }

    /// Generate speech from text with a voice description.
    /// Uses DeepInfra's ElevenLabs-compatible TTS API.
    /// Default model: hexgrad/Kokoro-82M.
    /// API: POST /v1/text-to-speech/{voice_id}
    ///
    /// REQ: INFER-039
    /// pre:  text is non-empty
    /// pre:  voice_id is a valid voice identifier
    /// post: returns Ok(serde_json::Value) with base64-encoded MP3 audio
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn generate_speech(
        &self,
        text: &str,
        voice_id: &str,
        model_id: Option<&str>,
    ) -> Result<serde_json::Value, InferenceError> {
        let model = model_id.unwrap_or("hexgrad/Kokoro-82M");
        let url = format!("{}/v1/text-to-speech/{}", self.base_url, voice_id);
        let body = serde_json::json!({
            "text": text,
            "model_id": model,
            "output_format": "mp3",
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(format!("DeepInfra TTS failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "DeepInfra TTS status {}: {}",
                status, error_text
            )));
        }

        // TTS returns raw audio bytes — wrap in a JSON response with metadata
        let audio_bytes = resp
            .bytes()
            .await
            .map_err(|e| InferenceError::Connection(format!("DeepInfra TTS read failed: {}", e)))?;

        // Return as base64 data URI for portability
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
        Ok(serde_json::json!({
            "audio": format!("data:audio/mp3;base64,{}", b64),
            "format": "mp3",
            "model": model,
            "voice_id": voice_id,
        }))
    }

    /// Transcribe speech audio to text using Whisper.
    /// Uses DeepInfra's OpenAI-compatible audio transcription endpoint.
    /// API: POST /v1/audio/transcriptions
    /// Requests word-level timestamps for interactive transcript bundles.
    ///
    /// REQ: INFER-040
    /// pre:  audio_url is a valid, accessible audio file URL
    /// post: returns Ok(serde_json::Value) with verbose_json transcription (word+segment timestamps)
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn transcribe(
        &self,
        audio_url: &str,
        language: Option<&str>,
    ) -> Result<serde_json::Value, InferenceError> {
        let url = format!("{}/v1/audio/transcriptions", self.base_url);
        let mut body = serde_json::json!({
            "file": audio_url,
            "model": "openai/whisper-large-v3",
            "response_format": "verbose_json",
            "timestamp_granularities": ["word", "segment"],
        });
        if let Some(lang) = language {
            body["language"] = serde_json::json!(lang);
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(format!("DeepInfra STT failed: {}", e)))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(InferenceError::Connection(format!(
                "DeepInfra STT status {}: {}",
                status, text
            )));
        }

        serde_json::from_str(&text)
            .map_err(|e| InferenceError::Json(format!("DeepInfra STT parse: {}", e)))
    }
}

// ── DeepInfra model types ────────────────────────────────────────────────────

/// A model entry from DeepInfra's `/v1/models` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepInfraModelEntry {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

/// OpenAI-compatible model list response.
#[derive(Debug, Deserialize)]
struct DeepInfraModelList {
    data: Vec<DeepInfraModelEntry>,
}
