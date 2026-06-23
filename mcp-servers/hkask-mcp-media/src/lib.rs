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

mod gallery;
mod templates;
mod video;

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
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use video::FfmpegRunner;

use ab_glyph::Font;
use face_id::analyzer::FaceAnalyzer;

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
        let ga = self.access_gallery()?;
        let mut state_clone = {
            let guard = self
                .gallery_state
                .lock()
                .map_err(|e| format!("Gallery state lock error: {}", e))?;
            guard
                .as_ref()
                .ok_or("No gallery organized. Use gallery_organize first.".to_string())?
                .clone()
        };
        let old_count = ga.image_count;
        let scan_result = state_clone.scan(recursive, None);
        let mut persisted = 0u32;
        for entry in &scan_result.entries {
            let abs_path = state_clone.path.join(&entry.relative_path);
            if self
                .gallery_store
                .add_image(
                    &ga.gallery_id,
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
        *self
            .gallery_state
            .lock()
            .map_err(|e| format!("Gallery state lock error: {}", e))? = Some(state_clone);
        Ok((
            ga.gallery_id,
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

#[tool_router(server_handler)]
impl MediaServer {
    // ── Gallery tools ────────────────────────────────────────────────────────

    #[tool(
        description = "Organize a photo gallery. Point at a folder — the system creates the index, scans for images, and returns status. Use gallery_search to find photos by content."
    )]
    async fn gallery_organize(
        &self,
        Parameters(GalleryOrganizeRequest {
            path,
            mode,
            recursive,
            auto_analyze,
        }): Parameters<GalleryOrganizeRequest>,
    ) -> String {
        execute_tool(self, "gallery_organize", async {
            let gallery_mode = match mode.as_str() {
                "read-only" => GalleryMode::ReadOnly,
                "copy-on-write" => GalleryMode::CopyOnWrite,
                "destructive" => GalleryMode::Destructive,
                other => {
                    return Err(McpToolError::invalid_argument(format!(
                        "Invalid mode '{}': must be read-only, copy-on-write, or destructive",
                        other
                    )));
                }
            };

            // Create gallery in SQLite
            let record = match self.gallery_store.create(&path, gallery_mode.clone()) {
                Ok(r) => r,
                Err(GalleryStoreError::AlreadyExists(_)) => {
                    // Re-scan existing gallery
                    match self.rescan_existing_gallery(recursive) {
                        Ok((gid, old_count, added, total, persisted)) => {
                            let result = serde_json::json!({
                                "status": "rescanned",
                                "gallery_id": gid,
                                "root_path": path,
                                "mode": mode,
                                "images_added": added,
                                "total_images": total,
                                "persisted": persisted,
                            });

                            if auto_analyze && added > 0 {
                                let new_indices: Vec<usize> = (old_count as usize
                                    ..(old_count as usize + added as usize))
                                    .collect();
                                let pipelines: Vec<String> =
                                    vec!["faces", "objects", "colors", "composition", "scene"]
                                        .into_iter()
                                        .map(|s| s.to_string())
                                        .collect();
                                let (analyzed, analyze_errors) =
                                    self.run_analysis_on_indices(&new_indices, &pipelines).await;
                                let mut r = result;
                                r["auto_analyzed"] = serde_json::json!(analyzed);
                                if !analyze_errors.is_empty() {
                                    r["analyze_errors"] = serde_json::json!(analyze_errors);
                                }
                                return Ok(r);
                            }

                            return Ok(result);
                        }
                        Err(e) => {
                            return Ok(serde_json::json!({
                                "status": "already_exists",
                                "message": e,
                            }));
                        }
                    }
                }
                Err(e) => {
                    return Err(McpToolError::internal(format!(
                        "Failed to create gallery: {}",
                        e
                    )));
                }
            };

            // Set up in-memory GalleryState
            let mut state = GalleryState::new(PathBuf::from(&path), gallery_mode.clone());
            state.validate().map_err(McpToolError::invalid_argument)?;
            state.ensure_meta_dir().map_err(McpToolError::internal)?;
            state.gallery_id = Some(record.id.clone());

            // Scan for images
            let scan_result = state.scan(recursive, None);
            let mut persisted = 0u32;
            for entry in &scan_result.entries {
                let abs_path = state.path.join(&entry.relative_path);
                if self
                    .gallery_store
                    .add_image(
                        &record.id,
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

            {
                let mut guard = self.gallery_state.lock().map_err(|e| {
                    McpToolError::internal(format!("Gallery state lock error: {}", e))
                })?;
                *guard = Some(state);
            }

            let result = serde_json::json!({
                "status": "organized",
                "gallery_id": record.id,
                "root_path": record.root_path,
                "mode": record.mode,
                "images_found": scan_result.added,
                "total_images": scan_result.total,
                "persisted": persisted,
                "message": "Gallery ready. Use gallery_search to find photos by content."
            });

            if auto_analyze && scan_result.added > 0 {
                let new_indices: Vec<usize> = (0..scan_result.added as usize).collect();
                let pipelines: Vec<String> =
                    vec!["faces", "objects", "colors", "composition", "scene"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect();
                let (analyzed, analyze_errors) =
                    self.run_analysis_on_indices(&new_indices, &pipelines).await;
                let mut r = result;
                r["auto_analyzed"] = serde_json::json!(analyzed);
                if !analyze_errors.is_empty() {
                    r["analyze_errors"] = serde_json::json!(analyze_errors);
                }
                Ok(r)
            } else {
                Ok(result)
            }
        })
        .await
    }

    #[tool(description = "Get gallery status: path, mode, image count, and total size.")]
    async fn gallery_status(&self) -> String {
        execute_tool(self, "gallery_status", async {
            match self.access_gallery() {
                Ok(ga) => Ok(serde_json::json!({
                    "gallery_id": ga.gallery_id,
                    "image_count": ga.image_count,
                    "root_path": ga.root_path.display().to_string(),
                })),
                Err(e) => Ok(serde_json::json!({
                    "status": "no_gallery",
                    "message": e,
                })),
            }
        })
        .await
    }

    #[tool(
        description = "Search your gallery by describing what you're looking for. Fuzzy-matches against AI-generated tags (objects, faces, colors, composition)."
    )]
    async fn gallery_search(
        &self,
        Parameters(GallerySearchRequest {
            query,
            limit,
            tag_types,
            min_similarity,
        }): Parameters<GallerySearchRequest>,
    ) -> String {
        execute_tool(self, "gallery_search", async {
            let ga = self
                .access_gallery()
                .map_err(McpToolError::invalid_argument)?;

            let all_tags = self
                .gallery_store
                .get_all_tags(&ga.gallery_id)
                .map_err(|e| McpToolError::internal(format!("Failed to query tags: {}", e)))?;

            let limit = limit.unwrap_or(10);
            let min_sim = min_similarity.unwrap_or(0.3);
            let type_filter: Option<Vec<String>> =
                tag_types.map(|t| t.into_iter().map(|s| s.to_lowercase()).collect());

            let mut image_scores: std::collections::HashMap<String, (f64, Vec<serde_json::Value>)> =
                std::collections::HashMap::new();

            for (tag, relative_path) in &all_tags {
                if let Some(ref filter) = type_filter {
                    if !filter.contains(&tag.tag_type.to_lowercase()) {
                        continue;
                    }
                }

                let sim = levenshtein_similarity(&query, &tag.value);
                if sim < min_sim {
                    continue;
                }

                let weighted_sim = sim * tag.confidence;
                let entry = image_scores
                    .entry(relative_path.clone())
                    .or_insert((0.0, Vec::new()));
                entry.0 = entry.0.max(weighted_sim);
                entry.1.push(serde_json::json!({
                    "tag_type": tag.tag_type,
                    "value": tag.value,
                    "similarity": sim,
                    "confidence": tag.confidence,
                }));
            }

            let mut ranked: Vec<(String, f64, Vec<serde_json::Value>)> = image_scores
                .into_iter()
                .map(|(path, (score, matches))| (path, score, matches))
                .collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ranked.truncate(limit);

            let results: Vec<serde_json::Value> = ranked
                .into_iter()
                .map(|(path, score, matches)| {
                    serde_json::json!({
                        "image": path,
                        "score": score,
                        "matching_tags": matches,
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "query": query,
                "results": results,
                "total_matches": results.len(),
            }))
        })
        .await
    }

    #[tool(
        description = "Find gallery images similar to a text description or to another image. Uses AI caption embeddings for semantic similarity (requires gallery_analyze to have been run first). Different from gallery_search which matches tags — this matches visual descriptions."
    )]
    async fn gallery_find_similar(
        &self,
        Parameters(GalleryFindSimilarRequest {
            text,
            image_index,
            limit,
            min_similarity,
        }): Parameters<GalleryFindSimilarRequest>,
    ) -> String {
        execute_tool(self, "gallery_find_similar", async {
            let query_label = text
                .clone()
                .unwrap_or_else(|| format!("image_index={}", image_index.unwrap_or(0)));

            if text.is_none() && image_index.is_none() {
                return Err(McpToolError::invalid_argument(
                    "Provide either 'text' or 'image_index' (not both).",
                ));
            }

            // Determine the query embedding
            let query_embedding: Vec<f32> = if let Some(ref query_text) = text {
                self.inference
                    .embed_text(query_text, None)
                    .await
                    .map_err(|e| {
                        McpToolError::unavailable(format!(
                            "Embedding model unavailable: {}. Configure a cloud provider.",
                            e
                        ))
                    })?
            } else if let Some(idx) = image_index {
                let image_id = self
                    .resolve_image_id(idx)
                    .map_err(McpToolError::invalid_argument)?;
                let tags = self
                    .gallery_store
                    .get_tags(&image_id)
                    .map_err(|e| {
                        McpToolError::internal(format!("Failed to query tags: {}", e))
                    })?;
                let captions: Vec<&str> = tags
                    .iter()
                    .filter(|t| t.tag_type == "caption")
                    .map(|t| t.value.as_str())
                    .collect();
                if captions.is_empty() {
                    return Err(McpToolError::invalid_argument(
                        "Image has no caption. Run gallery_analyze first to generate scene descriptions.",
                    ));
                }
                let caption_text = captions.join(" ");
                self.inference
                    .embed_text(&caption_text, None)
                    .await
                    .map_err(|e| {
                        McpToolError::unavailable(format!("Embedding model unavailable: {}", e))
                    })?
            } else {
                unreachable!();
            };

            // Collect captions for all images in the gallery
            let ga = self
                .access_gallery()
                .map_err(McpToolError::invalid_argument)?;

            let all_tags = self
                .gallery_store
                .get_all_tags(&ga.gallery_id)
                .map_err(|e| McpToolError::internal(format!("Failed to query tags: {}", e)))?;

            // Group captions by image path and embed them
            let mut candidates: Vec<(String, String)> = Vec::new();
            let mut current_path = String::new();
            let mut current_captions: Vec<String> = Vec::new();
            for (tag, path) in &all_tags {
                if tag.tag_type != "caption" {
                    continue;
                }
                if path != &current_path {
                    if !current_captions.is_empty() {
                        candidates.push((
                            std::mem::take(&mut current_path),
                            current_captions.join(" "),
                        ));
                        current_captions.clear();
                    }
                    current_path = path.clone();
                }
                current_captions.push(tag.value.clone());
            }
            if !current_captions.is_empty() {
                candidates.push((current_path, current_captions.join(" ")));
            }

            if candidates.is_empty() {
                return Ok(serde_json::json!({
                    "query": query_label,
                    "results": [],
                    "message": "No captions found. Run gallery_analyze first.",
                }));
            }

            // Embed candidate captions individually and compute similarity
            let candidate_texts: Vec<&str> = candidates.iter().map(|(_, c)| c.as_str()).collect();
            let mut candidate_embeddings = Vec::new();
            for ct in &candidate_texts {
                match self.inference.embed_text(ct, None).await {
                    Ok(v) => candidate_embeddings.push(v),
                    Err(_) => candidate_embeddings.push(vec![]),
                }
            }

            // Compute cosine similarity and rank
            let mut scored: Vec<(String, f32)> = candidates
                .iter()
                .zip(candidate_embeddings.iter())
                .filter_map(|((path, _), emb)| {
                    if emb.is_empty() {
                        return None;
                    }
                    let sim = cosine_similarity(&query_embedding, emb);
                    if sim >= min_similarity {
                        Some((path.clone(), sim))
                    } else {
                        None
                    }
                })
                .collect();

            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(limit);

            let results: Vec<serde_json::Value> = scored
                .into_iter()
                .map(|(path, score)| serde_json::json!({"image": path, "similarity": score}))
                .collect();

            Ok(serde_json::json!({
                "query": query_label,
                "results": results,
            }))
        })
        .await
    }

    #[tool(
        description = "Refresh the gallery: scan for new/removed images, then update all AI metadata (objects, colors, composition, scene descriptions). Face detection is OFF by default. When include_faces=true, also auto-matches detected faces against the face_registry — named faces get person names instead of face_group numbers."
    )]
    async fn gallery_refresh(
        &self,
        Parameters(GalleryRefreshRequest {
            recursive,
            include_faces,
            max_images,
        }): Parameters<GalleryRefreshRequest>,
    ) -> String {
        execute_tool(self, "gallery_refresh", async {
            let (gid, _old_count, added, total, persisted) = self
                .rescan_existing_gallery(recursive)
                .map_err(McpToolError::invalid_argument)?;

            let mut pipeline_names = vec!["objects", "colors", "composition", "scene"];
            if include_faces {
                pipeline_names.push("faces");
            }
            let pipelines: Vec<String> =
                pipeline_names.into_iter().map(|s| s.to_string()).collect();

            let all_indices: Vec<usize> = (0..total as usize).take(max_images).collect();
            let (analyzed, analyze_errors) =
                self.run_analysis_on_indices(&all_indices, &pipelines).await;

            let mut faces_matched = 0u32;
            let mut registry_count = 0usize;
            let mut match_errors: Vec<String> = Vec::new();
            if include_faces {
                let ga = match self.access_gallery() {
                    Ok(ga) => ga,
                    Err(e) => {
                        return Ok(serde_json::json!({
                            "status": "refreshed",
                            "gallery_id": gid,
                            "scan": {
                                "images_added": added,
                                "total_images": total,
                                "persisted": persisted,
                            },
                            "analysis": {
                                "images_analyzed": analyzed,
                                "pipelines": pipelines,
                            },
                            "face_matching": {
                                "error": format!("{} — cannot match faces", e)
                            },
                            "errors": {
                                "analysis": analyze_errors,
                                "matching": serde_json::json!([]),
                            },
                        }));
                    }
                };

                let registry = match self.gallery_store.list_faces(Some("valid")) {
                    Ok(faces) => faces,
                    Err(e) => {
                        match_errors.push(format!("Failed to query face registry: {}", e));
                        Vec::new()
                    }
                };
                registry_count = registry.len();

                let onnx_used = if self.face_analyzer.is_some() && !registry.is_empty() {
                    let (_onnx_faces, onnx_embeddings, onnx_errors) =
                        self.run_onnx_face_pipeline(&all_indices).await;
                    match_errors.extend(onnx_errors);

                    if !onnx_embeddings.is_empty() {
                        for (image_id, query_blob, bbox) in &onnx_embeddings {
                            let query_embedding = match blob_to_embedding(query_blob) {
                                Some(e) => e,
                                None => continue,
                            };

                            for reg_entry in &registry {
                                let ref_embedding = match &reg_entry.embedding {
                                    Some(blob) => match blob_to_embedding(blob) {
                                        Some(e) => e,
                                        None => continue,
                                    },
                                    None => continue,
                                };

                                let similarity =
                                    cosine_similarity(&query_embedding, &ref_embedding);
                                if similarity >= 0.6 {
                                    let name =
                                        format!("{} {}", reg_entry.first_name, reg_entry.last_name);
                                    let new_value = serde_json::json!({
                                        "name": name,
                                        "match_confidence": similarity,
                                        "registry_id": reg_entry.id,
                                        "method": "onnx",
                                        "bbox": bbox,
                                    });
                                    self.persist_tag(
                                        image_id,
                                        "face",
                                        &new_value.to_string(),
                                        similarity as f64,
                                        "arcface-onnx",
                                    );
                                    faces_matched += 1;
                                    break;
                                }
                            }
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !onnx_used && !registry.is_empty() {
                    let all_tags = match self.gallery_store.get_all_tags(&ga.gallery_id) {
                        Ok(t) => t,
                        Err(e) => {
                            match_errors.push(format!("Failed to query tags: {}", e));
                            Vec::new()
                        }
                    };

                    let vision_model = self.resolve_vision_model().await;
                    if vision_model.is_none() {
                        match_errors
                            .push("Face matching skipped: no vision model available".to_string());
                    }

                    for (tag, _path) in &all_tags {
                        if tag.tag_type != "face" || vision_model.is_none() {
                            continue;
                        }
                        let (vision_model, _vision_label) = vision_model.as_ref().unwrap();

                        let face_image_id = &tag.image_id;

                        let face_bbox: Option<serde_json::Value> =
                            serde_json::from_str::<serde_json::Value>(&tag.value)
                                .ok()
                                .and_then(|v| v.get("bbox").cloned());

                        let query_url = if let Some(ref bbox) = face_bbox {
                            match self.crop_face_region(face_image_id, bbox) {
                                Ok(cropped_url) => cropped_url,
                                Err(_) => match self.resolve_image_url_by_id(face_image_id) {
                                    Ok(url) => url,
                                    Err(e) => {
                                        match_errors.push(format!("Face tag {}: {}", tag.id, e));
                                        continue;
                                    }
                                },
                            }
                        } else {
                            match self.resolve_image_url_by_id(face_image_id) {
                                Ok(url) => url,
                                Err(e) => {
                                    match_errors.push(format!("Face tag {}: {}", tag.id, e));
                                    continue;
                                }
                            }
                        };

                        for reg_entry in &registry {
                            let ref_url = match self.resolve_image_url_by_id(&reg_entry.image_id) {
                                Ok(url) => url,
                                Err(e) => {
                                    match_errors
                                        .push(format!("Registry entry {}: {}", reg_entry.id, e));
                                    continue;
                                }
                            };

                            match vision::match_faces(
                                &self.inference,
                                &self.template_env,
                                &ref_url,
                                &query_url,
                                Some(vision_model),
                            )
                            .await
                            {
                                Ok(result) => {
                                    if result.is_match && result.confidence >= 0.7 {
                                        let name = format!(
                                            "{} {}",
                                            reg_entry.first_name, reg_entry.last_name
                                        );
                                        if let Ok(parsed) =
                                            serde_json::from_str::<serde_json::Value>(&tag.value)
                                        {
                                            let face_index = parsed["face_index"].as_u64();
                                            let new_value = serde_json::json!({
                                                "face_index": face_index,
                                                "name": name,
                                                "match_confidence": result.confidence,
                                                "registry_id": reg_entry.id,
                                                "method": "vision_llm",
                                            });
                                            self.persist_tag(
                                                &tag.image_id,
                                                "face",
                                                &new_value.to_string(),
                                                result.confidence,
                                                vision_model,
                                            );
                                            faces_matched += 1;
                                        }
                                        break;
                                    }
                                }
                                Err(e) => {
                                    match_errors.push(format!(
                                        "Match {} vs {}: {}",
                                        reg_entry.id, tag.id, e
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            Ok(serde_json::json!({
                "status": "refreshed",
                "gallery_id": gid,
                "scan": {
                    "images_added": added,
                    "total_images": total,
                    "persisted": persisted,
                },
                "analysis": {
                    "images_analyzed": analyzed,
                    "pipelines": pipelines,
                },
                "face_matching": {
                    "faces_matched": faces_matched,
                    "registry_entries": registry_count,
                },
                "errors": {
                    "analysis": analyze_errors,
                    "matching": match_errors,
                },
            }))
        })
        .await
    }

    // ── Image tools ──────────────────────────────────────────────────────────

    #[tool(
        description = "Describe an image in detail. Choose a style: descriptive (full scene), artistic (poetic), technical (photographic analysis), or alt_text (accessibility)."
    )]
    async fn describe_image(
        &self,
        Parameters(DescribeImageRequest { image_url, style }): Parameters<DescribeImageRequest>,
    ) -> String {
        execute_tool(self, "describe_image", async {
            validate_tool_url(&image_url)?;

            let style_str = style.as_deref().unwrap_or("descriptive");
            let mut vars = HashMap::new();
            vars.insert("style", style_str);
            let prompt = self
                .render_prompt("caption", &vars)
                .map_err(|e| McpToolError::internal(format!("Template render failed: {}", e)))?;

            let (vision_model, _vision_label) = self.require_vision().await?;
            let params = hkask_types::template::LLMParameters::default();
            let r = self
                .inference
                .generate_vision(&prompt, &[image_url], &params, Some(vision_model))
                .await
                .map_err(|e| {
                    McpToolError::unavailable(format!("Vision inference failed: {}", e))
                })?;

            Ok(serde_json::json!({"description": r.text.trim(), "style": style_str}))
        })
        .await
    }

    // ── Analysis tools ──────────────────────────────────────────────────────

    #[tool(
        description = "Analyze gallery images with AI: detect faces, objects, colors, composition, and generate scene descriptions. Tags are persisted and become searchable."
    )]
    async fn gallery_analyze(
        &self,
        Parameters(GalleryAnalyzeRequest {
            mode,
            image_indices,
            pipelines,
            max_images,
        }): Parameters<GalleryAnalyzeRequest>,
    ) -> String {
        execute_tool(self, "gallery_analyze", async {
            let ga = self
                .access_gallery()
                .map_err(McpToolError::invalid_argument)?;

            let indices: Vec<usize> = match mode.as_str() {
                "selection" => image_indices.unwrap_or_default(),
                "all" => (0..ga.image_count as usize).collect(),
                _ => {
                    let mut untagged = Vec::new();
                    for i in 0..ga.image_count as usize {
                        if let Ok(image_id) = self.resolve_image_id(i) {
                            match self.gallery_store.get_tags(&image_id) {
                                Ok(tags) if tags.is_empty() => untagged.push(i),
                                Ok(_) => continue,
                                Err(_) => untagged.push(i),
                            }
                        }
                    }
                    untagged
                }
            };

            let indices: Vec<usize> = indices.into_iter().take(max_images).collect();
            if indices.is_empty() {
                return Ok(serde_json::json!({
                    "status": "nothing_to_analyze",
                    "message": "No images to analyze."
                }));
            }

            let all_pipelines: Vec<String> =
                vec!["faces", "objects", "colors", "composition", "scene"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
            let pipelines = pipelines.unwrap_or(all_pipelines);

            let (analyzed, errors) = self.run_analysis_on_indices(&indices, &pipelines).await;

            let vision_label = self
                .resolve_vision_model()
                .await
                .map(|(_, label)| label)
                .unwrap_or("none");

            Ok(serde_json::json!({
                "status": "complete",
                "images_analyzed": analyzed,
                "total_images": indices.len(),
                "pipelines_run": pipelines,
                "model": vision_label,
                "errors": errors,
            }))
        })
        .await
    }

    #[tool(
        description = "Name a face group from gallery_analyze. Provide either a free-text 'name' or a 'face_id' from the face registry (which auto-resolves to 'First Last'). After naming, gallery_search can find photos of that person by name."
    )]
    async fn gallery_name_face(
        &self,
        Parameters(GalleryNameFaceRequest {
            face_group,
            name,
            face_id,
        }): Parameters<GalleryNameFaceRequest>,
    ) -> String {
        execute_tool(self, "gallery_name_face", async {
            let resolved_name = if let Some(ref fid) = face_id {
                self.gallery_store
                    .get_face(fid)
                    .map(|face| format!("{} {}", face.first_name, face.last_name))
                    .map_err(|e| {
                        McpToolError::invalid_argument(format!("Face registry ID not found: {}", e))
                    })?
            } else {
                match name {
                    Some(n) if !n.trim().is_empty() => n,
                    _ => {
                        return Err(McpToolError::invalid_argument(
                            "Either 'name' or 'face_id' must be provided.",
                        ));
                    }
                }
            };

            let ga = self
                .access_gallery()
                .map_err(McpToolError::invalid_argument)?;

            let all_tags = self
                .gallery_store
                .get_all_tags(&ga.gallery_id)
                .map_err(|e| McpToolError::internal(format!("Failed to query tags: {}", e)))?;

            let mut renamed = 0u32;
            for (tag, _path) in &all_tags {
                if tag.tag_type != "face" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&tag.value) {
                    if parsed["face_index"].as_u64() == Some(face_group as u64) {
                        let new_value = serde_json::json!({
                            "face_index": face_group,
                            "name": resolved_name,
                        });
                        self.persist_tag(
                            &tag.image_id,
                            "face",
                            &new_value.to_string(),
                            1.0,
                            "user",
                        );
                        renamed += 1;
                    }
                }
            }

            Ok(serde_json::json!({
                "status": "named",
                "face_group": face_group,
                "name": resolved_name,
                "images_updated": renamed,
            }))
        })
        .await
    }

    // ── Face registry tools ─────────────────────────────────────────────────

    #[tool(
        description = "Validate a gallery image as a face reference for facial recognition. Checks: exactly 1 face, face coverage ≥15%, frontal pose, good lighting, no occlusion, sharp focus. Returns structured pass/fail with specific reasons."
    )]
    async fn face_validate(
        &self,
        Parameters(FaceValidateRequest { image_index }): Parameters<FaceValidateRequest>,
    ) -> String {
        execute_tool(self, "face_validate", async {
            let image_url = self
                .resolve_image_url(image_index)
                .map_err(McpToolError::invalid_argument)?;

            let (vision_model, _vision_label) = self.require_vision().await?;

            let validation = vision::validate_face_reference(
                &self.inference,
                &self.template_env,
                &image_url,
                Some(vision_model),
            )
            .await
            .map_err(|e| McpToolError::internal(format!("Face validation failed: {}", e)))?;

            Ok(serde_json::json!(validation))
        })
        .await
    }

    #[tool(
        description = "Register a face reference with a person's name. Auto-validates the image against 6 criteria (face count, coverage, pose, lighting, occlusion, clarity). Pass --force to skip validation and register directly as valid. Stores in the face_registry table for automatic matching during gallery_refresh."
    )]
    async fn face_register(
        &self,
        Parameters(FaceRegisterRequest {
            image_index,
            first_name,
            last_name,
            force,
        }): Parameters<FaceRegisterRequest>,
    ) -> String {
        execute_tool(self, "face_register", async {
            let image_id = self
                .resolve_image_id(image_index)
                .map_err(McpToolError::invalid_argument)?;

            let (status, notes, validation) = if force {
                ("valid", String::new(), None)
            } else {
                let image_url = self
                    .resolve_image_url(image_index)
                    .map_err(McpToolError::invalid_argument)?;

                let (vision_model, _vision_label) = self.require_vision().await?;

                let v = vision::validate_face_reference(
                    &self.inference,
                    &self.template_env,
                    &image_url,
                    Some(vision_model),
                )
                .await
                .map_err(|e| {
                    McpToolError::internal(format!("Face validation failed: {}", e))
                })?;

                let status = if v.valid { "valid" } else { "rejected" };
                let notes = if v.valid {
                    String::new()
                } else {
                    v.issues.join("; ")
                };
                (status, notes, Some(v))
            };

            let embedding_blob = if let Some(ref analyzer) = self.face_analyzer {
                match self.resolve_image_path(image_index) {
                    Ok(path) => match image::open(&path) {
                        Ok(img) => match analyzer.analyze(&img) {
                            Ok(faces) => faces.first().map(|f| embedding_to_blob(&f.embedding)),
                            Err(e) => {
                                tracing::warn!(target: "cns.mcp.media.face", error = %e, "ONNX face analysis failed during registration");
                                None
                            }
                        },
                        Err(e) => {
                            tracing::warn!(target: "cns.mcp.media.face", error = %e, "Failed to open image for embedding");
                            None
                        }
                    },
                    Err(_) => None,
                }
            } else {
                None
            };

            let record = self
                .gallery_store
                .register_face(
                    &first_name,
                    &last_name,
                    &image_id,
                    embedding_blob.as_deref(),
                    status,
                    &notes,
                )
                .map_err(|e| {
                    McpToolError::internal(format!("Failed to register face: {}", e))
                })?;

            Ok(serde_json::json!({
                "face_id": record.id,
                "first_name": record.first_name,
                "last_name": record.last_name,
                "status": record.status,
                "validation": validation,
                "notes": record.notes,
            }))
        })
        .await
    }

    #[tool(
        description = "List all registered faces in the face registry. Optionally filter by status: 'valid', 'rejected', or 'pending'."
    )]
    async fn face_list(
        &self,
        Parameters(FaceListRequest { status }): Parameters<FaceListRequest>,
    ) -> String {
        execute_tool(self, "face_list", async {
            let faces = self
                .gallery_store
                .list_faces(status.as_deref())
                .map_err(|e| McpToolError::internal(format!("Failed to list faces: {}", e)))?;

            Ok(serde_json::json!({
                "count": faces.len(),
                "faces": faces,
            }))
        })
        .await
    }

    #[tool(
        description = "Remove a face from the registry by its ID (returned by face_register or face_list)."
    )]
    async fn face_remove(
        &self,
        Parameters(FaceRemoveRequest { face_id }): Parameters<FaceRemoveRequest>,
    ) -> String {
        execute_tool(self, "face_remove", async {
            self.gallery_store
                .remove_face(&face_id)
                .map_err(|e| McpToolError::invalid_argument(format!("Face not found: {}", e)))?;
            Ok(serde_json::json!({
                "status": "removed",
                "face_id": face_id,
            }))
        })
        .await
    }

    #[tool(
        description = "Extract a specific object from an image using AI segmentation. Returns the isolated object as a new image."
    )]
    async fn extract_object(
        &self,
        Parameters(ExtractObjectRequest {
            image_index,
            object_description,
        }): Parameters<ExtractObjectRequest>,
    ) -> String {
        execute_tool(self, "extract_object", async {
            let image_url = self
                .resolve_image_url(image_index)
                .map_err(McpToolError::invalid_argument)?;

            self.inference
                .segment_object(&image_url, &object_description)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Object extraction failed: {}", e)))
        })
        .await
    }

    #[tool(
        description = "Organize gallery images by time period using EXIF dates. Returns images grouped by year, month, or decade."
    )]
    async fn gallery_timeline(
        &self,
        Parameters(GalleryTimelineRequest {
            period,
            count,
            per_period,
            search_terms,
        }): Parameters<GalleryTimelineRequest>,
    ) -> String {
        execute_tool(self, "gallery_timeline", async {
            let ga = self
                .access_gallery()
                .map_err(McpToolError::invalid_argument)?;

            let mut dated_images: Vec<(String, String)> = Vec::new();
            for idx in 0..ga.image_count as usize {
                let img = match self
                    .gallery_store
                    .get_image(&ga.gallery_id, Some(idx), None)
                {
                    Ok(i) => i,
                    Err(_) => continue,
                };

                if let Some(ref terms) = search_terms {
                    let tags = self.gallery_store.get_tags(&img.id).unwrap_or_default();
                    let matches = terms.iter().any(|term| {
                        tags.iter()
                            .any(|t| t.value.to_lowercase().contains(&term.to_lowercase()))
                    });
                    if !matches {
                        continue;
                    }
                }

                let exif = Self::extract_exif(&img.absolute_path);
                let date_str = exif
                    .get("date_taken")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let period_key = match period.as_str() {
                    "month" => date_str.chars().take(7).collect(),
                    "decade" => format!("{}0s", &date_str[..3]),
                    _ => date_str.chars().take(4).collect(),
                };

                dated_images.push((period_key, img.relative_path));
            }

            let mut periods: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            for (key, path) in &dated_images {
                periods.entry(key.clone()).or_default().push(path.clone());
            }

            let mut result_periods: Vec<serde_json::Value> = Vec::new();
            for (key, images) in periods.iter().rev().take(count) {
                let selected: Vec<&String> = images.iter().take(per_period).collect();
                result_periods.push(serde_json::json!({
                    "period": key,
                    "total_images": images.len(),
                    "images": selected,
                }));
            }

            Ok(serde_json::json!({
                "period_type": period,
                "periods": result_periods,
            }))
        })
        .await
    }

    // ── Derivation tools ─────────────────────────────────────────────────────

    #[tool(
        description = "Remove background from a gallery image. Delegates to DeepInfra Bria RMBG 2.0."
    )]
    async fn image_remove_background(
        &self,
        Parameters(RemoveBackgroundRequest {
            image_index,
            new_bg_color: _new_bg_color,
        }): Parameters<RemoveBackgroundRequest>,
    ) -> String {
        execute_tool(self, "image_remove_background", async {
            let image_url = self
                .resolve_image_url(image_index)
                .map_err(McpToolError::invalid_argument)?;

            self.inference
                .remove_background(&image_url)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Background removal failed: {}", e)))
        })
        .await
    }

    #[tool(
        description = "Apply style transfer to a gallery image. Delegates to fal.ai Flux dev img2img."
    )]
    async fn image_apply_style(
        &self,
        Parameters(ApplyStyleRequest {
            image_index,
            style_prompt,
            strength,
        }): Parameters<ApplyStyleRequest>,
    ) -> String {
        execute_tool(self, "image_apply_style", async {
            let image_url = self
                .resolve_image_url(image_index)
                .map_err(McpToolError::invalid_argument)?;

            self.inference
                .image_to_image(&image_url, &style_prompt, strength)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Style transfer failed: {}", e)))
        })
        .await
    }

    #[tool(
        description = "Create a collage from multiple gallery images. Local composition using image crate. Three modes: search_terms (semantic tag search), similar_to_index (visually similar images), or image_indices (explicit list)."
    )]
    async fn image_create_collage(
        &self,
        Parameters(CreateCollageRequest {
            search_terms,
            similar_to_index,
            image_indices,
            max_items,
            layout,
            spacing,
            canvas_size,
        }): Parameters<CreateCollageRequest>,
    ) -> String {
        execute_tool(self, "image_create_collage", async {
            let mode_count = search_terms.is_some() as u8
                + similar_to_index.is_some() as u8
                + image_indices.is_some() as u8;
            if mode_count == 0 {
                return Err(McpToolError::invalid_argument(
                    "Must specify one of: search_terms, similar_to_index, or image_indices.",
                ));
            }
            if mode_count > 1 {
                return Err(McpToolError::invalid_argument(
                    "search_terms, similar_to_index, and image_indices are mutually exclusive. Choose one.",
                ));
            }

            let ga = self
                .access_gallery()
                .map_err(McpToolError::invalid_argument)?;

            let mut paths = Vec::new();

            if let Some(ref terms) = search_terms {
                let all_tags = self
                    .gallery_store
                    .get_all_tags(&ga.gallery_id)
                    .map_err(|e| {
                        McpToolError::internal(format!("Failed to query tags: {}", e))
                    })?;

                let mut image_scores: HashMap<String, f64> = HashMap::new();
                for (tag, relative_path) in &all_tags {
                    for term in terms {
                        let sim = levenshtein_similarity(term, &tag.value);
                        if sim >= 0.3 {
                            let weighted = sim * tag.confidence;
                            let entry =
                                image_scores.entry(relative_path.clone()).or_insert(0.0);
                            *entry = entry.max(weighted);
                        }
                    }
                }

                let mut ranked: Vec<(String, f64)> = image_scores.into_iter().collect();
                ranked
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                ranked.truncate(max_items);

                for (rel_path, _score) in &ranked {
                    paths.push(ga.root_path.join(rel_path));
                }
            } else if let Some(ref_idx) = similar_to_index {
                let ref_path = self
                    .resolve_image_path(ref_idx)
                    .map_err(McpToolError::invalid_argument)?;
                let ref_image_id = self
                    .resolve_image_id(ref_idx)
                    .map_err(McpToolError::invalid_argument)?;
                let ref_tags = self.gallery_store.get_tags(&ref_image_id).map_err(|e| {
                    McpToolError::internal(format!("Failed to get reference tags: {}", e))
                })?;

                let all_tags = self
                    .gallery_store
                    .get_all_tags(&ga.gallery_id)
                    .map_err(|e| {
                        McpToolError::internal(format!("Failed to query tags: {}", e))
                    })?;

                let mut image_scores: HashMap<String, f64> = HashMap::new();
                for (tag, relative_path) in &all_tags {
                    let abs_path = ga.root_path.join(relative_path);
                    if abs_path == ref_path {
                        continue;
                    }
                    for ref_tag in &ref_tags {
                        let sim = levenshtein_similarity(&ref_tag.value, &tag.value);
                        if sim >= 0.3 {
                            let weighted = sim * tag.confidence;
                            let entry =
                                image_scores.entry(relative_path.clone()).or_insert(0.0);
                            *entry = entry.max(weighted);
                        }
                    }
                }

                let mut ranked: Vec<(String, f64)> = image_scores.into_iter().collect();
                ranked
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                ranked.truncate(max_items.saturating_sub(1));

                paths.push(ref_path);
                for (rel_path, _score) in &ranked {
                    paths.push(ga.root_path.join(rel_path));
                }
            } else if let Some(ref indices) = image_indices {
                if indices.is_empty() {
                    return Err(McpToolError::invalid_argument(
                        "At least one image index is required.",
                    ));
                }
                if indices.len() > 9 {
                    return Err(McpToolError::invalid_argument(
                        "Maximum 9 images supported for collage.",
                    ));
                }
                let limit = indices.len().min(max_items);
                for idx in indices.iter().take(limit) {
                    paths.push(
                        self.resolve_image_path(*idx)
                            .map_err(McpToolError::invalid_argument)?,
                    );
                }
            }

            if paths.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "No images found for collage.",
                ));
            }

            let mut images = Vec::new();
            for path in &paths {
                images.push(
                    image::open(path).map_err(|e| {
                        McpToolError::internal(format!(
                            "Failed to open {}: {}",
                            path.display(),
                            e
                        ))
                    })?,
                );
            }

            if images.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "At least one image is required for collage composition",
                ));
            }

            let cols = match layout.as_str() {
                "horizontal" => images.len() as u32,
                "vertical" => 1u32,
                "masonry" => 3u32.min(images.len() as u32),
                _ => (images.len() as f64).sqrt().ceil() as u32,
            };
            let rows = (images.len() as u32).div_ceil(cols);

            let parts: Vec<&str> = canvas_size.split('x').collect();
            let canvas_w: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1200);
            let canvas_h: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(900);

            let cell_w = (canvas_w - spacing * (cols + 1)) / cols;
            let cell_h = (canvas_h - spacing * (rows + 1)) / rows;

            let mut canvas = image::DynamicImage::new_rgba8(canvas_w, canvas_h);
            let bg = image::Rgba([30u8, 30u8, 30u8, 255u8]);
            for pixel in canvas
                .as_mut_rgba8()
                .expect("canvas was created as RGBA8")
                .pixels_mut()
            {
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

            let temp_dir = std::env::temp_dir().join("hkask-media");
            let _ = std::fs::create_dir_all(&temp_dir);
            let output_path = temp_dir.join(format!("collage_{}.png", uuid::Uuid::new_v4()));

            canvas.save(&output_path).map_err(|e| {
                McpToolError::internal(format!("Failed to save collage: {}", e))
            })?;

            Ok(serde_json::json!({
                "status": "created",
                "image_count": images.len(),
                "layout": layout,
                "cols": cols,
                "rows": rows,
                "canvas_width": canvas_w,
                "canvas_height": canvas_h,
                "spacing": spacing,
                "output": output_path.display().to_string(),
            }))
        })
        .await
    }

    // ── Video tools ──────────────────────────────────────────────────────────

    #[tool(description = "Trim a video to specified start/end times using local ffmpeg.")]
    async fn video_clip(
        &self,
        Parameters(VideoClipRequest {
            video_url,
            start_sec,
            end_sec,
        }): Parameters<VideoClipRequest>,
    ) -> String {
        execute_tool(self, "video_clip", async {
            validate_tool_url(&video_url)?;

            if start_sec >= end_sec {
                return Err(McpToolError::invalid_argument(
                    "start_sec must be less than end_sec.",
                ));
            }

            self.require_ffmpeg()?;

            let output = self
                .ffmpeg
                .clip(&video_url, start_sec, end_sec)
                .await
                .map_err(McpToolError::internal)?;

            Ok(serde_json::json!({
                "status": "clipped",
                "source": video_url,
                "start_sec": start_sec,
                "end_sec": end_sec,
                "duration": end_sec - start_sec,
                "output": output.display().to_string(),
            }))
        })
        .await
    }

    #[tool(description = "Convert a video segment to GIF format using local ffmpeg.")]
    async fn video_to_gif(
        &self,
        Parameters(VideoToGifRequest {
            video_url,
            start_sec,
            duration_sec,
            width,
            fps,
        }): Parameters<VideoToGifRequest>,
    ) -> String {
        execute_tool(self, "video_to_gif", async {
            validate_tool_url(&video_url)?;

            self.require_ffmpeg()?;

            let start = start_sec.unwrap_or(0.0);
            let dur = duration_sec.unwrap_or(5.0);
            let w = width.unwrap_or(480);
            let f = fps.unwrap_or(10);

            let output = self
                .ffmpeg
                .to_gif(&video_url, start, dur, w, f)
                .await
                .map_err(McpToolError::internal)?;

            Ok(serde_json::json!({
                "status": "converted",
                "source": video_url,
                "start_sec": start,
                "duration_sec": dur,
                "width": w,
                "fps": f,
                "output": output.display().to_string(),
            }))
        })
        .await
    }

    #[tool(
        description = "Animate a gallery image into a short video clip. Delegates to fal.ai Seedance 2.0."
    )]
    async fn image_to_video(
        &self,
        Parameters(ImageToVideoRequest {
            image_index,
            prompt,
            duration,
            model: _model,
        }): Parameters<ImageToVideoRequest>,
    ) -> String {
        execute_tool(self, "image_to_video", async {
            let image_url = self
                .resolve_image_url(image_index)
                .map_err(McpToolError::invalid_argument)?;

            self.inference
                .image_to_video(&image_url, prompt.as_deref(), duration)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Image-to-video failed: {}", e)))
        })
        .await
    }

    #[tool(description = "Add text caption overlay to a video using local ffmpeg.")]
    async fn video_add_caption(
        &self,
        Parameters(VideoAddCaptionRequest {
            video_url,
            text,
            position,
            font_size,
        }): Parameters<VideoAddCaptionRequest>,
    ) -> String {
        execute_tool(self, "video_add_caption", async {
            validate_tool_url(&video_url)?;

            self.require_ffmpeg()?;

            let pos = position.as_deref().unwrap_or("bottom");
            let size = font_size.unwrap_or(24);
            if size == 0 {
                return Err(McpToolError::invalid_argument(
                    "font_size must be greater than 0",
                ));
            }

            let output = self
                .ffmpeg
                .add_caption(&video_url, &text, pos, size)
                .await
                .map_err(McpToolError::internal)?;

            Ok(serde_json::json!({
                "status": "captioned",
                "source": video_url,
                "text": text,
                "position": pos,
                "font_size": size,
                "output": output.display().to_string(),
            }))
        })
        .await
    }

    #[tool(description = "Generate a video remix: clip, add caption, convert to GIF.")]
    async fn video_remix(
        &self,
        Parameters(VideoRemixRequest {
            video_url,
            start_sec,
            end_sec,
            caption_text,
        }): Parameters<VideoRemixRequest>,
    ) -> String {
        execute_tool(self, "video_remix", async {
            validate_tool_url(&video_url)?;

            if start_sec >= end_sec {
                return Err(McpToolError::invalid_argument(
                    "start_sec must be less than end_sec.",
                ));
            }

            self.require_ffmpeg()?;

            let clipped = self
                .ffmpeg
                .clip(&video_url, start_sec, end_sec)
                .await
                .map_err(|e| McpToolError::internal(format!("Clip step failed: {}", e)))?;

            let captioned = if let Some(ref cap) = caption_text {
                self.ffmpeg
                    .add_caption(&clipped.to_string_lossy(), cap, "bottom", 24)
                    .await
                    .map_err(|e| McpToolError::internal(format!("Caption step failed: {}", e)))?
            } else {
                clipped.clone()
            };

            let gif = self
                .ffmpeg
                .to_gif(
                    &captioned.to_string_lossy(),
                    0.0,
                    end_sec - start_sec,
                    480,
                    10,
                )
                .await
                .map_err(|e| McpToolError::internal(format!("GIF step failed: {}", e)))?;

            let _ = std::fs::remove_file(&clipped);
            if caption_text.is_some() {
                let _ = std::fs::remove_file(&captioned);
            }

            Ok(serde_json::json!({
                "status": "remixed",
                "source": video_url,
                "start_sec": start_sec,
                "end_sec": end_sec,
                "caption": caption_text,
                "output": gif.display().to_string(),
            }))
        })
        .await
    }

    #[tool(description = "Create a video or GIF from a sequence of gallery images using ffmpeg.")]
    async fn video_from_images(
        &self,
        Parameters(VideoFromImagesRequest {
            image_indices,
            fps,
            format,
        }): Parameters<VideoFromImagesRequest>,
    ) -> String {
        execute_tool(self, "video_from_images", async {
            if image_indices.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "At least one image index is required.",
                ));
            }

            self.require_ffmpeg()?;

            let mut paths = Vec::new();
            for idx in &image_indices {
                paths.push(
                    self.resolve_image_path(*idx)
                        .map_err(McpToolError::invalid_argument)?,
                );
            }

            let fps = fps.unwrap_or(24);
            let fmt = format.as_deref().unwrap_or("mp4");

            let output = self
                .ffmpeg
                .images_to_video(&paths, fps, fmt)
                .await
                .map_err(McpToolError::internal)?;

            Ok(serde_json::json!({
                "status": "created",
                "frame_count": paths.len(),
                "fps": fps,
                "format": fmt,
                "output": output.display().to_string(),
            }))
        })
        .await
    }

    #[tool(description = "Concatenate multiple video clips into one using ffmpeg.")]
    async fn video_concat(
        &self,
        Parameters(VideoConcatRequest { video_urls }): Parameters<VideoConcatRequest>,
    ) -> String {
        execute_tool(self, "video_concat", async {
            if video_urls.len() < 2 {
                return Err(McpToolError::invalid_argument(
                    "At least 2 video URLs are required.",
                ));
            }

            self.require_ffmpeg()?;

            let output = self
                .ffmpeg
                .concat(&video_urls)
                .await
                .map_err(McpToolError::internal)?;

            Ok(serde_json::json!({
                "status": "concatenated",
                "clip_count": video_urls.len(),
                "output": output.display().to_string(),
            }))
        })
        .await
    }

    #[tool(
        description = "Generate a description of video content by extracting keyframes and analyzing them with a vision LLM."
    )]
    async fn video_caption(
        &self,
        Parameters(VideoCaptionRequest { video_url, style }): Parameters<VideoCaptionRequest>,
    ) -> String {
        execute_tool(self, "video_caption", async {
            let style_str = style.as_deref().unwrap_or("descriptive");
            self.require_ffmpeg()?;

            let frames = self
                .ffmpeg
                .extract_keyframes(&video_url, 2.0, 10)
                .await
                .map_err(|e| {
                    McpToolError::internal(format!("Keyframe extraction failed: {}", e))
                })?;

            if frames.is_empty() {
                return Err(McpToolError::internal(
                    "No keyframes extracted from video.",
                ));
            }

            let mut image_urls = Vec::new();
            for frame in &frames {
                match std::fs::read(frame) {
                    Ok(data) => {
                        let b64 = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &data,
                        );
                        image_urls.push(format!("data:image/jpeg;base64,{}", b64));
                    }
                    Err(e) => {
                        tracing::warn!(target: "cns.mcp.media", frame = %frame.display(), error = %e, "Failed to read keyframe");
                    }
                }
            }

            let mut vars = HashMap::new();
            vars.insert("style", style_str);
            let prompt = self
                .render_prompt("video_caption", &vars)
                .map_err(|e| McpToolError::internal(format!("Template render failed: {}", e)))?;

            let (vision_model, _vision_label) = self.require_vision().await?;
            let params = hkask_types::template::LLMParameters::default();
            let result = self
                .inference
                .generate_vision(&prompt, &image_urls, &params, Some(vision_model))
                .await;

            for frame in &frames {
                let _ = std::fs::remove_file(frame);
            }

            match result {
                Ok(r) => Ok(serde_json::json!({
                    "caption": r.text.trim(),
                    "style": style_str,
                    "frames_analyzed": image_urls.len(),
                })),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Vision inference failed: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Create a meme video from a gallery image with text overlay and camera motion. Composes text rendering + AI motion generation. Perfect for 'WHEN YOU SEE IT' style memes."
    )]
    async fn video_meme(
        &self,
        Parameters(VideoMemeRequest {
            image_index,
            top_text,
            bottom_text,
            motion,
            duration,
            font_path,
        }): Parameters<VideoMemeRequest>,
    ) -> String {
        execute_tool(self, "video_meme", async {
            let image_path = self
                .resolve_image_path(image_index)
                .map_err(McpToolError::invalid_argument)?;

            let mut img = image::open(&image_path)
                .map_err(|e| McpToolError::internal(format!("Failed to open image: {}", e)))?;

            let font = load_meme_font(font_path.as_deref()).map_err(|e| {
                McpToolError::unavailable(format!(
                    "No font available for text rendering: {}. Install fonts-dejavu-core or provide --font_path.",
                    e
                ))
            })?;

            let img_w = img.width();
            let img_h = img.height();
            let scale = ab_glyph::PxScale::from(img_h as f32 * 0.10);
            let white = image::Rgba([255u8, 255u8, 255u8, 255u8]);
            let black = image::Rgba([0u8, 0u8, 0u8, 255u8]);

            if let Some(ref text) = top_text {
                let text_upper: String = text.to_uppercase();
                let (tw, _th) = measure_text(&font, scale, &text_upper);
                let x = ((img_w as i32 - tw as i32) / 2).max(0);
                let y = (img_h as f32 * 0.05) as i32;
                for &(dx, dy) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    imageproc::drawing::draw_text_mut(
                        &mut img, black, x + dx, y + dy, scale, &font, &text_upper,
                    );
                }
                imageproc::drawing::draw_text_mut(&mut img, white, x, y, scale, &font, &text_upper);
            }

            if let Some(ref text) = bottom_text {
                let text_upper: String = text.to_uppercase();
                let (tw, th) = measure_text(&font, scale, &text_upper);
                let x = ((img_w as i32 - tw as i32) / 2).max(0);
                let y = (img_h as i32 - th as i32 - (img_h as f32 * 0.05) as i32).max(0);
                for &(dx, dy) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    imageproc::drawing::draw_text_mut(
                        &mut img, black, x + dx, y + dy, scale, &font, &text_upper,
                    );
                }
                imageproc::drawing::draw_text_mut(&mut img, white, x, y, scale, &font, &text_upper);
            }

            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| {
                    McpToolError::internal(format!("Failed to encode composited image: {}", e))
                })?;
            let b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf.get_ref());
            let data_uri = format!("data:image/png;base64,{}", b64);

            let motion_prompt = if motion.is_empty() {
                "slow zoom in".to_string()
            } else {
                motion.clone()
            };
            self.inference
                .image_to_video(&data_uri, Some(&motion_prompt), duration)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Image-to-video failed: {}", e)))
        })
        .await
    }

    // ── Voice tools ──────────────────────────────────────────────────────────

    #[tool(
        description = "Design a synthetic voice profile from a character description. Returns a VoiceDesign JSON for use with generate_speech."
    )]
    async fn voice_design(
        &self,
        Parameters(VoiceDesignRequest {
            character_description,
        }): Parameters<VoiceDesignRequest>,
    ) -> String {
        execute_tool(self, "voice_design", async {
            let mut vars = HashMap::new();
            vars.insert("character_description", character_description.as_str());
            let prompt = self
                .render_prompt("voice_design", &vars)
                .map_err(|e| McpToolError::internal(format!("Template render failed: {}", e)))?;

            let params = hkask_types::template::LLMParameters::default();
            let r = self
                .inference
                .generate_with_model(
                    &prompt,
                    &params,
                    Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
                    None,
                )
                .await
                .map_err(|e| {
                    McpToolError::unavailable(format!("Voice design inference failed: {}", e))
                })?;

            match serde_json::from_str::<serde_json::Value>(&r.text) {
                Ok(v) => Ok(serde_json::json!({
                    "voice_design": v,
                    "model": "llama-3.3-70b",
                })),
                Err(_) => Ok(serde_json::json!({
                    "voice_design": {"description": r.text.trim()},
                    "model": "llama-3.3-70b",
                    "warning": "LLM did not return valid JSON; using raw description."
                })),
            }
        })
        .await
    }

    #[tool(
        description = "Generate speech audio from text using a voice design. Returns audio as base64 data URI."
    )]
    async fn generate_speech(
        &self,
        Parameters(GenerateSpeechRequest { text, voice_design }): Parameters<GenerateSpeechRequest>,
    ) -> String {
        execute_tool(self, "generate_speech", async {
            let voice = if let Some(ref vd_json) = voice_design {
                match serde_json::from_str::<VoiceDesign>(vd_json) {
                    Ok(vd) => vd.to_elevenlabs_voice().to_string(),
                    Err(_) => "Rachel".to_string(),
                }
            } else {
                "Rachel".to_string()
            };

            self.inference
                .generate_speech(&text, &voice)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Speech generation failed: {}", e)))
        })
        .await
    }

    // ── Audio tools ─────────────────────────────────────────────────────────

    #[tool(
        description = "Transcribe speech audio to text. Returns transcribed text for REPL injection."
    )]
    async fn transcribe(
        &self,
        Parameters(TranscribeRequest {
            audio_url,
            language,
        }): Parameters<TranscribeRequest>,
    ) -> String {
        execute_tool(self, "transcribe", async {
            validate_tool_url(&audio_url)?;

            self.inference
                .transcribe(&audio_url, language.as_deref())
                .await
                .map_err(|e| McpToolError::unavailable(format!("Transcription failed: {}", e)))
        })
        .await
    }

    #[tool(
        description = "Transcribe audio and return a synchronized TranscriptBundle with word-level timings. Enables interactive highlighting and click-to-seek in frontends."
    )]
    async fn transcribe_bundle(
        &self,
        Parameters(TranscribeRequest {
            audio_url,
            language,
        }): Parameters<TranscribeRequest>,
    ) -> String {
        execute_tool(self, "transcribe_bundle", async {
            validate_tool_url(&audio_url)?;

            let raw = self
                .inference
                .transcribe(&audio_url, language.as_deref())
                .await
                .map_err(|e| McpToolError::unavailable(format!("Transcription failed: {}", e)))?;

            let full_text = raw
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let duration = raw.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0) as f32;
            let model = raw
                .get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());
            let words: Vec<TimedWord> = raw
                .get("words")
                .and_then(|w| w.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|w| {
                            Some(TimedWord {
                                word: w.get("word")?.as_str()?.to_string(),
                                start_ms: (w.get("start")?.as_f64()? * 1000.0) as u64,
                                end_ms: (w.get("end")?.as_f64()? * 1000.0) as u64,
                                confidence: w.get("confidence").and_then(|c| c.as_f64()),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let segments: Vec<TranscriptSegment> = raw
                .get("segments")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| {
                            Some(TranscriptSegment {
                                text: s.get("text")?.as_str()?.to_string(),
                                start_ms: (s.get("start")?.as_f64()? * 1000.0) as u64,
                                end_ms: (s.get("end")?.as_f64()? * 1000.0) as u64,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let bundle = TranscriptBundle {
                format: "hkask-transcript-v1".to_string(),
                audio_path: audio_url.clone(),
                audio_duration_secs: duration,
                full_text,
                words,
                segments,
                language: language.clone(),
                model,
            };

            Ok(serde_json::to_value(&bundle)
                .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize bundle"})))
        })
        .await
    }

    #[tool(
        description = "Capture audio from the default system microphone. Records to a WAV file optimized for Whisper transcription (16kHz mono)."
    )]
    async fn audio_capture(
        &self,
        Parameters(AudioCaptureRequest {
            duration_secs,
            output_path,
        }): Parameters<AudioCaptureRequest>,
    ) -> String {
        execute_tool(self, "audio_capture", async {
            if duration_secs <= 0.0 || duration_secs > 3600.0 {
                return Err(McpToolError::invalid_argument(
                    "duration_secs must be between 0.1 and 3600 (1 hour).",
                ));
            }

            self.require_ffmpeg()?;

            let path = self
                .ffmpeg
                .capture_audio(duration_secs, output_path.as_deref())
                .await
                .map_err(McpToolError::internal)?;

            Ok(serde_json::json!({
                "status": "captured",
                "duration_secs": duration_secs,
                "output": path.display().to_string(),
                "format": "wav",
                "sample_rate": 16000,
                "channels": 1,
            }))
        })
        .await
    }

    #[tool(
        description = "Record audio from microphone and transcribe it in one call. Returns linked audio file path and transcript. Use for meetings, notes, or any recording you want to keep."
    )]
    async fn record_and_transcribe(
        &self,
        Parameters(RecordAndTranscribeRequest {
            duration_secs,
            language,
        }): Parameters<RecordAndTranscribeRequest>,
    ) -> String {
        execute_tool(self, "record_and_transcribe", async {
            if duration_secs <= 0.0 || duration_secs > 3600.0 {
                return Err(McpToolError::invalid_argument(
                    "duration_secs must be between 0.1 and 3600 (1 hour).",
                ));
            }

            self.require_ffmpeg()?;

            let audio_path = self
                .ffmpeg
                .capture_audio(duration_secs, None)
                .await
                .map_err(|e| McpToolError::internal(format!("Audio capture failed: {}", e)))?;

            let audio_data = std::fs::read(&audio_path).map_err(|e| {
                McpToolError::internal(format!("Failed to read captured audio: {}", e))
            })?;
            let b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &audio_data);
            let audio_uri = format!("data:audio/wav;base64,{}", b64);

            let transcribe_result = self
                .inference
                .transcribe(&audio_uri, language.as_deref())
                .await
                .map_err(|e| McpToolError::unavailable(format!("Transcription failed: {}", e)));

            match transcribe_result {
                Ok(raw) => {
                    let full_text = raw
                        .get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();
                    let duration = raw
                        .get("duration")
                        .and_then(|d| d.as_f64())
                        .unwrap_or(duration_secs as f64) as f32;
                    let model = raw
                        .get("model")
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string());
                    let words: Vec<TimedWord> = raw
                        .get("words")
                        .and_then(|w| w.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|w| {
                                    Some(TimedWord {
                                        word: w.get("word")?.as_str()?.to_string(),
                                        start_ms: (w.get("start")?.as_f64()? * 1000.0) as u64,
                                        end_ms: (w.get("end")?.as_f64()? * 1000.0) as u64,
                                        confidence: w.get("confidence").and_then(|c| c.as_f64()),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let segments: Vec<TranscriptSegment> = raw
                        .get("segments")
                        .and_then(|s| s.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| {
                                    Some(TranscriptSegment {
                                        text: s.get("text")?.as_str()?.to_string(),
                                        start_ms: (s.get("start")?.as_f64()? * 1000.0) as u64,
                                        end_ms: (s.get("end")?.as_f64()? * 1000.0) as u64,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let audio_path_str = audio_path.display().to_string();
                    let bundle = TranscriptBundle {
                        format: "hkask-transcript-v1".to_string(),
                        audio_path: audio_path_str.clone(),
                        audio_duration_secs: duration,
                        full_text,
                        words,
                        segments,
                        language: language.clone(),
                        model,
                    };

                    Ok(serde_json::to_value(&bundle).unwrap_or_else(|_| {
                        serde_json::json!({"error": "Failed to serialize bundle"})
                    }))
                }
                Err(e) => Ok(serde_json::json!({
                    "status": "partial",
                    "duration_secs": duration_secs,
                    "audio_path": audio_path.display().to_string(),
                    "audio_format": "wav",
                    "sample_rate": 16000,
                    "channels": 1,
                    "transcript_error": e.to_json_string(),
                    "message": "Audio captured successfully but transcription failed. The audio file is saved and can be transcribed later."
                })),
            }
        })
        .await
    }

    // ── Generation tools ────────────────────────────────────────────────────

    #[tool(description = "Generate an image from a text prompt. Describe what you want to see.")]
    async fn generate_image(
        &self,
        Parameters(GenerateImageRequest {
            prompt,
            image_size,
            num_images,
        }): Parameters<GenerateImageRequest>,
    ) -> String {
        execute_tool(self, "generate_image", async {
            let size = image_size.clone();
            self.inference
                .generate_image(&prompt, size.as_deref(), num_images)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Image generation failed: {}", e)))
        })
        .await
    }

    #[tool(
        description = "Transform an existing image with a text prompt. Describe the change you want."
    )]
    async fn transform_image(
        &self,
        Parameters(TransformImageRequest {
            prompt,
            image_url,
            strength,
        }): Parameters<TransformImageRequest>,
    ) -> String {
        execute_tool(self, "transform_image", async {
            validate_tool_url(&image_url)?;
            self.inference
                .image_to_image(&image_url, &prompt, strength)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Image transform failed: {}", e)))
        })
        .await
    }

    #[tool(description = "Upscale an image to higher resolution.")]
    async fn upscale_image(
        &self,
        Parameters(UpscaleImageRequest { image_url, scale }): Parameters<UpscaleImageRequest>,
    ) -> String {
        execute_tool(self, "upscale_image", async {
            validate_tool_url(&image_url)?;
            self.inference
                .upscale(&image_url, scale)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Upscale failed: {}", e)))
        })
        .await
    }

    #[tool(
        description = "Generate a short video from a text prompt. Describe the scene you want to see in motion."
    )]
    async fn generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        execute_tool(self, "generate_video", async {
            self.inference
                .generate_video(&prompt, duration)
                .await
                .map_err(|e| McpToolError::unavailable(format!("Video generation failed: {}", e)))
        })
        .await
    }
}

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
