//! hKask MCP Media — AI media generation (image and short-video via centralized inference router)
//!
//! Tool families:
//! - Gallery: set_root, scan, info, get_image, get_metadata
//! - Tagging: tag_faces, tag_objects, tag_colors, tag_composition
//! - Abstraction: image_caption, image_describe_scene, image_classify_style
//! - Derivation: remove_background, apply_style, create_collage, upscale, image_to_image
//! - Video/GIF: generate_video, image_to_video, video_clip, video_to_gif, video_add_caption, video_remix
//! - Generation: generate_image, image_to_image, upscale, generate_video, caption

mod gallery;
mod templates;
mod video;

use gallery::{GalleryMode as LocalGalleryMode, GalleryState};
use hkask_inference::InferenceRouter;
use hkask_mcp::server::{McpToolError, ToolSpanGuard, validate_tool_url};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_storage::{GalleryMode, GalleryStore, GalleryStoreError};
use hkask_types::{InferencePort, McpErrorKind, VoiceDesign, WebID};
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
    /// Active gallery state (None until gallery_init is called)
    gallery_state: Arc<Mutex<Option<GalleryState>>>,
    /// SQLite-backed gallery store for persistent indexing
    gallery_store: Arc<GalleryStore>,
    /// Jinja2 template environment for prompt rendering
    template_env: minijinja::Environment<'static>,
    /// ffmpeg runner for video processing (None if ffmpeg not found)
    ffmpeg: FfmpegRunner,
}

// ── Legacy request types (existing tools) ───────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateImageRequest {
    pub prompt: String,
    pub image_size: Option<String>,
    pub num_images: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImageToImageRequest {
    pub prompt: String,
    pub image_url: String,
    pub strength: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpscaleRequest {
    pub image_url: String,
    pub scale: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateVideoRequest {
    pub prompt: String,
    pub duration: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CaptionRequest {
    pub image_url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryScanRequest {
    #[serde(default = "default_true")]
    pub recursive: bool,
    pub extensions: Option<Vec<String>>,
}

fn default_true() -> bool {
    true
}

// ── Gallery management request types ──────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GallerySetRootRequest {
    pub path: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "read-only".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryGetImageRequest {
    pub index: Option<usize>,
    pub hash: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GalleryGetMetadataRequest {
    pub index: Option<usize>,
    pub hash: Option<String>,
}

// ── Tagging request types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagFacesRequest {
    pub image_index: usize,
    pub detail_level: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagObjectsRequest {
    pub image_index: usize,
    pub detail_level: Option<String>,
    pub max_objects: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagColorsRequest {
    pub image_index: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagCompositionRequest {
    pub image_index: usize,
}

// ── Abstraction request types ────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DescribeSceneRequest {
    pub image_index: usize,
    pub style: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClassifyStyleRequest {
    pub image_index: usize,
    pub categories: Option<String>,
}

// ── Derivation request types ─────────────────────────────────────────────────

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
    pub image_indices: Vec<usize>,
    pub layout: Option<String>,
    pub spacing: Option<u32>,
    pub canvas_size: Option<String>,
}

// ── Video request types ──────────────────────────────────────────────────────

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

// ── Voice request types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoiceDesignRequest {
    /// Character description to design a voice for.
    pub character_description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateSpeechRequest {
    /// Text to convert to speech.
    pub text: String,
    /// Voice design JSON (as produced by voice_design tool).
    pub voice_design: Option<String>,
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
        description = "Scan the gallery directory for new, changed, or removed images. Computes SHA-256 checksums and image dimensions."
    )]
    async fn gallery_scan(
        &self,
        Parameters(GalleryScanRequest {
            recursive,
            extensions,
        }): Parameters<GalleryScanRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_scan", &self.webid);

        let mut guard = self.gallery_state.lock().unwrap();
        let state = match &mut *guard {
            Some(s) => s,
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(
                        "No gallery initialized. Use gallery_init first.",
                    )
                    .to_json_string(),
                );
            }
        };

        let result = state.scan(recursive, extensions.as_deref());

        // Persist discovered images to SQLite
        let gallery_id = state.gallery_id.clone();
        let mut persisted = 0u32;
        let mut persist_errors = Vec::new();
        if let Some(ref gid) = gallery_id {
            for entry in &result.entries {
                let abs_path = state.path.join(&entry.relative_path);
                match self.gallery_store.add_image(
                    gid,
                    &entry.relative_path,
                    &abs_path.to_string_lossy(),
                    &entry.checksum,
                    entry.width,
                    entry.height,
                    &entry.format,
                    entry.size_bytes,
                ) {
                    Ok(_) => persisted += 1,
                    Err(e) => persist_errors.push(format!("{}: {}", entry.relative_path, e)),
                }
            }
        }

        let summary = state.summary();

        span.ok_json(serde_json::json!({
            "scan": {
                "added": result.added,
                "removed": result.removed,
                "unchanged": result.unchanged,
                "total": result.total,
                "errors": result.errors,
                "persisted": persisted,
                "persist_errors": persist_errors,
            },
            "gallery": summary,
        }))
    }

    #[tool(description = "Get current gallery status: path, mode, image count, size, tags.")]
    async fn gallery_info(&self) -> String {
        let span = ToolSpanGuard::new("gallery_info", &self.webid);

        let guard = self.gallery_state.lock().unwrap();
        match &*guard {
            Some(state) => span.ok_json(state.summary()),
            None => span.ok_json(serde_json::json!({
                "status": "no_gallery",
                "message": "No gallery initialized. Use gallery_init to create one."
            })),
        }
    }

    // ── Gallery management tools ────────────────────────────────────────────

    #[tool(
        description = "Initialize or reconfigure an image gallery with a root path and policy mode (read-only, copy-on-write, destructive)."
    )]
    async fn gallery_set_root(
        &self,
        Parameters(GallerySetRootRequest { path, mode }): Parameters<GallerySetRootRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_set_root", &self.webid);

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

        // Create in SQLite
        match self.gallery_store.create(&path, gallery_mode.clone()) {
            Ok(record) => {
                // Also set up the in-memory GalleryState for filesystem operations
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
                // Store gallery_id so scans can persist to SQLite
                state.gallery_id = Some(record.id.clone());
                *self.gallery_state.lock().unwrap() = Some(state);

                span.ok_json(serde_json::json!({
                    "status": "initialized",
                    "gallery_id": record.id,
                    "root_path": record.root_path,
                    "mode": record.mode,
                }))
            }
            Err(GalleryStoreError::AlreadyExists(_)) => span.ok_json(serde_json::json!({
                "status": "already_exists",
                "message": "A gallery already exists at this path. Use gallery_scan to update it."
            })),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to create gallery: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Get a reference to a gallery image by index or hash. Returns path, base64, or URL."
    )]
    async fn gallery_get_image(
        &self,
        Parameters(GalleryGetImageRequest {
            index,
            hash,
            format,
        }): Parameters<GalleryGetImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_get_image", &self.webid);

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

        // Look up image in SQLite via GalleryStore
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

        let img_record = match self
            .gallery_store
            .get_image(&gallery_id, index, hash.as_deref())
        {
            Ok(r) => r,
            Err(GalleryStoreError::ImageNotFound(msg)) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(msg).to_json_string(),
                );
            }
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Gallery lookup failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        let img_path = PathBuf::from(&img_record.absolute_path);
        let fmt = format.as_deref().unwrap_or("path");
        match fmt {
            "base64" => match std::fs::read(&img_path) {
                Ok(data) => {
                    let b64 =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                    span.ok_json(serde_json::json!({
                        "format": "base64",
                        "data": format!("data:image/{};base64,{}", img_record.format, b64),
                        "path": img_record.absolute_path,
                        "hash": img_record.hash,
                        "width": img_record.width,
                        "height": img_record.height,
                    }))
                }
                Err(e) => span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to read image: {}", e)).to_json_string(),
                ),
            },
            _ => span.ok_json(serde_json::json!({
                "format": "path",
                "path": img_record.absolute_path,
                "hash": img_record.hash,
                "width": img_record.width,
                "height": img_record.height,
            })),
        }
    }

    #[tool(description = "Get metadata for a gallery image including AI-generated tags.")]
    async fn gallery_get_metadata(
        &self,
        Parameters(GalleryGetMetadataRequest { index, hash: _hash }): Parameters<
            GalleryGetMetadataRequest,
        >,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_get_metadata", &self.webid);

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

        let img_record = match self.gallery_store.get_image(&gallery_id, index, None) {
            Ok(r) => r,
            Err(GalleryStoreError::ImageNotFound(msg)) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(msg).to_json_string(),
                );
            }
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Gallery lookup failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        // Read tags from SQLite
        let tags = self
            .gallery_store
            .get_tags(&img_record.id)
            .unwrap_or_default();
        let tag_list: Vec<serde_json::Value> = tags
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": t.tag_type,
                    "value": t.value,
                    "confidence": t.confidence,
                    "model": t.model_used,
                })
            })
            .collect();

        // Read EXIF metadata from the image file
        let exif_data = Self::extract_exif(&img_record.absolute_path);

        span.ok_json(serde_json::json!({
            "path": img_record.absolute_path,
            "filename": PathBuf::from(&img_record.absolute_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            "format": img_record.format,
            "width": img_record.width,
            "height": img_record.height,
            "size_bytes": img_record.size_bytes,
            "hash": img_record.hash,
            "tags": tag_list,
            "exif": exif_data,
        }))
    }

    // ── Tagging tools ───────────────────────────────────────────────────────

    #[tool(description = "Detect and describe faces in a gallery image using vision LLM.")]
    async fn tag_faces(
        &self,
        Parameters(TagFacesRequest {
            image_index,
            detail_level: _detail_level,
        }): Parameters<TagFacesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tag_faces", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let detail = _detail_level.as_deref().unwrap_or("detailed");
        let mut vars = HashMap::new();
        vars.insert("detail_level", detail);
        let prompt = match self.render_prompt("tag_faces", &vars) {
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
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "tag_faces",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"detail_level": detail}),
        );

        match result {
            Ok(r) => {
                // Persist face tags to gallery store
                if let Ok(image_id) = self.resolve_image_id(image_index) {
                    if let Ok(faces) = serde_json::from_str::<Vec<serde_json::Value>>(&r.text) {
                        for face in &faces {
                            let value = serde_json::to_string(face).unwrap_or_default();
                            self.persist_tag(&image_id, "face", &value, 0.85, "llama-3.2-vision");
                        }
                    } else {
                        // Non-JSON response — store as raw text
                        self.persist_tag(&image_id, "face", r.text.trim(), 0.7, "llama-3.2-vision");
                    }
                }
                span.ok_json(serde_json::json!({"faces": r.text, "model": "llama-3.2-vision"}))
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Detect and label objects in a gallery image using vision LLM.")]
    async fn tag_objects(
        &self,
        Parameters(TagObjectsRequest {
            image_index,
            detail_level: _detail_level,
            max_objects,
        }): Parameters<TagObjectsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tag_objects", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let detail = _detail_level.as_deref().unwrap_or("detailed");
        let max = max_objects.unwrap_or(20);
        let max_str = max.to_string();
        let mut vars = HashMap::new();
        vars.insert("detail_level", detail);
        vars.insert("max_objects", &max_str);
        let prompt = match self.render_prompt("tag_objects", &vars) {
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
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "tag_objects",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"max_objects": max}),
        );

        match result {
            Ok(r) => {
                // Persist object tags to gallery store
                if let Ok(image_id) = self.resolve_image_id(image_index) {
                    if let Ok(objects) = serde_json::from_str::<Vec<serde_json::Value>>(&r.text) {
                        for obj in &objects {
                            let value = serde_json::to_string(obj).unwrap_or_default();
                            self.persist_tag(&image_id, "object", &value, 0.85, "llama-3.2-vision");
                        }
                    } else {
                        self.persist_tag(
                            &image_id,
                            "object",
                            r.text.trim(),
                            0.7,
                            "llama-3.2-vision",
                        );
                    }
                }
                span.ok_json(serde_json::json!({"objects": r.text, "model": "llama-3.2-vision"}))
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Analyze dominant colors and palette in a gallery image using vision LLM."
    )]
    async fn tag_colors(
        &self,
        Parameters(TagColorsRequest { image_index }): Parameters<TagColorsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tag_colors", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let mut vars = HashMap::new();
        vars.insert("max_colors", "8");
        let prompt = match self.render_prompt("tag_colors", &vars) {
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
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "tag_colors",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({}),
        );

        match result {
            Ok(r) => {
                // Persist color tags to gallery store
                if let Ok(image_id) = self.resolve_image_id(image_index) {
                    // Try to parse as structured JSON with colors array
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&r.text) {
                        if let Some(colors) = parsed["colors"].as_array() {
                            for color in colors {
                                let value = serde_json::to_string(color).unwrap_or_default();
                                self.persist_tag(
                                    &image_id,
                                    "color",
                                    &value,
                                    0.85,
                                    "llama-3.2-vision",
                                );
                            }
                        }
                        // Also store palette-level metadata
                        for field in &["palette_style", "temperature", "saturation"] {
                            if let Some(v) = parsed.get(*field).and_then(|v| v.as_str()) {
                                self.persist_tag(&image_id, "color", v, 0.9, "llama-3.2-vision");
                            }
                        }
                    } else {
                        self.persist_tag(
                            &image_id,
                            "color",
                            r.text.trim(),
                            0.7,
                            "llama-3.2-vision",
                        );
                    }
                }
                span.ok_json(serde_json::json!({"colors": r.text, "model": "llama-3.2-vision"}))
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Analyze photographic composition of a gallery image using vision LLM.")]
    async fn tag_composition(
        &self,
        Parameters(TagCompositionRequest { image_index }): Parameters<TagCompositionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tag_composition", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let vars = HashMap::new();
        let prompt = match self.render_prompt("tag_composition", &vars) {
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
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "tag_composition",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({}),
        );

        match result {
            Ok(r) => {
                // Persist composition tags to gallery store
                if let Ok(image_id) = self.resolve_image_id(image_index) {
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
                                    "llama-3.2-vision",
                                );
                            }
                        }
                    } else {
                        self.persist_tag(
                            &image_id,
                            "composition",
                            r.text.trim(),
                            0.7,
                            "llama-3.2-vision",
                        );
                    }
                }
                span.ok_json(
                    serde_json::json!({"composition": r.text, "model": "llama-3.2-vision"}),
                )
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    // ── Abstraction tools ───────────────────────────────────────────────────

    #[tool(
        description = "Describe the full scene: subject, setting, lighting, mood using vision LLM."
    )]
    async fn image_describe_scene(
        &self,
        Parameters(DescribeSceneRequest { image_index, style }): Parameters<DescribeSceneRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("image_describe_scene", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let style_str = style.as_deref().unwrap_or("descriptive");
        let mut vars = HashMap::new();
        vars.insert("style", style_str);
        let prompt = match self.render_prompt("describe_scene", &vars) {
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
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "image_describe_scene",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"style": style_str}),
        );

        match result {
            Ok(r) => span.ok_json(serde_json::json!({"description": r.text, "style": style_str})),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Classify image style: photographic style, genre, era using vision LLM.")]
    async fn image_classify_style(
        &self,
        Parameters(ClassifyStyleRequest {
            image_index,
            categories,
        }): Parameters<ClassifyStyleRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("image_classify_style", &self.webid);
        let image_url = match self.resolve_image_url(image_index) {
            Ok(url) => url,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
        };

        let cats = categories.as_deref().unwrap_or("");
        let mut vars = HashMap::new();
        vars.insert("categories", cats);
        let prompt = match self.render_prompt("classify_style", &vars) {
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
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "image_classify_style",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"categories": categories}),
        );

        match result {
            Ok(r) => span.ok_json(
                serde_json::json!({"classifications": r.text, "model": "llama-3.2-vision"}),
            ),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Vision inference failed: {}", e))
                    .to_json_string(),
            ),
        }
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
        description = "Create a collage from multiple gallery images. Local composition using image crate."
    )]
    async fn image_create_collage(
        &self,
        Parameters(CreateCollageRequest {
            image_indices,
            layout: _layout,
            spacing: _spacing,
            canvas_size: _canvas_size,
        }): Parameters<CreateCollageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("image_create_collage", &self.webid);

        if image_indices.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("At least one image index is required.")
                    .to_json_string(),
            );
        }

        if image_indices.len() > 9 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("Maximum 9 images supported for collage.")
                    .to_json_string(),
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

        let spacing = _spacing.unwrap_or(8);
        let layout = _layout.as_deref().unwrap_or("grid");

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
        let cols = match layout {
            "horizontal" => images.len() as u32,
            "vertical" => 1u32,
            _ => {
                // grid: auto-compute columns for roughly square layout
                (images.len() as f64).sqrt().ceil() as u32
            }
        };
        let rows = (images.len() as u32 + cols - 1) / cols;

        // Determine cell size from canvas or auto-compute
        let canvas_w: u32;
        let canvas_h: u32;
        if let Some(ref size_str) = _canvas_size {
            let parts: Vec<&str> = size_str.split('x').collect();
            canvas_w = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1920);
            canvas_h = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1080);
        } else {
            // Auto-size: use the largest image dimensions as cell size
            let max_w = images.iter().map(|img| img.width()).max().unwrap_or(640);
            let max_h = images.iter().map(|img| img.height()).max().unwrap_or(480);
            canvas_w = max_w * cols + spacing * (cols + 1);
            canvas_h = max_h * rows + spacing * (rows + 1);
        }

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
        let result = self
            .inference
            .generate(&prompt, &params)
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
        description = "Generate speech audio from text using a voice design. Returns audio URL."
    )]
    async fn generate_speech(
        &self,
        Parameters(GenerateSpeechRequest { text, voice_design }): Parameters<GenerateSpeechRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("generate_speech", &self.webid);

        // Build voice description for TTS model
        let voice_desc = if let Some(ref vd_json) = voice_design {
            match serde_json::from_str::<VoiceDesign>(vd_json) {
                Ok(vd) => vd.to_tts_description(),
                Err(_) => vd_json.clone(),
            }
        } else {
            VoiceDesign::default().to_tts_description()
        };

        let result = self
            .inference
            .generate_speech(&text, &voice_desc)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Speech generation failed: {}", e)));

        self.record_experience(
            "generate_speech",
            &text,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"voice": voice_desc}),
        );

        span.finish(result)
    }

    // ── fal.ai generation tools ──────────────────────────────────────────────

    #[tool(description = "Ping Fal.ai API to verify connectivity and authentication")]
    async fn fal_ping(&self) -> String {
        let span = ToolSpanGuard::new("fal_ping", &self.webid);
        // Ping via inference router — try a lightweight image generation
        match self
            .inference
            .generate_image("test ping", Some("64x64"), Some(1))
            .await
        {
            Ok(_) => span.ok_json(serde_json::json!({
                "status": "ok",
                "message": "fal.ai API is reachable and authenticated via inference router",
            })),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Connection failed: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(description = "Generate an image from a prompt")]
    async fn fal_generate_image(
        &self,
        Parameters(GenerateImageRequest {
            prompt,
            image_size,
            num_images,
        }): Parameters<GenerateImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_generate_image", &self.webid);
        let size = image_size.clone();
        let result = self
            .inference
            .generate_image(&prompt, size.as_deref(), num_images)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Image generation failed: {}", e)));
        self.record_experience(
            "fal_generate_image",
            &prompt,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"image_size": size, "num_images": num_images}),
        );
        span.finish(result)
    }

    #[tool(description = "Transform an image with a prompt")]
    async fn fal_image_to_image(
        &self,
        Parameters(ImageToImageRequest {
            prompt,
            image_url,
            strength,
        }): Parameters<ImageToImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_image_to_image", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let result = self
            .inference
            .image_to_image(&image_url, &prompt, strength)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Image-to-image failed: {}", e)));
        span.finish(result)
    }

    #[tool(description = "Upscale an image")]
    async fn fal_upscale(
        &self,
        Parameters(UpscaleRequest { image_url, scale }): Parameters<UpscaleRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_upscale", &self.webid);
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

    #[tool(description = "Generate a video from a prompt")]
    async fn fal_generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_generate_video", &self.webid);
        let result = self
            .inference
            .generate_video(&prompt, duration)
            .await
            .map_err(|e| McpToolError::unavailable(format!("Video generation failed: {}", e)));
        span.finish(result)
    }

    #[tool(description = "Generate a caption for an image")]
    async fn fal_caption(
        &self,
        Parameters(CaptionRequest { image_url }): Parameters<CaptionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_caption", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let mut vars = HashMap::new();
        vars.insert("style", "descriptive");
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
        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(
                &prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await
            .map_err(|e| McpToolError::unavailable(format!("Caption generation failed: {}", e)));
        match result {
            Ok(r) => span.ok_json(serde_json::json!({"caption": r.text})),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
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
