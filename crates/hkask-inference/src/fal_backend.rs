//! fal.ai backend — cloud inference via OpenAI-compatible API.
//!
//! fal.ai exposes `/v1/chat/completions` for text and vision models.
//! Requires Bearer token authentication via `FA_API_KEY`.
//!
//! Model listing: fal.ai does not expose a standard `/v1/models` endpoint.
//! Instead, a static catalog of known vision-capable models is used.

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

/// fal.ai backend for chat completions and vision inference.
#[derive(Debug)]
pub struct FalBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl FalBackend {
    /// Create a new Fal backend from inference config.
    ///
    /// Returns an error if `fal_api_key` is empty.
    ///
    /// REQ: P4-inf-fal-backend-new
    /// \[P4\] Motivating: Clear Boundaries — fal.ai provider membrane requires valid API key
    /// pre:  config.fal_api_key is set
    /// post: returns FalBackend with configured HTTP client
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.fal_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "fal.ai API key not configured (set FA_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(InferenceError::Connection)?;
        Ok(Self {
            base_url: config.fal_base_url.clone(),
            api_key: config.fal_api_key.clone(),
            client,
        })
    }

    /// Send a chat completion request to fal.ai.
    ///
    /// REQ: P9-inf-fal-generate
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated text generation
    /// pre:  model is a valid fal.ai model name
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
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "fal.ai status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("fal.ai JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "cns.inference",
            provider = "FA",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "fal.ai inference completed"
        );
        Ok(result)
    }

    /// Vision/multimodal inference with base64-encoded images.
    ///
    /// REQ: P9-inf-fal-generate-vision
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated multimodal generation
    /// pre:  model is a valid fal.ai vision-capable model name
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
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "fal.ai vision status {}: {}",
                status, error_text
            )));
        }

        let chat_response = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("fal.ai JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "cns.inference",
            provider = "FA",
            model = %result.model,
            tokens = result.usage.total_tokens,
            "fal.ai vision inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from fal.ai via SSE.
    /// Generate a streaming completion from Fal.
    ///
    /// REQ: P9-inf-fal-generate-stream
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated streaming text generation
    /// pre:  model is a valid Fal model name
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
                    .header("Authorization", format!("Key {}", api_key))
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
                        "fal.ai streaming status {}: {}",
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

    /// List known fal.ai models from the static catalog.
    ///
    /// fal.ai does not expose a standard `/v1/models` endpoint.
    /// Returns a curated list of vision-capable models known to work
    /// with the OpenAI-compatible chat completions endpoint.
    ///
    /// REQ: P9-inf-fal-list-models
    /// \[P9\] Motivating: Homeostatic Self-Regulation — static model catalog for variety
    /// pre:  none (static catalog, no API call)
    /// post: returns Ok(Vec<FalModelEntry>) with curated model list
    pub async fn list_models(&self) -> Result<Vec<FalModelEntry>, InferenceError> {
        // Static catalog of known fal.ai vision models.
        // These are models confirmed to work via the chat completions endpoint.
        Ok(vec![
            FalModelEntry {
                id: "paddleocr".into(),
                description: Some("PaddleOCR — document OCR model".into()),
            },
            FalModelEntry {
                id: "nemotron-parse".into(),
                description: Some("Nemotron Parse — document parsing and OCR".into()),
            },
            FalModelEntry {
                id: "docres".into(),
                description: Some(
                    "DocRes — document enhancement: deshadow, deblur, binarize, dewarp".into(),
                ),
            },
        ])
    }

    // ── Media generation methods ───────────────────────────────────────────

    /// Call a fal.ai sync endpoint (https://fal.run/{endpoint}).
    async fn fal_sync_post(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, InferenceError> {
        let url = format!("https://fal.run/{}", endpoint);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| InferenceError::Connection(format!("fal.ai request failed: {}", e)))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(InferenceError::Connection(format!(
                "fal.ai {} status {}: {}",
                endpoint, status, text
            )));
        }
        serde_json::from_str(&text)
            .map_err(|e| InferenceError::Json(format!("fal.ai JSON parse: {}", e)))
    }

    /// Call a fal.ai queue endpoint (https://queue.fal.run/{endpoint}) with polling.
    async fn fal_queue_post(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, InferenceError> {
        let submit_url = format!("https://queue.fal.run/{}", endpoint);
        let resp = self
            .client
            .post(&submit_url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("fal.ai queue submit failed: {}", e))
            })?;

        let status = resp.status();
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("fal.ai queue parse: {}", e)))?;

        if !status.is_success() {
            return Err(InferenceError::Connection(format!(
                "fal.ai queue {} status {}: {}",
                endpoint, status, v
            )));
        }

        let request_id = v
            .get("request_id")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown")
            .to_string();

        let status_url = format!(
            "https://queue.fal.run/{}/requests/{}/status",
            endpoint, request_id
        );
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(InferenceError::Connection(format!(
                    "fal.ai queue poll timed out after 120s (request_id: {})",
                    request_id
                )));
            }
            match self
                .client
                .get(&status_url)
                .header("Authorization", format!("Key {}", self.api_key))
                .send()
                .await
            {
                Ok(resp) => {
                    let v: serde_json::Value = resp
                        .json()
                        .await
                        .map_err(|e| InferenceError::Json(format!("fal.ai status parse: {}", e)))?;
                    match v.get("status").and_then(|s| s.as_str()) {
                        Some("COMPLETED") => break,
                        Some("FAILED") => {
                            return Err(InferenceError::Generation(format!(
                                "fal.ai job failed: {}",
                                v
                            )));
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(InferenceError::Connection(format!(
                        "fal.ai status check failed: {}",
                        e
                    )));
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        let result_url = format!("https://queue.fal.run/{}/requests/{}", endpoint, request_id);
        let resp = self
            .client
            .get(&result_url)
            .header("Authorization", format!("Key {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("fal.ai result fetch failed: {}", e))
            })?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(InferenceError::Connection(format!(
                "fal.ai result {} status {}: {}",
                endpoint, status, text
            )));
        }
        serde_json::from_str(&text)
            .map_err(|e| InferenceError::Json(format!("fal.ai result parse: {}", e)))
    }

    /// Generate an image from a text prompt.
    /// Endpoint: fal-ai/flux/schnell (fast) or fal-ai/flux-pro (quality).
    ///
    /// REQ: P9-inf-fal-generate-image
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image generation
    /// pre:  prompt is a non-empty text description
    /// post: returns Ok(serde_json::Value) with generated image data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn generate_image(
        &self,
        prompt: &str,
        image_size: Option<&str>,
        num_images: Option<u32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size.unwrap_or("1024x1024"),
            "num_images": num_images.unwrap_or(1),
        });
        self.fal_sync_post("fal-ai/flux/schnell", body).await
    }

    /// Transform an existing image with a prompt (image-to-image).
    /// Endpoint: fal-ai/flux/dev/image-to-image
    ///
    /// REQ: P9-inf-fal-image-to-image
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image editing
    /// pre:  image_url is a valid, accessible image URL
    /// pre:  prompt is a non-empty transformation instruction
    /// post: returns Ok(serde_json::Value) with transformed image data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn image_to_image(
        &self,
        image_url: &str,
        prompt: &str,
        strength: Option<f32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let mut body = serde_json::json!({
            "prompt": prompt,
            "image_url": image_url,
        });
        if let Some(s) = strength {
            body["strength"] = serde_json::json!(s);
        }
        self.fal_sync_post("fal-ai/flux/dev/image-to-image", body)
            .await
    }

    /// Remove background from an image.
    /// Endpoint: fal-ai/birefnet
    ///
    /// REQ: P9-inf-fal-remove-background
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image transformation
    /// pre:  image_url is a valid, accessible image URL
    /// post: returns Ok(serde_json::Value) with background-removed image data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn remove_background(
        &self,
        image_url: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({"image_url": image_url});
        self.fal_sync_post("fal-ai/birefnet", body).await
    }

    /// Upscale an image.
    /// Endpoint: fal-ai/seedvr2 (queue)
    ///
    /// REQ: P9-inf-fal-upscale
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image upscaling
    /// pre:  image_url is a valid, accessible image URL
    /// post: returns Ok(serde_json::Value) with upscaled image data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn upscale(
        &self,
        image_url: &str,
        scale: Option<u32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({
            "image_url": image_url,
            "scale": scale.unwrap_or(4),
        });
        self.fal_queue_post("fal-ai/seedvr2", body).await
    }

    /// Generate a video from a text prompt.
    /// Endpoint: fal-ai/minimax/video-01-live (queue)
    ///
    /// REQ: P9-inf-fal-generate-video
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated video generation
    /// pre:  prompt is a non-empty text description
    /// post: returns Ok(serde_json::Value) with generated video data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn generate_video(
        &self,
        prompt: &str,
        duration: Option<f32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let mut body = serde_json::json!({"prompt": prompt});
        if let Some(d) = duration {
            body["duration"] = serde_json::json!(d);
        }
        self.fal_queue_post("fal-ai/minimax/video-01-live", body)
            .await
    }

    /// Animate a still image into a video.
    /// Endpoint: fal-ai/seedance-2.0/image-to-video (queue)
    ///
    /// REQ: P9-inf-fal-image-to-video
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated video generation
    /// pre:  image_url is a valid, accessible image URL
    /// post: returns Ok(serde_json::Value) with generated video data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn image_to_video(
        &self,
        image_url: &str,
        prompt: Option<&str>,
        duration: Option<f32>,
    ) -> Result<serde_json::Value, InferenceError> {
        let mut body = serde_json::json!({"image_url": image_url});
        if let Some(p) = prompt {
            body["prompt"] = serde_json::json!(p);
        }
        if let Some(d) = duration {
            body["duration"] = serde_json::json!(d);
        }
        self.fal_queue_post("fal-ai/seedance-2.0/image-to-video", body)
            .await
    }

    /// Segment/extract a specific object from an image.
    /// Endpoint: fal-ai/florence-2-large/referring-expression-segmentation
    ///
    /// REQ: P9-inf-fal-segment-object
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated image segmentation
    /// pre:  image_url is a valid, accessible image URL
    /// pre:  object_description is a non-empty description of the object to segment
    /// post: returns Ok(serde_json::Value) with segmented object data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn segment_object(
        &self,
        image_url: &str,
        object_description: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({
            "image_url": image_url,
            "prompt": object_description,
        });
        self.fal_sync_post(
            "fal-ai/florence-2-large/referring-expression-segmentation",
            body,
        )
        .await
    }

    /// Generate speech from text with a voice preset.
    /// Uses fal.ai ElevenLabs TTS (eleven-v3).
    /// Available voices: Rachel, Aria, Roger, Sarah, Laura, Charlie, George,
    /// Callum, River, Liam, Charlotte, Alice, Matilda, Will, Jessica, Eric,
    /// Chris, Brian, Daniel, Lily, Bill. Default: "Rachel".
    ///
    /// REQ: P9-inf-fal-generate-speech
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated speech synthesis
    /// pre:  text is non-empty
    /// pre:  voice is a valid voice preset name
    /// post: returns Ok(serde_json::Value) with generated speech audio data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn generate_speech(
        &self,
        text: &str,
        voice: &str,
    ) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({
            "text": text,
            "voice": voice,
        });
        self.fal_sync_post("fal-ai/elevenlabs/tts/eleven-v3", body)
            .await
    }

    /// Transcribe speech audio to text using Whisper.
    /// Endpoint: fal-ai/whisper
    ///
    /// REQ: P9-inf-fal-transcribe
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated speech transcription
    /// pre:  audio_url is a valid, accessible audio file URL
    /// post: returns Ok(serde_json::Value) with transcription data
    /// post: if API call fails → Err(InferenceError::Connection)
    pub async fn transcribe(&self, audio_url: &str) -> Result<serde_json::Value, InferenceError> {
        let body = serde_json::json!({"audio_url": audio_url});
        self.fal_sync_post("fal-ai/whisper", body).await
    }
}

// ── fal.ai model types ──────────────────────────────────────────────────────

/// A model entry from fal.ai's static catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalModelEntry {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ: P9-inf-test-fal-backend-new-fails — Construction fails without API key
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates boundary enforcement without key
    #[test]
    fn construction_fails_without_api_key() {
        let config = InferenceConfig::default();
        assert!(config.fal_api_key.is_empty());
        let result = FalBackend::new(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("FA_API_KEY"),
            "error should mention FA_API_KEY, got: {}",
            err
        );
    }

    /// REQ: P9-inf-test-fal-backend-new-succeeds — Construction succeeds with API key
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates boundary construction with key
    #[test]
    fn construction_succeeds_with_api_key() {
        let config = InferenceConfig {
            fal_api_key: "test-key-123".into(),
            ..Default::default()
        };
        let result = FalBackend::new(&config);
        assert!(
            result.is_ok(),
            "should succeed with API key: {:?}",
            result.err()
        );
    }

    /// REQ: P9-inf-test-fal-static-catalog — Static catalog returns known vision models
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates model variety catalog
    #[tokio::test]
    async fn static_catalog_returns_vision_models() {
        let config = InferenceConfig {
            fal_api_key: "test-key".into(),
            ..Default::default()
        };
        let backend = FalBackend::new(&config).unwrap();
        let models = backend.list_models().await.unwrap();
        assert!(!models.is_empty(), "catalog should not be empty");
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(
            ids.contains(&"paddleocr"),
            "catalog should include paddleocr"
        );
        assert!(
            ids.contains(&"nemotron-parse"),
            "catalog should include nemotron-parse"
        );
        assert!(ids.contains(&"docres"), "catalog should include docres");
    }

    /// REQ: P9-inf-test-fal-vision-support — Vision support heuristic recognizes fal.ai models
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates vision model heuristic
    #[test]
    fn vision_support_heuristic_recognizes_fal_models() {
        use crate::RouterModelEntry;
        assert_eq!(
            RouterModelEntry::infer_vision_support("paddleocr", None),
            Some(true)
        );
        assert_eq!(
            RouterModelEntry::infer_vision_support("nemotron-parse", None),
            Some(true)
        );
        assert_eq!(
            RouterModelEntry::infer_vision_support("FA/paddleocr", None),
            Some(true)
        );
    }
}
