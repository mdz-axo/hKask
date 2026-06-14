//! hKask MCP Media — AI media generation (image, video, voice via centralized inference router)
//!
//! Tool families:
//! - Gallery: organize, search, status
//! - Image: describe, remove_background, apply_style, create_collage
//! - Video: clip, to_gif, image_to_video, add_caption, remix, concat, from_images
//! - Generation: generate_image, transform_image, upscale_image, generate_video
//! - Voice: voice_design, generate_speech

mod gallery;
mod templates;
mod video;

use gallery::{GalleryMode as LocalGalleryMode, GalleryState};
use hkask_inference::InferenceRouter;
use hkask_mcp::server::{McpToolError, ToolSpanGuard, validate_tool_url};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_storage::{GalleryMode, GalleryStore, GalleryStoreError};
use hkask_types::{
    InferencePort, McpErrorKind, TimedWord, TranscriptBundle, TranscriptSegment, VoiceDesign, WebID,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use video::FfmpegRunner;

pub struct MediaServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<DaemonClient>,
    /// Centralized inference router for ALL model calls (vision LLM + media generation)
    inference: Arc<InferenceRouter>,
    /// Active gallery state (None until gallery_set_root is called)
    gallery_state: Arc<Mutex<Option<GalleryState>>>,
    /// SQLite-backed gallery store for persistent indexing
    gallery_store: Arc<GalleryStore>,
    /// Jinja2 template environment for prompt rendering
    template_env: minijinja::Environment<'static>,
    /// ffmpeg runner for video processing (None if ffmpeg not found)
    ffmpeg: FfmpegRunner,
}

// ── Request types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateImageRequest {
    pub prompt: String,
    pub image_size: Option<String>,
    pub num_images: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TransformImageRequest {
    pub prompt: String,
    pub image_url: String,
    pub strength: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpscaleImageRequest {
    pub image_url: String,
    pub scale: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateVideoRequest {
    pub prompt: String,
    pub duration: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DescribeImageRequest {
    /// Image URL or gallery search result reference.
    pub image_url: String,
    /// Caption style: "descriptive", "artistic", "technical", "alt_text".
    pub style: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryOrganizeRequest {
    /// Absolute path to the gallery folder.
    pub path: String,
    /// Policy mode: "read-only", "copy-on-write", or "destructive".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Whether to scan subdirectories recursively (default: true).
    #[serde(default = "default_true")]
    pub recursive: bool,
}

fn default_mode() -> String {
    "read-only".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GallerySearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub tag_types: Option<Vec<String>>,
    pub min_similarity: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryAnalyzeRequest {
    /// Which images to analyze: "new" (untagged only), "all" (everything), or "selection" (specific indices).
    #[serde(default = "default_analyze_mode")]
    pub mode: String,
    /// Specific image indices (only when mode="selection").
    pub image_indices: Option<Vec<usize>>,
    /// Which pipelines to run: "faces", "objects", "colors", "composition", "scene". Default: all.
    pub pipelines: Option<Vec<String>>,
    /// Maximum images to process (safety limit, default: 50).
    #[serde(default = "default_analyze_limit")]
    pub max_images: usize,
}

fn default_analyze_mode() -> String {
    "new".to_string()
}
fn default_analyze_limit() -> usize {
    50
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryNameFaceRequest {
    /// The face group number (from analyze results).
    pub face_group: usize,
    /// Human-readable name for this person.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractObjectRequest {
    /// Gallery image index containing the object.
    pub image_index: usize,
    /// Description of the object to extract (e.g., "the golden retriever on the left").
    pub object_description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryTimelineRequest {
    /// Time period: "year", "month", or "decade".
    #[serde(default = "default_period")]
    pub period: String,
    /// How many periods to include (default: 5).
    #[serde(default = "default_count")]
    pub count: usize,
    /// Max images per period (default: 3).
    #[serde(default = "default_per_period")]
    pub per_period: usize,
    /// Optional search terms to filter by.
    pub search_terms: Option<Vec<String>>,
}

fn default_period() -> String {
    "year".to_string()
}
fn default_count() -> usize {
    5
}
fn default_per_period() -> usize {
    3
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveBackgroundRequest {
    pub image_index: usize,
    pub new_bg_color: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApplyStyleRequest {
    pub image_index: usize,
    pub style_prompt: String,
    pub strength: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateCollageRequest {
    pub search_terms: Option<Vec<String>>,
    pub similar_to_index: Option<usize>,
    pub image_indices: Option<Vec<usize>>,
    #[serde(default = "default_max_items")]
    pub max_items: usize,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default = "default_spacing")]
    pub spacing: u32,
    #[serde(default = "default_canvas")]
    pub canvas_size: String,
}

fn default_max_items() -> usize {
    6
}
fn default_layout() -> String {
    "grid".to_string()
}
fn default_spacing() -> u32 {
    8
}
fn default_canvas() -> String {
    "1200x900".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoClipRequest {
    pub video_url: String,
    pub start_sec: f32,
    pub end_sec: f32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoToGifRequest {
    pub video_url: String,
    pub start_sec: Option<f32>,
    pub duration_sec: Option<f32>,
    pub width: Option<u32>,
    pub fps: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImageToVideoRequest {
    pub image_index: usize,
    pub prompt: Option<String>,
    pub duration: Option<f32>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoAddCaptionRequest {
    pub video_url: String,
    pub text: String,
    pub position: Option<String>,
    pub font_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoRemixRequest {
    pub video_url: String,
    pub start_sec: f32,
    pub end_sec: f32,
    pub caption_text: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoFromImagesRequest {
    pub image_indices: Vec<usize>,
    pub fps: Option<u32>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoConcatRequest {
    pub video_urls: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VideoCaptionRequest {
    pub video_url: String,
    pub style: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoiceDesignRequest {
    pub character_description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateSpeechRequest {
    pub text: String,
    pub voice_design: Option<String>,
}

// ── Audio request types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TranscribeRequest {
    /// URL or base64 data URI of the audio to transcribe.
    pub audio_url: String,
    /// Optional ISO 639-1 language code (e.g., "en", "ja").
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AudioCaptureRequest {
    /// Duration to record in seconds (max 3600 = 1 hour).
    pub duration_secs: f32,
    /// Optional output path. Defaults to temp directory with UUID filename.
    pub output_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecordAndTranscribeRequest {
    /// Duration to record in seconds (max 3600 = 1 hour).
    pub duration_secs: f32,
    /// Optional ISO 639-1 language code for transcription.
    pub language: Option<String>,
}

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

    /// REQ: media-search-levenshtein-01 — identical strings return 1.0
    #[test]
    fn identical_strings() {
        assert!((levenshtein_similarity("sunset", "sunset") - 1.0).abs() < 0.001);
    }

    /// REQ: media-search-levenshtein-02 — completely different strings return low score
    #[test]
    fn completely_different() {
        let sim = levenshtein_similarity("sunset", "xyzzy");
        assert!(sim < 0.3, "expected low similarity, got {}", sim);
    }

    /// REQ: media-search-levenshtein-03 — case insensitive
    #[test]
    fn case_insensitive() {
        assert!((levenshtein_similarity("Sunset", "sunset") - 1.0).abs() < 0.001);
    }

    /// REQ: media-search-levenshtein-04 — typo-tolerant
    #[test]
    fn typo_tolerant() {
        let sim = levenshtein_similarity("sunset", "sunest");
        assert!(sim > 0.6, "expected high similarity for typo, got {}", sim);
    }

    /// REQ: media-search-levenshtein-05 — empty strings
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
        })
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool,
                "input": input_summary,
                "outcome": outcome,
                "detail": detail,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let tool_name = tool.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(&replicant, "mcp_session", "observed", &value, Some(0.85))
                    .await
                {
                    Ok(DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.media.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.media.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.media.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }

    /// Render a Jinja2 prompt template with the given variables.
    fn render_prompt(&self, name: &str, vars: &HashMap<&str, &str>) -> Result<String, String> {
        templates::render(&self.template_env, name, vars)
    }

    /// Resolve an image index to a base64 data URL for vision LLM calls.
    fn resolve_image_url(&self, image_index: usize) -> Result<String, String> {
        let guard = self.gallery_state.lock().unwrap();
        let state = guard
            .as_ref()
            .ok_or("No gallery initialized.".to_string())?;
        let gallery_id = state
            .gallery_id
            .as_ref()
            .ok_or("Gallery not persisted — run gallery_set_root first.".to_string())?;

        let img = self
            .gallery_store
            .get_image(gallery_id, Some(image_index), None)
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
        let guard = self.gallery_state.lock().unwrap();
        let state = guard
            .as_ref()
            .ok_or("No gallery initialized.".to_string())?;
        let gallery_id = state
            .gallery_id
            .as_ref()
            .ok_or("Gallery not persisted — run gallery_set_root first.".to_string())?;

        let img = self
            .gallery_store
            .get_image(gallery_id, Some(image_index), None)
            .map_err(|e| format!("Image not found at index {}: {}", image_index, e))?;

        Ok(PathBuf::from(&img.absolute_path))
    }

    /// Resolve an image index to its SQLite image ID for tag persistence.
    fn resolve_image_id(&self, image_index: usize) -> Result<String, String> {
        let guard = self.gallery_state.lock().unwrap();
        let state = guard
            .as_ref()
            .ok_or("No gallery initialized.".to_string())?;
        let gallery_id = state
            .gallery_id
            .as_ref()
            .ok_or("Gallery not persisted — run gallery_set_root first.".to_string())?;

        let img = self
            .gallery_store
            .get_image(gallery_id, Some(image_index), None)
            .map_err(|e| format!("Image not found at index {}: {}", image_index, e))?;

        Ok(img.id)
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
                tracing::debug!(target: "hkask.mcp.media.tags", image_id = %image_id, tag_type = %tag_type, value = %value, "Tag persisted")
            }
            Err(e) => {
                tracing::warn!(target: "hkask.mcp.media.tags", image_id = %image_id, tag_type = %tag_type, error = %e, "Failed to persist tag")
            }
        }
    }

    /// Resolve the best available vision model with fallback chain.
    /// Tries: DeepInfra → Fireworks → Ollama (local).
    /// Returns the model name and a label for recording.
    async fn resolve_vision_model(&self) -> (&'static str, &'static str) {
        let models = self.inference.list_vision_models().await;

        for model in &models {
            match model.provider {
                hkask_inference::ProviderId::DeepInfra => {
                    return (
                        "DI/meta-llama/Llama-3.2-11B-Vision-Instruct",
                        "llama-3.2-vision",
                    );
                }
                hkask_inference::ProviderId::Fireworks => {
                    return ("FW/llama-v3p1-70b-instruct", "llama-3.1-vision");
                }
                hkask_inference::ProviderId::Ollama => return ("OM/llava:13b", "llava"),
                _ => continue,
            }
        }

        // Fallback: try DeepInfra anyway (will error if unavailable)
        (
            "DI/meta-llama/Llama-3.2-11B-Vision-Instruct",
            "llama-3.2-vision",
        )
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
        }): Parameters<GalleryOrganizeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_organize", &self.webid);

        let gallery_mode = match mode.as_str() {
            "read-only" => GalleryMode::ReadOnly,
            "copy-on-write" => GalleryMode::CopyOnWrite,
            "destructive" => GalleryMode::Destructive,
            other => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Invalid mode '{}': must be read-only, copy-on-write, or destructive",
                        other
                    ))
                    .to_json_string(),
                );
            }
        };

        // Create gallery in SQLite
        let record = match self.gallery_store.create(&path, gallery_mode.clone()) {
            Ok(r) => r,
            Err(GalleryStoreError::AlreadyExists(_)) => {
                // Gallery exists — just scan
                let guard = self.gallery_state.lock().unwrap();
                if let Some(ref state) = *guard {
                    if let Some(ref gid) = state.gallery_id {
                        let gid_clone = gid.clone();
                        let mut state_clone = state.clone();
                        drop(guard);
                        let scan_result = state_clone.scan(recursive, None);
                        let mut persisted = 0u32;
                        for entry in &scan_result.entries {
                            let abs_path = state_clone.path.join(&entry.relative_path);
                            if self
                                .gallery_store
                                .add_image(
                                    &gid_clone,
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
                        *self.gallery_state.lock().unwrap() = Some(state_clone);
                        return span.ok_json(serde_json::json!({
                            "status": "rescanned",
                            "gallery_id": gid_clone,
                            "root_path": path,
                            "mode": mode,
                            "images_added": scan_result.added,
                            "total_images": scan_result.total,
                            "persisted": persisted,
                        }));
                    }
                }
                return span.ok_json(serde_json::json!({
                    "status": "already_exists",
                    "message": "A gallery already exists at this path but its state was lost. Please restart the server."
                }));
            }
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to create gallery: {}", e))
                        .to_json_string(),
                );
            }
        };

        // Set up in-memory GalleryState
        let local_mode = match gallery_mode {
            GalleryMode::ReadOnly | GalleryMode::CopyOnWrite => LocalGalleryMode::Original,
            GalleryMode::Destructive => LocalGalleryMode::Copy,
        };
        let mut state = GalleryState::new(PathBuf::from(&path), local_mode);
        if let Err(e) = state.validate() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            );
        }
        if let Err(e) = state.ensure_meta_dir() {
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            );
        }
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

        *self.gallery_state.lock().unwrap() = Some(state);

        span.ok_json(serde_json::json!({
            "status": "organized",
            "gallery_id": record.id,
            "root_path": record.root_path,
            "mode": record.mode,
            "images_found": scan_result.added,
            "total_images": scan_result.total,
            "persisted": persisted,
            "message": "Gallery ready. Use gallery_search to find photos by content."
        }))
    }

    #[tool(description = "Get gallery status: path, mode, image count, and total size.")]
    async fn gallery_status(&self) -> String {
        let span = ToolSpanGuard::new("gallery_status", &self.webid);
        let guard = self.gallery_state.lock().unwrap();
        match &*guard {
            Some(state) => span.ok_json(state.summary()),
            None => span.ok_json(serde_json::json!({
                "status": "no_gallery",
                "message": "No gallery organized. Use gallery_organize to point at a photo folder."
            })),
        }
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
        let span = ToolSpanGuard::new("gallery_search", &self.webid);

        let guard = self.gallery_state.lock().unwrap();
        let state = match &*guard {
            Some(s) => s,
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(
                        "No gallery organized. Use gallery_organize first.",
                    )
                    .to_json_string(),
                );
            }
        };

        let gallery_id = match &state.gallery_id {
            Some(id) => id.clone(),
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("Gallery not persisted.").to_json_string(),
                );
            }
        };

        let all_tags = match self.gallery_store.get_all_tags(&gallery_id) {
            Ok(tags) => tags,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to query tags: {}", e)).to_json_string(),
                );
            }
        };

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

        span.ok_json(serde_json::json!({
            "query": query,
            "results": results,
            "total_matches": results.len(),
        }))
    }

    // ── Image tools ──────────────────────────────────────────────────────────

    #[tool(
        description = "Describe an image in detail. Choose a style: descriptive (full scene), artistic (poetic), technical (photographic analysis), or alt_text (accessibility)."
    )]
    async fn describe_image(
        &self,
        Parameters(DescribeImageRequest { image_url, style }): Parameters<DescribeImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("describe_image", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }

        let style_str = style.as_deref().unwrap_or("descriptive");
        let mut vars = HashMap::new();
        vars.insert("style", style_str);
        let prompt = match self.render_prompt("caption", &vars) {
            Ok(p) => p,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Template render failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        let (vision_model, _vision_label) = self.resolve_vision_model().await;
        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(&prompt, &[image_url], &params, Some(vision_model))
            .await
            .map_err(|e| McpToolError::unavailable(format!("Vision inference failed: {}", e)));

        match result {
            Ok(r) => {
                span.ok_json(serde_json::json!({"description": r.text.trim(), "style": style_str}))
            }
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
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
        let span = ToolSpanGuard::new("gallery_analyze", &self.webid);

        // Extract gallery state in a block so the MutexGuard is dropped before any await
        let (image_count, _gallery_id) = {
            let guard = self.gallery_state.lock().unwrap();
            let state = match &*guard {
                Some(s) => s,
                None => {
                    return span.error(
                        McpErrorKind::InvalidArgument,
                        McpToolError::invalid_argument(
                            "No gallery organized. Use gallery_organize first.",
                        )
                        .to_json_string(),
                    );
                }
            };
            let gid = match &state.gallery_id {
                Some(id) => id.clone(),
                None => {
                    return span.error(
                        McpErrorKind::InvalidArgument,
                        McpToolError::invalid_argument("Gallery not persisted.").to_json_string(),
                    );
                }
            };
            (state.image_count, gid)
        };

        // Determine which images to process
        let indices: Vec<usize> = match mode.as_str() {
            "selection" => image_indices.unwrap_or_default(),
            "all" => (0..image_count as usize).collect(),
            _ => (0..image_count as usize).collect(),
        };

        let indices: Vec<usize> = indices.into_iter().take(max_images).collect();
        if indices.is_empty() {
            return span.ok_json(serde_json::json!({
                "status": "nothing_to_analyze",
                "message": "No images to analyze."
            }));
        }

        // Determine which pipelines to run
        let all_pipelines: Vec<String> = vec!["faces", "objects", "colors", "composition", "scene"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let pipelines = pipelines.unwrap_or(all_pipelines);
        let run_faces = pipelines.contains(&"faces".to_string());
        let run_objects = pipelines.contains(&"objects".to_string());
        let run_colors = pipelines.contains(&"colors".to_string());
        let run_composition = pipelines.contains(&"composition".to_string());
        let run_scene = pipelines.contains(&"scene".to_string());

        let (vision_model, vision_label) = self.resolve_vision_model().await;
        let params = hkask_types::LLMParameters::default();

        let mut analyzed = 0u32;
        let mut errors = Vec::new();

        for idx in &indices {
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

            // Run each pipeline
            if run_faces {
                if let Ok(prompt) = self.render_prompt(
                    "tag_faces",
                    &[("detail_level", "detailed")].into_iter().collect(),
                ) {
                    if let Ok(r) = self
                        .inference
                        .generate_vision(&prompt, &[image_url.clone()], &params, Some(vision_model))
                        .await
                    {
                        if let Ok(faces) = serde_json::from_str::<Vec<serde_json::Value>>(&r.text) {
                            for face in &faces {
                                let value = serde_json::to_string(face).unwrap_or_default();
                                self.persist_tag(&image_id, "face", &value, 0.85, vision_label);
                            }
                        } else {
                            self.persist_tag(&image_id, "face", r.text.trim(), 0.7, vision_label);
                        }
                    }
                }
            }

            if run_objects {
                let mut vars = HashMap::new();
                vars.insert("detail_level", "detailed");
                let max_str = "20".to_string();
                vars.insert("max_objects", &max_str);
                if let Ok(prompt) = self.render_prompt("tag_objects", &vars) {
                    if let Ok(r) = self
                        .inference
                        .generate_vision(&prompt, &[image_url.clone()], &params, Some(vision_model))
                        .await
                    {
                        if let Ok(objects) = serde_json::from_str::<Vec<serde_json::Value>>(&r.text)
                        {
                            for obj in &objects {
                                let value = serde_json::to_string(obj).unwrap_or_default();
                                self.persist_tag(&image_id, "object", &value, 0.85, vision_label);
                            }
                        } else {
                            self.persist_tag(&image_id, "object", r.text.trim(), 0.7, vision_label);
                        }
                    }
                }
            }

            if run_colors {
                let mut vars = HashMap::new();
                vars.insert("max_colors", "8");
                if let Ok(prompt) = self.render_prompt("tag_colors", &vars) {
                    if let Ok(r) = self
                        .inference
                        .generate_vision(&prompt, &[image_url.clone()], &params, Some(vision_model))
                        .await
                    {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&r.text) {
                            if let Some(colors) = parsed["colors"].as_array() {
                                for color in colors {
                                    let value = serde_json::to_string(color).unwrap_or_default();
                                    self.persist_tag(
                                        &image_id,
                                        "color",
                                        &value,
                                        0.85,
                                        vision_label,
                                    );
                                }
                            }
                            for field in &["palette_style", "temperature", "saturation"] {
                                if let Some(v) = parsed.get(*field).and_then(|v| v.as_str()) {
                                    self.persist_tag(&image_id, "color", v, 0.9, vision_label);
                                }
                            }
                        } else {
                            self.persist_tag(&image_id, "color", r.text.trim(), 0.7, vision_label);
                        }
                    }
                }
            }

            if run_composition {
                if let Ok(prompt) = self.render_prompt("tag_composition", &HashMap::new()) {
                    if let Ok(r) = self
                        .inference
                        .generate_vision(&prompt, &[image_url.clone()], &params, Some(vision_model))
                        .await
                    {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&r.text) {
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
                                    self.persist_tag(
                                        &image_id,
                                        "composition",
                                        v,
                                        0.85,
                                        vision_label,
                                    );
                                }
                            }
                        } else {
                            self.persist_tag(
                                &image_id,
                                "composition",
                                r.text.trim(),
                                0.7,
                                vision_label,
                            );
                        }
                    }
                }
            }

            if run_scene {
                let mut vars = HashMap::new();
                vars.insert("style", "descriptive");
                if let Ok(prompt) = self.render_prompt("caption", &vars) {
                    if let Ok(r) = self
                        .inference
                        .generate_vision(&prompt, &[image_url.clone()], &params, Some(vision_model))
                        .await
                    {
                        self.persist_tag(&image_id, "caption", r.text.trim(), 0.9, vision_label);
                    }
                }
            }

            analyzed += 1;
        }

        span.ok_json(serde_json::json!({
            "status": "complete",
            "images_analyzed": analyzed,
            "total_images": indices.len(),
            "pipelines_run": pipelines,
            "model": vision_label,
            "errors": errors,
        }))
    }

    #[tool(
        description = "Name a face group from gallery_analyze. After naming, gallery_search can find photos of that person by name."
    )]
    async fn gallery_name_face(
        &self,
        Parameters(GalleryNameFaceRequest { face_group, name }): Parameters<GalleryNameFaceRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_name_face", &self.webid);

        let guard = self.gallery_state.lock().unwrap();
        let state = match &*guard {
            Some(s) => s,
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("No gallery organized.").to_json_string(),
                );
            }
        };
        let gallery_id = match &state.gallery_id {
            Some(id) => id.clone(),
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("Gallery not persisted.").to_json_string(),
                );
            }
        };
        drop(guard);

        // Find all face tags and update those matching the group
        let all_tags = match self.gallery_store.get_all_tags(&gallery_id) {
            Ok(t) => t,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to query tags: {}", e)).to_json_string(),
                );
            }
        };

        let mut renamed = 0u32;
        for (tag, _path) in &all_tags {
            if tag.tag_type != "face" {
                continue;
            }
            // Face tags store face_index in the value — match by group
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&tag.value) {
                if parsed["face_index"].as_u64() == Some(face_group as u64) {
                    // Re-persist with name added
                    let new_value = serde_json::json!({
                        "face_index": face_group,
                        "name": name,
                    });
                    self.persist_tag(&tag.image_id, "face", &new_value.to_string(), 1.0, "user");
                    renamed += 1;
                }
            }
        }

        span.ok_json(serde_json::json!({
            "status": "named",
            "face_group": face_group,
            "name": name,
            "images_updated": renamed,
        }))
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
        let span = ToolSpanGuard::new("extract_object", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        // Use Florence-2 segmentation via fal.ai
        let result = self
            .inference
            .segment_object(&image_url, &object_description)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Object extraction failed: {}", e)));

        self.record_experience(
            "extract_object",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"object": object_description}),
        );

        span.finish(result)
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
        let span = ToolSpanGuard::new("gallery_timeline", &self.webid);

        let guard = self.gallery_state.lock().unwrap();
        let state = match &*guard {
            Some(s) => s.clone(),
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("No gallery organized.").to_json_string(),
                );
            }
        };
        drop(guard);

        let gallery_id = match &state.gallery_id {
            Some(id) => id.clone(),
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("Gallery not persisted.").to_json_string(),
                );
            }
        };

        // Collect images with their dates
        let mut dated_images: Vec<(String, String)> = Vec::new(); // (period_key, relative_path)
        for idx in 0..state.image_count as usize {
            let img = match self.gallery_store.get_image(&gallery_id, Some(idx), None) {
                Ok(i) => i,
                Err(_) => continue,
            };

            // Apply search filter if specified
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

            // Extract date from EXIF
            let exif = Self::extract_exif(&img.absolute_path);
            let date_str = exif
                .get("date_taken")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let period_key = match period.as_str() {
                "month" => date_str.chars().take(7).collect(), // "2024-03"
                "decade" => format!("{}0s", &date_str[..3]),   // "2020s"
                _ => date_str.chars().take(4).collect(),       // "2024"
            };

            dated_images.push((period_key, img.relative_path));
        }

        // Group by period and take top N per period
        let mut periods: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for (key, path) in &dated_images {
            periods.entry(key.clone()).or_default().push(path.clone());
        }

        // Take last N periods, per_period images each
        let mut result_periods: Vec<serde_json::Value> = Vec::new();
        for (key, images) in periods.iter().rev().take(count) {
            let selected: Vec<&String> = images.iter().take(per_period).collect();
            result_periods.push(serde_json::json!({
                "period": key,
                "total_images": images.len(),
                "images": selected,
            }));
        }

        span.ok_json(serde_json::json!({
            "period_type": period,
            "periods": result_periods,
        }))
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
        let span = ToolSpanGuard::new("image_remove_background", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        // Route through centralized inference router
        let result = self
            .inference
            .remove_background(&image_url)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Background removal failed: {}", e)));

        self.record_experience(
            "image_remove_background",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({}),
        );
        span.finish(result)
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
        let span = ToolSpanGuard::new("image_apply_style", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let result = self
            .inference
            .image_to_image(&image_url, &style_prompt, strength)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Style transfer failed: {}", e)));
        self.record_experience(
            "image_apply_style",
            &style_prompt,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"strength": strength}),
        );
        span.finish(result)
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
        let span = ToolSpanGuard::new("image_create_collage", &self.webid);

        // Validate mutual exclusivity: exactly one mode must be active
        let mode_count = search_terms.is_some() as u8
            + similar_to_index.is_some() as u8
            + image_indices.is_some() as u8;
        if mode_count == 0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "Must specify one of: search_terms, similar_to_index, or image_indices.",
                )
                .to_json_string(),
            );
        }
        if mode_count > 1 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "search_terms, similar_to_index, and image_indices are mutually exclusive. Choose one.",
                )
                .to_json_string(),
            );
        }

        // Get gallery state
        let guard = self.gallery_state.lock().unwrap();
        let state = match &*guard {
            Some(s) => s,
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("No gallery initialized.").to_json_string(),
                );
            }
        };
        let gallery_id = match &state.gallery_id {
            Some(id) => id.clone(),
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(
                        "Gallery not persisted — run gallery_set_root first.",
                    )
                    .to_json_string(),
                );
            }
        };
        let gallery_root = state.path.clone();
        drop(guard); // release lock before long operations

        // Resolve image paths based on mode
        let mut paths = Vec::new();

        if let Some(ref terms) = search_terms {
            // ── Search mode: find images matching search terms via tag similarity ──
            let all_tags = match self.gallery_store.get_all_tags(&gallery_id) {
                Ok(tags) => tags,
                Err(e) => {
                    return span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(format!("Failed to query tags: {}", e))
                            .to_json_string(),
                    );
                }
            };

            let mut image_scores: HashMap<String, f64> = HashMap::new();
            for (tag, relative_path) in &all_tags {
                for term in terms {
                    let sim = levenshtein_similarity(term, &tag.value);
                    if sim >= 0.3 {
                        let weighted = sim * tag.confidence;
                        let entry = image_scores.entry(relative_path.clone()).or_insert(0.0);
                        *entry = entry.max(weighted);
                    }
                }
            }

            let mut ranked: Vec<(String, f64)> = image_scores.into_iter().collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ranked.truncate(max_items);

            for (rel_path, _score) in &ranked {
                paths.push(gallery_root.join(rel_path));
            }
        } else if let Some(ref_idx) = similar_to_index {
            // ── Similar mode: find images with tags similar to the reference image ──
            let ref_path = match self.resolve_image_path(ref_idx) {
                Ok(p) => p,
                Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
            };
            let ref_image_id = match self.resolve_image_id(ref_idx) {
                Ok(id) => id,
                Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
            };
            let ref_tags = match self.gallery_store.get_tags(&ref_image_id) {
                Ok(tags) => tags,
                Err(e) => {
                    return span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(format!("Failed to get reference tags: {}", e))
                            .to_json_string(),
                    );
                }
            };

            let all_tags = match self.gallery_store.get_all_tags(&gallery_id) {
                Ok(tags) => tags,
                Err(e) => {
                    return span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(format!("Failed to query tags: {}", e))
                            .to_json_string(),
                    );
                }
            };

            let mut image_scores: HashMap<String, f64> = HashMap::new();
            for (tag, relative_path) in &all_tags {
                let abs_path = gallery_root.join(relative_path);
                if abs_path == ref_path {
                    continue; // skip the reference image itself
                }
                for ref_tag in &ref_tags {
                    let sim = levenshtein_similarity(&ref_tag.value, &tag.value);
                    if sim >= 0.3 {
                        let weighted = sim * tag.confidence;
                        let entry = image_scores.entry(relative_path.clone()).or_insert(0.0);
                        *entry = entry.max(weighted);
                    }
                }
            }

            let mut ranked: Vec<(String, f64)> = image_scores.into_iter().collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ranked.truncate(max_items.saturating_sub(1)); // reserve spot for reference

            // Reference image first, then similar images
            paths.push(ref_path);
            for (rel_path, _score) in &ranked {
                paths.push(gallery_root.join(rel_path));
            }
        } else if let Some(ref indices) = image_indices {
            // ── Explicit mode: use provided indices ──
            if indices.is_empty() {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("At least one image index is required.")
                        .to_json_string(),
                );
            }
            if indices.len() > 9 {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument("Maximum 9 images supported for collage.")
                        .to_json_string(),
                );
            }
            let limit = indices.len().min(max_items);
            for idx in indices.iter().take(limit) {
                match self.resolve_image_path(*idx) {
                    Ok(p) => paths.push(p),
                    Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
                }
            }
        }

        if paths.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("No images found for collage.").to_json_string(),
            );
        }

        // Load all images
        let mut images = Vec::new();
        for path in &paths {
            match image::open(path) {
                Ok(img) => images.push(img),
                Err(e) => {
                    return span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(format!("Failed to open {}: {}", path.display(), e))
                            .to_json_string(),
                    );
                }
            }
        }

        // Compute grid dimensions
        let cols = match layout.as_str() {
            "horizontal" => images.len() as u32,
            "vertical" => 1u32,
            "masonry" => 3u32.min(images.len() as u32),
            _ => {
                // grid: auto-compute columns for roughly square layout
                (images.len() as f64).sqrt().ceil() as u32
            }
        };
        let rows = (images.len() as u32 + cols - 1) / cols;

        // Parse canvas size
        let parts: Vec<&str> = canvas_size.split('x').collect();
        let canvas_w: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1200);
        let canvas_h: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(900);

        let cell_w = (canvas_w - spacing * (cols + 1)) / cols;
        let cell_h = (canvas_h - spacing * (rows + 1)) / rows;

        // Create canvas
        let mut canvas = image::DynamicImage::new_rgba8(canvas_w, canvas_h);
        let bg = image::Rgba([30u8, 30u8, 30u8, 255u8]);
        for pixel in canvas.as_mut_rgba8().unwrap().pixels_mut() {
            *pixel = bg;
        }

        // Place images on grid
        for (i, img) in images.iter().enumerate() {
            let col = i as u32 % cols;
            let row = i as u32 / cols;

            // Resize image to fit cell while preserving aspect ratio
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

        // Save to temp directory
        let temp_dir = std::env::temp_dir().join("hkask-media");
        let _ = std::fs::create_dir_all(&temp_dir);
        let output_path = temp_dir.join(format!("collage_{}.png", uuid::Uuid::new_v4()));

        match canvas.save(&output_path) {
            Ok(_) => span.ok_json(serde_json::json!({
                "status": "created",
                "image_count": images.len(),
                "layout": layout,
                "cols": cols,
                "rows": rows,
                "canvas_width": canvas_w,
                "canvas_height": canvas_h,
                "spacing": spacing,
                "output": output_path.display().to_string(),
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to save collage: {}", e)).to_json_string(),
            ),
        }
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
        let span = ToolSpanGuard::new("video_clip", &self.webid);
        if let Err(e) = validate_tool_url(&video_url) {
            return span.error(e.kind, e.to_json_string());
        }

        if start_sec >= end_sec {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("start_sec must be less than end_sec.")
                    .to_json_string(),
            );
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(
                    "ffmpeg not found on system PATH — video tools unavailable.",
                )
                .to_json_string(),
            );
        }

        match self.ffmpeg.clip(&video_url, start_sec, end_sec).await {
            Ok(output) => span.ok_json(serde_json::json!({
                "status": "clipped",
                "source": video_url,
                "start_sec": start_sec,
                "end_sec": end_sec,
                "duration": end_sec - start_sec,
                "output": output.display().to_string(),
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
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
        let span = ToolSpanGuard::new("video_to_gif", &self.webid);
        if let Err(e) = validate_tool_url(&video_url) {
            return span.error(e.kind, e.to_json_string());
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found on system PATH.").to_json_string(),
            );
        }

        let start = start_sec.unwrap_or(0.0);
        let dur = duration_sec.unwrap_or(5.0);
        let w = width.unwrap_or(480);
        let f = fps.unwrap_or(10);

        match self.ffmpeg.to_gif(&video_url, start, dur, w, f).await {
            Ok(output) => span.ok_json(serde_json::json!({
                "status": "converted",
                "source": video_url,
                "start_sec": start,
                "duration_sec": dur,
                "width": w,
                "fps": f,
                "output": output.display().to_string(),
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
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
            model,
        }): Parameters<ImageToVideoRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("image_to_video", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let result = self
            .inference
            .image_to_video(&image_url, prompt.as_deref(), duration)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Image-to-video failed: {}", e)));
        self.record_experience(
            "image_to_video",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"model": model, "duration": duration}),
        );
        span.finish(result)
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
        let span = ToolSpanGuard::new("video_add_caption", &self.webid);
        if let Err(e) = validate_tool_url(&video_url) {
            return span.error(e.kind, e.to_json_string());
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found on system PATH.").to_json_string(),
            );
        }

        let pos = position.as_deref().unwrap_or("bottom");
        let size = font_size.unwrap_or(24);

        match self.ffmpeg.add_caption(&video_url, &text, pos, size).await {
            Ok(output) => span.ok_json(serde_json::json!({
                "status": "captioned",
                "source": video_url,
                "text": text,
                "position": pos,
                "font_size": size,
                "output": output.display().to_string(),
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
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
        let span = ToolSpanGuard::new("video_remix", &self.webid);
        if let Err(e) = validate_tool_url(&video_url) {
            return span.error(e.kind, e.to_json_string());
        }

        if start_sec >= end_sec {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("start_sec must be less than end_sec.")
                    .to_json_string(),
            );
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found on system PATH.").to_json_string(),
            );
        }

        // Step 1: clip
        let clipped = match self.ffmpeg.clip(&video_url, start_sec, end_sec).await {
            Ok(p) => p,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Clip step failed: {}", e)).to_json_string(),
                );
            }
        };

        // Step 2: add caption (if text provided)
        let captioned = if let Some(ref cap) = caption_text {
            match self
                .ffmpeg
                .add_caption(&clipped.to_string_lossy(), cap, "bottom", 24)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    return span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(format!("Caption step failed: {}", e))
                            .to_json_string(),
                    );
                }
            }
        } else {
            clipped.clone()
        };

        // Step 3: convert to GIF
        match self
            .ffmpeg
            .to_gif(
                &captioned.to_string_lossy(),
                0.0,
                end_sec - start_sec,
                480,
                10,
            )
            .await
        {
            Ok(gif) => {
                // Clean up intermediate files
                let _ = std::fs::remove_file(&clipped);
                if caption_text.is_some() {
                    let _ = std::fs::remove_file(&captioned);
                }
                span.ok_json(serde_json::json!({
                    "status": "remixed",
                    "source": video_url,
                    "start_sec": start_sec,
                    "end_sec": end_sec,
                    "caption": caption_text,
                    "output": gif.display().to_string(),
                }))
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("GIF step failed: {}", e)).to_json_string(),
            ),
        }
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
        let span = ToolSpanGuard::new("video_from_images", &self.webid);

        if image_indices.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("At least one image index is required.")
                    .to_json_string(),
            );
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found on system PATH.").to_json_string(),
            );
        }

        // Resolve all image paths
        let mut paths = Vec::new();
        for idx in &image_indices {
            match self.resolve_image_path(*idx) {
                Ok(p) => paths.push(p),
                Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
            }
        }

        let fps = fps.unwrap_or(24);
        let fmt = format.as_deref().unwrap_or("mp4");

        match self.ffmpeg.images_to_video(&paths, fps, fmt).await {
            Ok(output) => span.ok_json(serde_json::json!({
                "status": "created",
                "frame_count": paths.len(),
                "fps": fps,
                "format": fmt,
                "output": output.display().to_string(),
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Concatenate multiple video clips into one using ffmpeg.")]
    async fn video_concat(
        &self,
        Parameters(VideoConcatRequest { video_urls }): Parameters<VideoConcatRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("video_concat", &self.webid);

        if video_urls.len() < 2 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("At least 2 video URLs are required.")
                    .to_json_string(),
            );
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found on system PATH.").to_json_string(),
            );
        }

        match self.ffmpeg.concat(&video_urls).await {
            Ok(output) => span.ok_json(serde_json::json!({
                "status": "concatenated",
                "clip_count": video_urls.len(),
                "output": output.display().to_string(),
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Generate a description of video content by extracting keyframes and analyzing them with a vision LLM."
    )]
    async fn video_caption(
        &self,
        Parameters(VideoCaptionRequest { video_url, style }): Parameters<VideoCaptionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("video_caption", &self.webid);

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found on system PATH.").to_json_string(),
            );
        }

        // Extract keyframes (1 frame per 2 seconds, max 10 frames)
        let frames = match self.ffmpeg.extract_keyframes(&video_url, 2.0, 10).await {
            Ok(f) => f,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Keyframe extraction failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        if frames.is_empty() {
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal("No keyframes extracted from video.").to_json_string(),
            );
        }

        // Encode frames as base64 for vision LLM
        let mut image_urls = Vec::new();
        for frame in &frames {
            match std::fs::read(frame) {
                Ok(data) => {
                    let b64 =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                    image_urls.push(format!("data:image/jpeg;base64,{}", b64));
                }
                Err(e) => {
                    tracing::warn!(target: "hkask.mcp.media", frame = %frame.display(), error = %e, "Failed to read keyframe");
                }
            }
        }

        let style_str = style.as_deref().unwrap_or("descriptive");
        let mut vars = HashMap::new();
        vars.insert("style", style_str);
        let prompt = match self.render_prompt("video_caption", &vars) {
            Ok(p) => p,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Template render failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        let (vision_model, _vision_label) = self.resolve_vision_model().await;
        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(&prompt, &image_urls, &params, Some(vision_model))
            .await;

        // Clean up temp frames
        for frame in &frames {
            let _ = std::fs::remove_file(frame);
        }

        match result {
            Ok(r) => span.ok_json(serde_json::json!({
                "caption": r.text.trim(),
                "style": style_str,
                "frames_analyzed": image_urls.len(),
            })),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
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
        let span = ToolSpanGuard::new("voice_design", &self.webid);

        let mut vars = HashMap::new();
        vars.insert("character_description", character_description.as_str());
        let prompt = match self.render_prompt("voice_design", &vars) {
            Ok(p) => p,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Template render failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        let params = hkask_types::LLMParameters::default();
        // Use Llama 3.3 70B for structured JSON voice design generation
        let result = self
            .inference
            .generate_with_model(
                &prompt,
                &params,
                Some("DI/meta-llama/Llama-3.3-70B-Instruct"),
            )
            .await
            .map_err(|e| {
                McpToolError::unavailable(format!("Voice design inference failed: {}", e))
            });

        self.record_experience(
            "voice_design",
            &character_description,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({}),
        );

        match result {
            Ok(r) => {
                // Validate that the response is valid JSON
                match serde_json::from_str::<serde_json::Value>(&r.text) {
                    Ok(v) => span.ok_json(serde_json::json!({
                        "voice_design": v,
                        "model": "llama-3.3-70b",
                    })),
                    Err(_) => span.ok_json(serde_json::json!({
                        "voice_design": {"description": r.text.trim()},
                        "model": "llama-3.3-70b",
                        "warning": "LLM did not return valid JSON; using raw description."
                    })),
                }
            }
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(
        description = "Generate speech audio from text using a voice design. Returns audio as base64 data URI."
    )]
    async fn generate_speech(
        &self,
        Parameters(GenerateSpeechRequest { text, voice_design }): Parameters<GenerateSpeechRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("generate_speech", &self.webid);

        // Resolve voice preset from VoiceDesign or use default
        let voice = if let Some(ref vd_json) = voice_design {
            match serde_json::from_str::<VoiceDesign>(vd_json) {
                Ok(vd) => vd.to_elevenlabs_voice().to_string(),
                Err(_) => "Rachel".to_string(),
            }
        } else {
            "Rachel".to_string()
        };

        let result = self
            .inference
            .generate_speech(&text, &voice)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Speech generation failed: {}", e)));

        self.record_experience(
            "generate_speech",
            &text,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"voice": voice}),
        );

        span.finish(result)
    }

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
        let span = ToolSpanGuard::new("transcribe", &self.webid);
        if let Err(e) = validate_tool_url(&audio_url) {
            return span.error(e.kind, e.to_json_string());
        }

        let result = self
            .inference
            .transcribe(&audio_url, language.as_deref())
            .await
            .map_err(|e| McpToolError::unavailable(format!("Transcription failed: {}", e)));

        self.record_experience(
            "transcribe",
            &format!("audio_url={}", audio_url),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"language": language}),
        );

        span.finish(result)
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
        let span = ToolSpanGuard::new("transcribe_bundle", &self.webid);
        if let Err(e) = validate_tool_url(&audio_url) {
            return span.error(e.kind, e.to_json_string());
        }

        let result = self
            .inference
            .transcribe(&audio_url, language.as_deref())
            .await
            .map_err(|e| McpToolError::unavailable(format!("Transcription failed: {}", e)));

        match result {
            Ok(raw) => {
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

                self.record_experience(
                    "transcribe_bundle",
                    &format!("audio_url={}", audio_url),
                    "success",
                    serde_json::json!({"word_count": bundle.word_count()}),
                );

                span.ok_json(
                    serde_json::to_value(&bundle).unwrap_or_else(
                        |_| serde_json::json!({"error": "Failed to serialize bundle"}),
                    ),
                )
            }
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
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
        let span = ToolSpanGuard::new("audio_capture", &self.webid);

        if duration_secs <= 0.0 || duration_secs > 3600.0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "duration_secs must be between 0.1 and 3600 (1 hour).",
                )
                .to_json_string(),
            );
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found — audio capture unavailable.")
                    .to_json_string(),
            );
        }

        match self
            .ffmpeg
            .capture_audio(duration_secs, output_path.as_deref())
            .await
        {
            Ok(path) => {
                self.record_experience(
                    "audio_capture",
                    &format!("duration={}s", duration_secs),
                    "success",
                    serde_json::json!({"output": path.display().to_string()}),
                );
                span.ok_json(serde_json::json!({
                    "status": "captured",
                    "duration_secs": duration_secs,
                    "output": path.display().to_string(),
                    "format": "wav",
                    "sample_rate": 16000,
                    "channels": 1,
                }))
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
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
        let span = ToolSpanGuard::new("record_and_transcribe", &self.webid);

        if duration_secs <= 0.0 || duration_secs > 3600.0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "duration_secs must be between 0.1 and 3600 (1 hour).",
                )
                .to_json_string(),
            );
        }

        if !self.ffmpeg.available {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("ffmpeg not found — audio capture unavailable.")
                    .to_json_string(),
            );
        }

        // Step 1: capture audio
        let audio_path = match self.ffmpeg.capture_audio(duration_secs, None).await {
            Ok(p) => p,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Audio capture failed: {}", e)).to_json_string(),
                );
            }
        };

        // Step 2: read audio file and encode as base64 data URI
        let audio_data = match std::fs::read(&audio_path) {
            Ok(d) => d,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to read captured audio: {}", e))
                        .to_json_string(),
                );
            }
        };
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &audio_data);
        let audio_uri = format!("data:audio/wav;base64,{}", b64);

        // Step 3: transcribe
        let transcribe_result = self
            .inference
            .transcribe(&audio_uri, language.as_deref())
            .await
            .map_err(|e| McpToolError::unavailable(format!("Transcription failed: {}", e)));

        match transcribe_result {
            Ok(tr) => {
                self.record_experience(
                    "record_and_transcribe",
                    &format!("duration={}s", duration_secs),
                    "success",
                    serde_json::json!({"audio_path": audio_path.display().to_string()}),
                );
                span.ok_json(serde_json::json!({
                    "status": "recorded_and_transcribed",
                    "duration_secs": duration_secs,
                    "audio_path": audio_path.display().to_string(),
                    "audio_format": "wav",
                    "sample_rate": 16000,
                    "channels": 1,
                    "transcript": tr,
                }))
            }
            Err(e) => {
                // Transcription failed but audio was captured — return partial success
                span.ok_json(serde_json::json!({
                    "status": "partial",
                    "duration_secs": duration_secs,
                    "audio_path": audio_path.display().to_string(),
                    "audio_format": "wav",
                    "sample_rate": 16000,
                    "channels": 1,
                    "transcript_error": e.to_json_string(),
                    "message": "Audio captured successfully but transcription failed. The audio file is saved and can be transcribed later."
                }))
            }
        }
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
        let span = ToolSpanGuard::new("generate_image", &self.webid);
        let size = image_size.clone();
        let result = self
            .inference
            .generate_image(&prompt, size.as_deref(), num_images)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Image generation failed: {}", e)));
        self.record_experience(
            "generate_image",
            &prompt,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"image_size": size, "num_images": num_images}),
        );
        span.finish(result)
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
        let span = ToolSpanGuard::new("transform_image", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let result = self
            .inference
            .image_to_image(&image_url, &prompt, strength)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Image transform failed: {}", e)));
        span.finish(result)
    }

    #[tool(description = "Upscale an image to higher resolution.")]
    async fn upscale_image(
        &self,
        Parameters(UpscaleImageRequest { image_url, scale }): Parameters<UpscaleImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("upscale_image", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let result = self
            .inference
            .upscale(&image_url, scale)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Upscale failed: {}", e)));
        span.finish(result)
    }

    #[tool(
        description = "Generate a short video from a text prompt. Describe the scene you want to see in motion."
    )]
    async fn generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("generate_video", &self.webid);
        let result = self
            .inference
            .generate_video(&prompt, duration)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Video generation failed: {}", e)));
        span.finish(result)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    // Build the inference router for vision LLM tasks.
    // Backends are constructed lazily — only those with configured API keys are available.
    let inference_config = hkask_inference::InferenceConfig::from_env();
    let inference = Arc::new(InferenceRouter::new(inference_config));

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.media", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(DaemonClient::new())
    } else {
        None
    };

    // Create an in-memory GalleryStore for the media server
    let db = hkask_storage::in_memory_db();
    {
        let conn = db.conn_arc();
        let conn = conn.lock().unwrap();
        GalleryStore::init_tables(&conn).expect("Failed to initialize gallery tables");
    }
    let gallery_store = Arc::new(GalleryStore::new(db.conn_arc()));

    hkask_mcp::run_server(
        "hkask-mcp-media",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            MediaServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                inference.clone(),
                gallery_store.clone(),
            )
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
                "FW_API_KEY",
                "Fireworks API key for vision LLMs",
            ),
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = DaemonClient::new();

    let auth = client.auth_query(replicant).await?;
    match auth {
        DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some(ref webid),
            ..
        } => {
            tracing::info!(target: "hkask.mcp.media", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
        }
        DaemonResponse::AuthResponse {
            authenticated: false,
            action: Some(ref action),
            ..
        } if action == "prompt_user" => {
            anyhow::bail!(
                "Replicant '{}' is not authenticated. Enter the replicant's passphrase in the hKask terminal.",
                replicant
            );
        }
        other => anyhow::bail!("Unexpected auth response: {:?}", other),
    }

    let assignment = client.assignment_query(replicant, "media").await?;
    match assignment {
        DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.media", replicant = %replicant, "Replicant assigned to media role");
        }
        DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the media MCP role. Use 'kask pod assign {} media' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.media", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
