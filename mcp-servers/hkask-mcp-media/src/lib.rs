//! hKask MCP Media — AI media generation (image, video, voice via centralized inference router)
//!
//! Tool families:
//! - Gallery: organize, search, status
//! - Image: describe, remove_background, apply_style, create_collage
//! - Video: clip, to_gif, image_to_video, add_caption, remix, concat, from_images
//! - Generation: generate_image, transform_image, upscale_image, generate_video
//! - Voice: voice_design, generate_speech
//! - Audio: transcribe, transcribe_bundle, audio_capture, record_and_transcribe

// Pre-existing clippy lints from original bin-only codebase (addressed in separate refactoring pass).
#![allow(clippy::collapsible_if, clippy::cloned_ref_to_slice_refs)]

pub mod omc;

mod gallery;
mod templates;
mod video;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use gallery::GalleryState;
use gallery::vision::{self};
use hkask_agents::VoiceDesign;
use hkask_inference::InferenceRouter;
use hkask_mcp::DaemonClient;
use hkask_mcp::server::{McpToolError, execute_tool, validate_tool_url};
use hkask_ports::InferencePort;
use hkask_storage::{GalleryMode, GalleryStore, GalleryStoreError, Store};

use hkask_types::{TimedWord, TranscriptBundle, TranscriptSegment, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
pub mod tools;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use video::FfmpegRunner;

use ab_glyph::Font;
use face_id::analyzer::FaceAnalyzer;

// ── Model configuration ───────────────────────────────────────────────

/// Default open-weight models for media processing.
/// All can be overridden via environment variables.
pub mod models {
    /// Default TTS model via DeepInfra
    pub const TTS_DEFAULT: &str = "Qwen3-TTS";
    pub const TTS_ENV: &str = "HKASK_MEDIA_TTS_MODEL";

    /// Default STT model: Qwen3 ASR via DeepInfra
    pub const STT_DEFAULT: &str = "Qwen/Qwen3-ASR";
    pub const STT_ENV: &str = "HKASK_MEDIA_STT_MODEL";

    /// Default vision model: Qwen 2.5 VL
    pub const VISION_DEFAULT: &str = "Qwen/Qwen3-VL-235B-A22B-Instruct";
    pub const VISION_ENV: &str = "HKASK_MEDIA_VISION_MODEL";

    /// Default image generation model
    pub const IMAGE_GEN_DEFAULT: &str = "fal-ai/flux-pro/v1.1";
    pub const IMAGE_GEN_ENV: &str = "HKASK_MEDIA_IMAGE_GEN_MODEL";

    /// Resolve a model name from env var or default.
    pub fn resolve(env_key: &str, default: &str) -> String {
        std::env::var(env_key).unwrap_or_else(|_| default.to_string())
    }

    pub fn tts_model() -> String {
        resolve(TTS_ENV, TTS_DEFAULT)
    }
    pub fn stt_model() -> String {
        resolve(STT_ENV, STT_DEFAULT)
    }
    pub fn vision_model() -> String {
        resolve(VISION_ENV, VISION_DEFAULT)
    }
    pub fn image_gen_model() -> String {
        resolve(IMAGE_GEN_ENV, IMAGE_GEN_DEFAULT)
    }
}

/// Lock-free snapshot of gallery state — safe to hold across .await points.
struct GalleryAccess {
    gallery_id: String,
    image_count: u64,
    root_path: PathBuf,
}

pub struct MediaServer {
    pub webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<DaemonClient>,
    /// Centralized inference router for ALL model calls (vision LLM + media generation)
    pub inference: Arc<InferenceRouter>,
    /// Active gallery state (None until gallery_set_root is called)
    pub gallery_state: Arc<Mutex<Option<GalleryState>>>,
    /// SQLite-backed gallery store for persistent indexing
    pub gallery_store: Arc<GalleryStore>,
    /// Jinja2 template environment for prompt rendering
    pub template_env: minijinja::Environment<'static>,
    /// ffmpeg runner for video processing (None if ffmpeg not found)
    pub ffmpeg: FfmpegRunner,
    /// ONNX face detection + recognition pipeline (None if model download failed)
    pub face_analyzer: Option<Arc<FaceAnalyzer>>,
}

pub mod types;
use types::*;

/// Compute normalized Levenshtein similarity between two strings.
/// Returns 1.0 for identical strings, 0.0 for completely different.
fn levenshtein_similarity(a: &str, b: &str) -> f64 {
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    if a_len == 0 && b_len == 0 {
        return 1.0;
    }
    if a_len == 0 || b_len == 0 {
        return 0.0;
    }

    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_chars: Vec<char> = a_lower.chars().collect();
    let b_chars: Vec<char> = b_lower.chars().collect();

    // Space-optimized DP: only keep two rows
    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    let distance = prev[b_len];
    let max_len = a_len.max(b_len) as f64;
    1.0 - (distance as f64 / max_len)
}

#[cfg(test)]
mod levenshtein_tests {
    use super::*;

    #[test]
    fn identical_strings() {
        assert!((levenshtein_similarity("sunset", "sunset") - 1.0).abs() < 0.001);
    }

    #[test]
    fn completely_different() {
        let sim = levenshtein_similarity("sunset", "xyzzy");
        assert!(sim < 0.3, "expected low similarity, got {}", sim);
    }

    #[test]
    fn case_insensitive() {
        assert!((levenshtein_similarity("Sunset", "sunset") - 1.0).abs() < 0.001);
    }

    #[test]
    fn typo_tolerant() {
        let sim = levenshtein_similarity("sunset", "sunest");
        assert!(sim > 0.6, "expected high similarity for typo, got {}", sim);
    }

    #[test]
    fn empty_strings() {
        assert!((levenshtein_similarity("", "") - 1.0).abs() < 0.001);
        assert!((levenshtein_similarity("sunset", "") - 0.0).abs() < 0.001);
        assert!((levenshtein_similarity("", "sunset") - 0.0).abs() < 0.001);
    }
}

impl MediaServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        inference: Arc<InferenceRouter>,
        gallery_store: Arc<GalleryStore>,
        face_analyzer: Option<Arc<FaceAnalyzer>>,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            webid,
            replicant,
            daemon,
            inference,
            gallery_state: Arc::new(Mutex::new(None)),
            gallery_store,
            template_env: templates::create_env(),
            ffmpeg: FfmpegRunner::detect(),
            face_analyzer,
        })
    }

    // ── ToolContext implementation for framework-level execute_tool ──────

    /// Record a tool outcome to semantic memory (fire-and-forget via daemon).
    /// Called automatically by `execute_tool()`.
    fn record_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }

    /// Lock the gallery and extract essential state. Drops the lock before
    /// returning, so the result is safe to hold across .await points.
    fn access_gallery(&self) -> Result<GalleryAccess, String> {
        let guard = self
            .gallery_state
            .lock()
            .map_err(|e| format!("Gallery state lock error: {}", e))?;
        let state = guard
            .as_ref()
            .ok_or_else(|| "No gallery organized. Use gallery_organize first.".to_string())?;
        let access = GalleryAccess {
            gallery_id: state
                .gallery_id
                .clone()
                .ok_or_else(|| "Gallery not persisted — run gallery_organize first.".to_string())?,
            image_count: state.image_count,
            root_path: state.path.clone(),
        };
        Ok(access)
    }

    /// Return the ffmpeg runner or an error if ffmpeg is not installed.
    fn require_ffmpeg(&self) -> Result<&FfmpegRunner, McpToolError> {
        if self.ffmpeg.available {
            Ok(&self.ffmpeg)
        } else {
            Err(McpToolError::unavailable(
                "ffmpeg not found on system PATH — video tools unavailable.",
            ))
        }
    }

    /// Return the best available vision model or an error if none is configured.
    async fn require_vision(&self) -> Result<(&'static str, &'static str), McpToolError> {
        self.resolve_vision_model().await.ok_or_else(|| {
            McpToolError::unavailable(
                "No vision-capable provider configured (set DEEPINFRA_API_KEY, OPENROUTER_API_KEY, or TOGETHER_API_KEY)",
            )
        })
    }

    /// Render a Jinja2 prompt template with the given variables.
    fn render_prompt(&self, name: &str, vars: &HashMap<&str, &str>) -> Result<String, String> {
        templates::render(&self.template_env, name, vars)
    }

    /// Resolve an image index to a base64 data URL for vision LLM calls.
    fn resolve_image_url(&self, image_index: usize) -> Result<String, String> {
        let ga = self.access_gallery()?;

        let img = self
            .gallery_store
            .get_image(&ga.gallery_id, Some(image_index), None)
            .map_err(|e| format!("Image not found at index {}: {}", image_index, e))?;

        let data = std::fs::read(&img.absolute_path)
            .map_err(|e| format!("Failed to read image: {}", e))?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
        let mime = match img.format.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "tiff" => "image/tiff",
            _ => "image/png",
        };
        Ok(format!("data:{};base64,{}", mime, b64))
    }

    /// Resolve an image index to a filesystem path.
    fn resolve_image_path(&self, image_index: usize) -> Result<PathBuf, String> {
        let ga = self.access_gallery()?;

        let img = self
            .gallery_store
            .get_image(&ga.gallery_id, Some(image_index), None)
            .map_err(|e| format!("Image not found at index {}: {}", image_index, e))?;

        Ok(PathBuf::from(&img.absolute_path))
    }

    /// Resolve an image index to its SQLite image ID for tag persistence.
    fn resolve_image_id(&self, image_index: usize) -> Result<String, String> {
        let ga = self.access_gallery()?;

        let img = self
            .gallery_store
            .get_image(&ga.gallery_id, Some(image_index), None)
            .map_err(|e| format!("Image not found at index {}: {}", image_index, e))?;

        Ok(img.id)
    }

    /// Resolve an image ID directly to a base64 data URL.
    ///
    /// Used by face matching where we have image IDs from tags/registry,
    /// not gallery indices.
    fn resolve_image_url_by_id(&self, image_id: &str) -> Result<String, String> {
        let ga = self.access_gallery()?;

        // Look up the image's absolute path by its SQLite ID
        let conn = self
            .gallery_store
            .lock_conn()
            .map_err(|e| format!("Failed to lock store: {}", e))?;
        let absolute_path: String = conn
            .query_row(
                "SELECT absolute_path FROM gallery_images WHERE id = ?1 AND gallery_id = ?2",
                [image_id, ga.gallery_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| format!("Image not found by ID {}: {}", image_id, e))?;
        drop(conn);

        let data =
            std::fs::read(&absolute_path).map_err(|e| format!("Failed to read image: {}", e))?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
        let mime = if absolute_path.ends_with(".png") {
            "image/png"
        } else if absolute_path.ends_with(".jpg") || absolute_path.ends_with(".jpeg") {
            "image/jpeg"
        } else if absolute_path.ends_with(".webp") {
            "image/webp"
        } else if absolute_path.ends_with(".gif") {
            "image/gif"
        } else {
            "image/png"
        };
        Ok(format!("data:{};base64,{}", mime, b64))
    }

    /// Persist a single tag to the gallery store (best-effort, logs errors).
    fn persist_tag(
        &self,
        image_id: &str,
        tag_type: &str,
        value: &str,
        confidence: f64,
        model: &str,
    ) {
        match self
            .gallery_store
            .tag_image(image_id, tag_type, value, confidence, model)
        {
            Ok(_) => {
                tracing::debug!(target: "cns.mcp.media.tags", image_id = %image_id, tag_type = %tag_type, value = %value, "Tag persisted")
            }
            Err(e) => {
                tracing::warn!(target: "cns.mcp.media.tags", image_id = %image_id, tag_type = %tag_type, error = %e, "Failed to persist tag")
            }
        }
    }

    /// Crop a face region from an image using bounding box percentages.
    ///
    /// Returns a base64 data URL of the cropped face region, or the original
    /// image URL if cropping fails (graceful degradation).
    fn crop_face_region(&self, image_id: &str, bbox: &serde_json::Value) -> Result<String, String> {
        let ga = self.access_gallery()?;

        let conn = self
            .gallery_store
            .lock_conn()
            .map_err(|e| format!("Failed to lock store: {}", e))?;
        let absolute_path: String = conn
            .query_row(
                "SELECT absolute_path FROM gallery_images WHERE id = ?1 AND gallery_id = ?2",
                [image_id, ga.gallery_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| format!("Image not found: {}", e))?;
        drop(conn);

        // Read and crop the image
        let img =
            image::open(&absolute_path).map_err(|e| format!("Failed to open image: {}", e))?;

        let x_pct = bbox["x_pct"].as_f64().unwrap_or(0.0);
        let y_pct = bbox["y_pct"].as_f64().unwrap_or(0.0);
        let w_pct = bbox["w_pct"].as_f64().unwrap_or(100.0);
        let h_pct = bbox["h_pct"].as_f64().unwrap_or(100.0);

        let (img_w, img_h) = (img.width(), img.height());
        let x = ((x_pct / 100.0) * img_w as f64).round() as u32;
        let y = ((y_pct / 100.0) * img_h as f64).round() as u32;
        let w = ((w_pct / 100.0) * img_w as f64).round() as u32;
        let h = ((h_pct / 100.0) * img_h as f64).round() as u32;

        // Clamp to image bounds
        let x = x.min(img_w.saturating_sub(1));
        let y = y.min(img_h.saturating_sub(1));
        let w = w.min(img_w - x).max(1);
        let h = h.min(img_h - y).max(1);

        let cropped = img.crop_imm(x, y, w, h);

        // Encode as base64 data URL
        let mut buf = std::io::Cursor::new(Vec::new());
        cropped
            .write_to(&mut buf, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to encode cropped image: {}", e))?;
        let data = buf.into_inner();
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
        Ok(format!("data:image/jpeg;base64,{}", b64))
    }

    /// Resolve the best available vision model with fallback chain.
    /// Tries: DeepInfra → OpenRouter → Together AI.
    /// Returns (model_name, label) or None if no vision provider is configured.
    async fn resolve_vision_model(&self) -> Option<(&'static str, &'static str)> {
        let models = self.inference.list_vision_models().await;

        for model in &models {
            match model.provider {
                hkask_inference::ProviderId::DeepInfra => {
                    return Some((
                        "DI/meta-llama/Llama-3.2-11B-Vision-Instruct",
                        "llama-3.2-vision",
                    ));
                }
                hkask_inference::ProviderId::OpenRouter => {
                    return Some(("OR/openai/gpt-4o", "gpt-4o-vision"));
                }
                hkask_inference::ProviderId::Together => {
                    return Some(("TG/Qwen/Qwen2.5-VL-72B-Instruct", "qwen-vl"));
                }
                _ => continue,
            }
        }

        None
    }

    /// Re-scan an existing gallery and persist new images.
    /// Returns (gallery_id, old_image_count, images_added, total_images, persisted_count).
    /// The MutexGuard is dropped before return so callers can safely await.
    fn rescan_existing_gallery(
        &self,
        recursive: bool,
    ) -> Result<(String, u64, u32, u32, u32), String> {
        // Hold the lock for the entire scan→persist operation to prevent lost-update
        // races under concurrent calls. All operations inside are synchronous I/O
        // (std::fs + GalleryStore), so holding std::sync::Mutex is safe.
        let mut guard = self
            .gallery_state
            .lock()
            .map_err(|e| format!("Gallery state lock error: {}", e))?;
        let state = guard
            .as_mut()
            .ok_or("No gallery organized. Use gallery_organize first.".to_string())?;

        let gallery_id = state
            .gallery_id
            .clone()
            .ok_or_else(|| "Gallery not persisted — run gallery_organize first.".to_string())?;
        let old_count = state.image_count;

        let scan_result = state.scan(recursive, None);
        let mut persisted = 0u32;
        for entry in &scan_result.entries {
            let abs_path = state.path.join(&entry.relative_path);
            if self
                .gallery_store
                .add_image(
                    &gallery_id,
                    &entry.relative_path,
                    &abs_path.to_string_lossy(),
                    &entry.checksum,
                    entry.width,
                    entry.height,
                    &entry.format,
                    entry.size_bytes,
                )
                .is_ok()
            {
                persisted += 1;
            }
        }

        Ok((
            gallery_id,
            old_count,
            scan_result.added,
            scan_result.total,
            persisted,
        ))
    }

    /// Run the analysis pipeline on a subset of gallery images.
    /// Used internally by gallery_organize auto_analyze and gallery_analyze.
    /// Returns (analyzed_count, error_messages).
    async fn run_analysis_on_indices(
        &self,
        indices: &[usize],
        pipelines: &[String],
    ) -> (u32, Vec<String>) {
        let (vision_model, vision_label) = match self.resolve_vision_model().await {
            Some(v) => v,
            None => return (0, vec!["No vision model available — configure a vision-capable provider (DeepInfra, OpenRouter, or Together AI)".to_string()]),
        };
        let mut analyzed = 0u32;
        let mut errors = Vec::new();

        let run_faces = pipelines.iter().any(|p| p == "faces");
        let run_objects = pipelines.iter().any(|p| p == "objects");
        let run_colors = pipelines.iter().any(|p| p == "colors");
        let run_composition = pipelines.iter().any(|p| p == "composition");
        let run_scene = pipelines.iter().any(|p| p == "scene");

        for idx in indices {
            let image_url = match self.resolve_image_url(*idx) {
                Ok(url) => url,
                Err(e) => {
                    errors.push(format!("image {}: {}", idx, e));
                    continue;
                }
            };
            let image_id = match self.resolve_image_id(*idx) {
                Ok(id) => id,
                Err(e) => {
                    errors.push(format!("image {}: {}", idx, e));
                    continue;
                }
            };

            if run_faces {
                match vision::detect_faces(
                    &self.inference,
                    &self.template_env,
                    &image_url,
                    Some(vision_model),
                )
                .await
                {
                    Ok(faces) => {
                        for face in &faces {
                            let value = serde_json::to_string(face).unwrap_or_default();
                            self.persist_tag(&image_id, "face", &value, 0.85, vision_label);
                        }
                    }
                    Err(e) => {
                        errors.push(format!("image {} face detection: {}", idx, e));
                    }
                }
            }

            if run_objects {
                match vision::detect_objects(
                    &self.inference,
                    &self.template_env,
                    &image_url,
                    Some(vision_model),
                )
                .await
                {
                    Ok(objects) => {
                        for obj in &objects {
                            let value = serde_json::to_string(obj).unwrap_or_default();
                            self.persist_tag(&image_id, "object", &value, 0.85, vision_label);
                        }
                    }
                    Err(e) => {
                        errors.push(format!("image {} object detection: {}", idx, e));
                    }
                }
            }

            if run_colors {
                match vision::analyze_colors(
                    &self.inference,
                    &self.template_env,
                    &image_url,
                    Some(vision_model),
                )
                .await
                {
                    Ok(parsed) => {
                        if let Some(colors) = parsed["colors"].as_array() {
                            for color in colors {
                                let value = serde_json::to_string(color).unwrap_or_default();
                                self.persist_tag(&image_id, "color", &value, 0.85, vision_label);
                            }
                        }
                        for field in &["palette_style", "temperature", "saturation"] {
                            if let Some(v) = parsed.get(*field).and_then(|v| v.as_str()) {
                                self.persist_tag(&image_id, "color", v, 0.9, vision_label);
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("image {} color analysis: {}", idx, e));
                    }
                }
            }

            if run_composition {
                match vision::analyze_composition(
                    &self.inference,
                    &self.template_env,
                    &image_url,
                    Some(vision_model),
                )
                .await
                {
                    Ok(parsed) => {
                        for field in &[
                            "focal_point",
                            "rule_of_thirds",
                            "leading_lines",
                            "depth_of_field",
                            "perspective",
                            "framing",
                            "symmetry",
                            "negative_space",
                        ] {
                            if let Some(v) = parsed.get(*field).and_then(|v| v.as_str()) {
                                self.persist_tag(&image_id, "composition", v, 0.85, vision_label);
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("image {} composition analysis: {}", idx, e));
                    }
                }
            }

            if run_scene {
                match vision::caption_scene(
                    &self.inference,
                    &self.template_env,
                    &image_url,
                    Some(vision_model),
                )
                .await
                {
                    Ok(caption) => {
                        self.persist_tag(&image_id, "caption", &caption, 0.9, vision_label);
                    }
                    Err(e) => {
                        errors.push(format!("image {} scene caption: {}", idx, e));
                    }
                }
            }

            analyzed += 1;
        }

        (analyzed, errors)
    }

    /// Run ONNX-based face detection + embedding extraction on gallery images.
    /// Returns (faces_found, embeddings_by_image, errors).
    /// Each embedding is paired with its image_id and bbox for cropping.
    async fn run_onnx_face_pipeline(
        &self,
        indices: &[usize],
    ) -> (u32, Vec<(String, Vec<u8>, serde_json::Value)>, Vec<String>) {
        let analyzer = match &self.face_analyzer {
            Some(a) => a,
            None => {
                return (
                    0,
                    Vec::new(),
                    vec!["ONNX analyzer not available".to_string()],
                );
            }
        };

        let mut faces_found = 0u32;
        let mut embeddings: Vec<(String, Vec<u8>, serde_json::Value)> = Vec::new();
        let mut errors = Vec::new();

        for idx in indices {
            let image_id = match self.resolve_image_id(*idx) {
                Ok(id) => id,
                Err(e) => {
                    errors.push(format!("image {}: {}", idx, e));
                    continue;
                }
            };
            let image_path = match self.resolve_image_path(*idx) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("image {}: {}", idx, e));
                    continue;
                }
            };

            let img = match image::open(&image_path) {
                Ok(i) => i,
                Err(e) => {
                    errors.push(format!("image {} open: {}", idx, e));
                    continue;
                }
            };

            match analyzer.analyze(&img) {
                Ok(faces) => {
                    for face in &faces {
                        let bbox = serde_json::json!({
                            "x_pct": (face.detection.bbox.x1 * 100.0).round(),
                            "y_pct": (face.detection.bbox.y1 * 100.0).round(),
                            "w_pct": ((face.detection.bbox.x2 - face.detection.bbox.x1) * 100.0).round(),
                            "h_pct": ((face.detection.bbox.y2 - face.detection.bbox.y1) * 100.0).round(),
                        });
                        let blob = embedding_to_blob(&face.embedding);
                        embeddings.push((image_id.clone(), blob, bbox));
                        faces_found += 1;
                    }
                }
                Err(e) => {
                    errors.push(format!("image {} analysis: {}", idx, e));
                }
            }
        }

        (faces_found, embeddings, errors)
    }

    /// Extract EXIF metadata from an image file.
    /// Returns key fields as a JSON object, or null if EXIF is unavailable.
    fn extract_exif(path: &str) -> serde_json::Value {
        let exif = match nom_exif::read_exif(path) {
            Ok(e) => e,
            Err(_) => return serde_json::Value::Null,
        };

        let mut fields = serde_json::Map::new();

        // Map common EXIF tag codes to human-readable names
        let tag_map: &[(u16, &str)] = &[
            (0x010F, "camera_make"),   // Make
            (0x0110, "camera_model"),  // Model
            (0x9003, "date_taken"),    // DateTimeOriginal
            (0x829A, "exposure_time"), // ExposureTime
            (0x829D, "f_number"),      // FNumber
            (0x8827, "iso"),           // ISOSpeedRatings
            (0x920A, "focal_length"),  // FocalLength
            (0x9209, "flash"),         // Flash
            (0x010E, "description"),   // ImageDescription
            (0x013B, "artist"),        // Artist
            (0x8298, "copyright"),     // Copyright
            (0x0131, "software"),      // Software
        ];

        for (code, name) in tag_map {
            if let Some(entry) = exif.get_by_code(nom_exif::IfdIndex::MAIN, *code) {
                if let Some(value_str) = entry.as_str() {
                    fields.insert(
                        name.to_string(),
                        serde_json::Value::String(value_str.to_string()),
                    );
                }
            }
        }

        // GPS info
        if let Some(gps) = exif.gps_info() {
            fields.insert(
                "gps".to_string(),
                serde_json::Value::String(gps.to_iso6709()),
            );
        }

        if fields.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::Object(fields)
        }
    }
}

impl hkask_mcp::server::ToolContext for MediaServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        self.record_outcome(tool, outcome);
    }
}

/// Load a font for meme text rendering. Tries the provided path first,
/// then common system paths, then returns an error with guidance.
fn load_meme_font(font_path: Option<&str>) -> Result<ab_glyph::FontVec, String> {
    if let Some(path) = font_path {
        // Reject path traversal attempts — font_path must be a simple filename
        if path.contains('/') || path.contains('\\') || path.contains("..") {
            return Err(format!(
                "font_path must be a simple filename, not a path: '{}'",
                path
            ));
        }
        let data =
            std::fs::read(path).map_err(|e| format!("Cannot read font at '{}': {}", path, e))?;
        return ab_glyph::FontVec::try_from_vec(data)
            .map_err(|e| format!("Invalid font file at '{}': {:?}", path, e));
    }

    // Try common system paths
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    ];

    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(font) = ab_glyph::FontVec::try_from_vec(data) {
                return Ok(font);
            }
        }
    }

    Err("No system font found".to_string())
}

/// Measure rendered text dimensions for centering.
fn measure_text(font: &ab_glyph::FontVec, scale: ab_glyph::PxScale, text: &str) -> (u32, u32) {
    let mut total_width = 0.0f32;
    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        total_width += font.h_advance_unscaled(glyph_id) * scale.x;
    }
    let height = (font.ascent_unscaled() * scale.y / font.height_unscaled()).ceil() as u32;
    (total_width.ceil() as u32, height)
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Convert a 512-dim f32 embedding to raw bytes for BLOB storage.
fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert raw BLOB bytes back to a 512-dim f32 embedding.
fn blob_to_embedding(blob: &[u8]) -> Option<Vec<f32>> {
    if !blob.len().is_multiple_of(4) {
        return None;
    }
    let count = blob.len() / 4;
    let mut vec = Vec::with_capacity(count);
    for chunk in blob.chunks_exact(4) {
        vec.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Some(vec)
}

// ── Combined tool router (P5 Essentialism — modular tool groups) ──────────

impl MediaServer {
    fn combined_router() -> rmcp::handler::server::router::tool::ToolRouter<Self> {
        Self::gallery_router()
            + Self::processing_router()
            + Self::audio_router()
            + Self::generation_router()
    }
}

#[rmcp::tool_handler(router = Self::combined_router())]
impl rmcp::ServerHandler for MediaServer {}

/// Run the media MCP server (used by binary target).
pub async fn run(
    replicant: String,
    _daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();

    // Build the inference router for vision LLM tasks.
    // Backends are constructed lazily — only those with configured API keys are available.
    let inference_config = hkask_inference::InferenceConfig::from_env();
    let inference = Arc::new(InferenceRouter::new(inference_config));

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "cns.mcp.media", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(DaemonClient::new())
    } else {
        None
    };

    // Create an in-memory GalleryStore for the media server.
    // Gracefully degrade if DB initialization fails — gallery tools
    // will return errors but the server stays alive (matching face_analyzer pattern).
    let gallery_store = {
        let db = hkask_storage::in_memory_db();
        let conn = db.conn_arc();
        match conn.lock() {
            Ok(conn) => match GalleryStore::init_tables(&conn) {
                Ok(()) => {
                    tracing::info!(target: "cns.mcp.media", "Gallery store initialized");
                    Arc::new(GalleryStore::new(db.conn_arc()))
                }
                Err(e) => {
                    tracing::warn!(target: "cns.mcp.media", error = %e, "Gallery table initialization failed — gallery tools will be unavailable");
                    Arc::new(GalleryStore::new(db.conn_arc()))
                }
            },
            Err(e) => {
                tracing::warn!(target: "cns.mcp.media", error = %e, "Gallery DB lock failed — gallery tools will be unavailable");
                Arc::new(GalleryStore::new(db.conn_arc()))
            }
        }
    };

    // Initialize ONNX face analyzer (downloads ~250MB models on first run)
    let face_analyzer = match FaceAnalyzer::from_hf().build().await {
        Ok(a) => {
            tracing::info!(target: "cns.mcp.media", "ONNX face analyzer ready");
            Some(Arc::new(a))
        }
        Err(e) => {
            tracing::warn!(target: "cns.mcp.media", error = %e, "ONNX face analyzer unavailable — face detection will use vision LLM fallback");
            None
        }
    };

    hkask_mcp::run_server(
        "hkask-mcp-media",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            Ok(MediaServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                inference.clone(),
                gallery_store.clone(),
                face_analyzer.clone(),
            )?)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "DI_API_KEY",
                "DeepInfra API key for vision LLMs and media generation",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "FA_API_KEY",
                "fal.ai API key for image/video generation",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "TOGETHER_API_KEY",
                "Together AI API key for vision LLMs",
            ),
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "media", &[]).await?;
    tracing::info!(target: "cns.mcp.media", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}

// ── Integration tests ────────────────────────────────────────────────────
//
// These tests exercise the GalleryStore + GalleryState pipeline and collage
// composition logic. Inference-dependent tools require a running LLM backend
// and are tested via the MCP protocol in live sessions.

#[cfg(test)]
mod integration_tests {
    use crate::gallery::GalleryState;
    use hkask_storage::gallery::{GalleryMode, GalleryStore};
    use image::{Rgb, RgbImage};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    fn setup_store() -> (Arc<GalleryStore>, TempDir) {
        let temp = TempDir::new().expect("tempdir");
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        let conn = Arc::new(Mutex::new(conn));
        GalleryStore::init_tables(&conn.lock().unwrap()).expect("init tables");
        (Arc::new(GalleryStore::new(conn)), temp)
    }

    fn create_test_image(dir: &std::path::Path, name: &str, r: u8, g: u8, b: u8) {
        let img: RgbImage = RgbImage::from_pixel(64, 64, Rgb([r, g, b]));
        img.save(dir.join(name)).expect("save test image");
    }

    #[test]
    fn gallery_lifecycle_init_to_search() {
        let (store, temp) = setup_store();

        create_test_image(temp.path(), "sunset.jpg", 255, 100, 50);
        create_test_image(temp.path(), "ocean.jpg", 50, 100, 255);
        create_test_image(temp.path(), "forest.png", 34, 139, 34);

        let gallery = store
            .create(
                &temp.path().to_string_lossy(),
                hkask_storage::GalleryMode::ReadOnly,
            )
            .expect("create gallery");
        assert_eq!(gallery.image_count, 0);

        let mut state = GalleryState::new(temp.path().to_path_buf(), GalleryMode::ReadOnly);
        let scan = state.scan(false, None);
        assert_eq!(scan.added, 3);

        for entry in &scan.entries {
            store
                .add_image(
                    &gallery.id,
                    &entry.relative_path,
                    &temp.path().join(&entry.relative_path).to_string_lossy(),
                    &entry.checksum,
                    entry.width,
                    entry.height,
                    &entry.format,
                    entry.size_bytes,
                )
                .expect("add image");
        }

        let img = store
            .get_image(&gallery.id, Some(0), None)
            .expect("get image");
        assert_eq!(img.width, 64);

        store
            .tag_image(&img.id, "object", "sunset", 0.95, "test")
            .expect("tag");

        let tags = store.get_tags(&img.id).expect("get tags");
        assert_eq!(tags.len(), 1);

        let all_tags = store.get_all_tags(&gallery.id).expect("get all tags");
        assert!(!all_tags.is_empty());
        assert!(all_tags.iter().any(|(t, _)| t.value == "sunset"));
    }

    #[test]
    fn collage_compose_grid_layout() {
        let temp = TempDir::new().expect("tempdir");

        let images: Vec<image::DynamicImage> = vec![
            image::DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([255u8, 0, 0]))),
            image::DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([0, 255u8, 0]))),
            image::DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([0, 0, 255u8]))),
            image::DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([255u8, 255u8, 0]))),
        ];

        let spacing: u32 = 8;
        let canvas_w: u32 = 800;
        let canvas_h: u32 = 600;
        let cols = (images.len() as f64).sqrt().ceil() as u32;
        let rows = (images.len() as u32).div_ceil(cols);
        assert_eq!(cols, 2);
        assert_eq!(rows, 2);

        let cell_w = (canvas_w - spacing * (cols + 1)) / cols;
        let cell_h = (canvas_h - spacing * (rows + 1)) / rows;

        let mut canvas = image::DynamicImage::new_rgba8(canvas_w, canvas_h);
        let bg = image::Rgba([30u8, 30u8, 30u8, 255u8]);
        for pixel in canvas.as_mut_rgba8().unwrap().pixels_mut() {
            *pixel = bg;
        }

        for (i, img) in images.iter().enumerate() {
            let col = i as u32 % cols;
            let row = i as u32 / cols;
            let scaled = img.resize_exact(
                cell_w.saturating_sub(spacing),
                cell_h.saturating_sub(spacing),
                image::imageops::FilterType::Lanczos3,
            );
            let x = spacing
                + col * (cell_w + spacing)
                + (cell_w.saturating_sub(spacing) - scaled.width()) / 2;
            let y = spacing
                + row * (cell_h + spacing)
                + (cell_h.saturating_sub(spacing) - scaled.height()) / 2;
            image::imageops::overlay(&mut canvas, &scaled, x as i64, y as i64);
        }

        let output_path = temp.path().join("collage_test.png");
        canvas.save(&output_path).expect("save collage");
        let collage = image::open(&output_path).expect("reopen");
        assert_eq!(collage.width(), 800);
        assert_eq!(collage.height(), 600);

        let non_bg = collage
            .to_rgba8()
            .pixels()
            .filter(|p| p.0 != [30, 30, 30, 255])
            .count();
        assert!(
            non_bg > 100,
            "collage should have non-bg pixels (got {})",
            non_bg
        );
    }

    #[test]
    fn gallery_store_image_not_found() {
        let (store, temp) = setup_store();
        let gallery = store
            .create(
                &temp.path().to_string_lossy(),
                hkask_storage::GalleryMode::ReadOnly,
            )
            .expect("create gallery");

        assert!(store.get_image(&gallery.id, Some(999), None).is_err());
        assert!(
            store
                .get_image(&gallery.id, None, Some("nonexistent"))
                .is_err()
        );
    }

    #[test]
    fn gallery_three_state_policy() {
        use hkask_storage::GalleryMode;
        assert_eq!(GalleryMode::ReadOnly.as_str(), "read-only");
        assert_eq!(GalleryMode::CopyOnWrite.as_str(), "copy-on-write");
        assert_eq!(GalleryMode::Destructive.as_str(), "destructive");
        assert_ne!(
            GalleryMode::ReadOnly.as_str(),
            GalleryMode::Destructive.as_str()
        );
    }

    // ── Face recognition tests ─────────────────────────────────────────────

    #[test]
    fn face_validation_deserialize_pass() {
        let json = r#"{
            "valid": true,
            "face_count": 1,
            "face_coverage_pct": 45,
            "pose": "frontal",
            "lighting": "good",
            "occlusion": "none",
            "clarity": "sharp",
            "issues": []
        }"#;
        let result: crate::gallery::vision::FaceValidationResult =
            serde_json::from_str(json).expect("deserialize");
        assert!(result.valid);
        assert_eq!(result.face_count, 1);
        assert_eq!(result.face_coverage_pct, 45);
        assert_eq!(result.pose, "frontal");
        assert!(result.issues.is_empty());
    }

    #[test]
    fn face_validation_deserialize_reject() {
        let json = r#"{
            "valid": false,
            "face_count": 2,
            "face_coverage_pct": 10,
            "pose": "profile",
            "lighting": "poor",
            "occlusion": "significant",
            "clarity": "blurry",
            "issues": [
                "Multiple faces detected (2) — reference must contain exactly 1 face",
                "Face coverage too low (10%) — minimum 15% required",
                "Profile pose — frontal or near-frontal required"
            ]
        }"#;
        let result: crate::gallery::vision::FaceValidationResult =
            serde_json::from_str(json).expect("deserialize");
        assert!(!result.valid);
        assert_eq!(result.face_count, 2);
        assert_eq!(result.issues.len(), 3);
        assert!(result.issues[0].contains("Multiple faces"));
    }

    #[test]
    fn face_match_deserialize_match() {
        let json = r#"{
            "match": true,
            "confidence": 0.94,
            "reasoning": "Same bone structure, identical eye spacing, matching nose shape."
        }"#;
        let result: crate::gallery::vision::FaceMatchResult =
            serde_json::from_str(json).expect("deserialize");
        assert!(result.is_match);
        assert!((result.confidence - 0.94).abs() < 0.001);
        assert!(result.reasoning.contains("bone structure"));
    }

    #[test]
    fn face_match_deserialize_no_match() {
        let json = r#"{
            "match": false,
            "confidence": 0.85,
            "reasoning": "Different jawline structure and eye shape — likely different people."
        }"#;
        let result: crate::gallery::vision::FaceMatchResult =
            serde_json::from_str(json).expect("deserialize");
        assert!(!result.is_match);
        assert!((result.confidence - 0.85).abs() < 0.001);
        assert!(result.reasoning.contains("Different"));
    }

    #[test]
    fn face_registry_lifecycle() {
        let (store, _temp) = setup_store();

        // Create a gallery and image for the face reference
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .expect("create gallery");
        let img = store
            .add_image(
                &gallery.id,
                "alice.jpg",
                "/tmp/test-gallery/alice.jpg",
                "hash1",
                400,
                600,
                "jpg",
                50000,
            )
            .expect("add image");

        // Register a face
        let face = store
            .register_face("Alice", "Chen", &img.id, None, "valid", "Frontal portrait")
            .expect("register face");
        assert_eq!(face.first_name, "Alice");
        assert_eq!(face.status, "valid");

        // List faces
        let faces = store.list_faces(None).expect("list faces");
        assert_eq!(faces.len(), 1);

        // Get by ID
        let retrieved = store.get_face(&face.id).expect("get face");
        assert_eq!(retrieved.last_name, "Chen");

        // Remove
        store.remove_face(&face.id).expect("remove face");
        let faces = store.list_faces(None).expect("list after remove");
        assert_eq!(faces.len(), 0);
    }

    #[test]
    fn face_registry_status_filter() {
        let (store, _temp) = setup_store();
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .expect("create gallery");
        let img1 = store
            .add_image(
                &gallery.id,
                "a.jpg",
                "/tmp/a.jpg",
                "h1",
                100,
                100,
                "jpg",
                1000,
            )
            .expect("add img1");
        let img2 = store
            .add_image(
                &gallery.id,
                "b.jpg",
                "/tmp/b.jpg",
                "h2",
                100,
                100,
                "jpg",
                1000,
            )
            .expect("add img2");

        store
            .register_face("Alice", "A", &img1.id, None, "valid", "")
            .unwrap();
        store
            .register_face("Bob", "B", &img2.id, None, "rejected", "Too dark")
            .unwrap();

        let valid = store.list_faces(Some("valid")).unwrap();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].first_name, "Alice");

        let rejected = store.list_faces(Some("rejected")).unwrap();
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].first_name, "Bob");

        let pending = store.list_faces(Some("pending")).unwrap();
        assert_eq!(pending.len(), 0);
    }
}
