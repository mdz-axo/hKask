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
mod video;

use gallery::{GalleryMode as LocalGalleryMode, GalleryState};
use hkask_inference::InferenceRouter;
use hkask_mcp::server::{McpToolError, ToolSpanGuard, validate_tool_url};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_storage::{GalleryMode, GalleryStore, GalleryStoreError};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
pub struct GalleryInitRequest {
    pub path: String,
    #[serde(default = "default_gallery_mode")]
    pub mode: String,
}

fn default_gallery_mode() -> String {
    "original".to_string()
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

    /// Resolve an image index to a base64 data URL for vision LLM calls.
    fn resolve_image_url(&self, image_index: usize) -> Result<String, String> {
        let guard = self.gallery_state.lock().unwrap();
        let state = guard
            .as_ref()
            .ok_or("No gallery initialized.".to_string())?;

        let mut count = 0usize;
        for entry in walkdir::WalkDir::new(&state.path)
            .max_depth(1)
            .into_iter()
            .flatten()
        {
            if entry.file_type().is_file() {
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff"
                ) {
                    if count == image_index {
                        let data = std::fs::read(entry.path())
                            .map_err(|e| format!("Failed to read image: {}", e))?;
                        let b64 = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &data,
                        );
                        let mime = match ext.to_lowercase().as_str() {
                            "jpg" | "jpeg" => "image/jpeg",
                            "png" => "image/png",
                            "webp" => "image/webp",
                            "gif" => "image/gif",
                            "bmp" => "image/bmp",
                            "tiff" => "image/tiff",
                            _ => "image/png",
                        };
                        return Ok(format!("data:{};base64,{}", mime, b64));
                    }
                    count += 1;
                }
            }
        }
        Err(format!("Image not found at index {}", image_index))
    }

    /// Resolve an image index to a filesystem path.
    fn resolve_image_path(&self, image_index: usize) -> Result<PathBuf, String> {
        let guard = self.gallery_state.lock().unwrap();
        let state = guard
            .as_ref()
            .ok_or("No gallery initialized.".to_string())?;

        let mut count = 0usize;
        for entry in walkdir::WalkDir::new(&state.path)
            .max_depth(1)
            .into_iter()
            .flatten()
        {
            if entry.file_type().is_file() {
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff"
                ) {
                    if count == image_index {
                        return Ok(entry.path().to_path_buf());
                    }
                    count += 1;
                }
            }
        }
        Err(format!("Image not found at index {}", image_index))
    }
}

#[tool_router(server_handler)]
impl MediaServer {
    // ── Gallery tools ────────────────────────────────────────────────────────

    #[tool(
        description = "Initialize or reconfigure an image gallery. Sets the folder path and operating mode (original=read-only, copy=editable)."
    )]
    async fn gallery_init(
        &self,
        Parameters(GalleryInitRequest { path, mode }): Parameters<GalleryInitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gallery_init", &self.webid);

        let gallery_mode = match mode.as_str() {
            "copy" => LocalGalleryMode::Copy,
            _ => LocalGalleryMode::Original,
        };

        let state = GalleryState::new(PathBuf::from(&path), gallery_mode);

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

        let summary = state.summary();
        *self.gallery_state.lock().unwrap() = Some(state);

        span.ok_json(serde_json::json!({
            "status": "initialized",
            "gallery": summary,
        }))
    }

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
        let summary = state.summary();

        span.ok_json(serde_json::json!({
            "scan": {
                "added": result.added,
                "removed": result.removed,
                "unchanged": result.unchanged,
                "total": result.total,
                "errors": result.errors,
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
                let state = GalleryState::new(PathBuf::from(&path), local_mode);
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

        // Find image by index in the in-memory state
        // For now, use the filesystem path directly
        let img_path = if let Some(idx) = index {
            // Walk the gallery to find the nth image
            let mut found: Option<PathBuf> = None;
            let mut count = 0usize;
            for entry in walkdir::WalkDir::new(&state.path)
                .max_depth(1)
                .into_iter()
                .flatten()
            {
                if entry.file_type().is_file() {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if matches!(
                        ext.to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff"
                    ) {
                        if count == idx {
                            found = Some(entry.path().to_path_buf());
                            break;
                        }
                        count += 1;
                    }
                }
            }
            found
        } else if let Some(_h) = &hash {
            // Hash lookup not supported in in-memory mode — would need SQLite
            None
        } else {
            None
        };

        match img_path {
            Some(p) => {
                let fmt = format.as_deref().unwrap_or("path");
                match fmt {
                    "base64" => match std::fs::read(&p) {
                        Ok(data) => {
                            let b64 = base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &data,
                            );
                            span.ok_json(serde_json::json!({
                                    "format": "base64",
                                    "data": format!("data:image/{};base64,{}", p.extension().and_then(|e| e.to_str()).unwrap_or("png"), b64),
                                    "path": p.display().to_string(),
                                }))
                        }
                        Err(e) => span.error(
                            McpErrorKind::Internal,
                            McpToolError::internal(format!("Failed to read image: {}", e))
                                .to_json_string(),
                        ),
                    },
                    _ => span.ok_json(serde_json::json!({
                        "format": "path",
                        "path": p.display().to_string(),
                    })),
                }
            }
            None => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("Image not found at specified index/hash.")
                    .to_json_string(),
            ),
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

        // Find image by index
        let img_path = if let Some(idx) = index {
            let mut found: Option<PathBuf> = None;
            let mut count = 0usize;
            for entry in walkdir::WalkDir::new(&state.path)
                .max_depth(1)
                .into_iter()
                .flatten()
            {
                if entry.file_type().is_file() {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if matches!(
                        ext.to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff"
                    ) {
                        if count == idx {
                            found = Some(entry.path().to_path_buf());
                            break;
                        }
                        count += 1;
                    }
                }
            }
            found
        } else {
            None
        };

        match img_path {
            Some(p) => {
                // Read dimensions
                let dims = image::image_dimensions(&p).ok();
                let file_size = std::fs::metadata(&p).ok().map(|m| m.len());

                span.ok_json(serde_json::json!({
                    "path": p.display().to_string(),
                    "filename": p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
                    "format": p.extension().and_then(|e| e.to_str()).unwrap_or("unknown"),
                    "width": dims.map(|(w, _)| w),
                    "height": dims.map(|(_, h)| h),
                    "size_bytes": file_size,
                    "tags": [],
                }))
            }
            None => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("Image not found at specified index.")
                    .to_json_string(),
            ),
        }
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

        let _detail = _detail_level.unwrap_or_else(|| "detailed".to_string());
        let prompt = "Analyze this image and detect all visible human faces.\n\nFor each face, provide:\n1. Estimated age range\n2. Apparent gender presentation\n3. Notable features (glasses, beard, expression, hair color/style)\n4. Position in image\n5. Approximate face size relative to image\n\nReturn ONLY a JSON array. Each element: face_index, age_range, gender_presentation, features, position, size.";

        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(
                prompt,
                &[image_url],
                &params,
                Some("DI/meta-llama/Llama-3.2-11B-Vision-Instruct"),
            )
            .await;

        self.record_experience(
            "tag_faces",
            &format!("image_index={}", image_index),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"detail_level": _detail}),
        );

        match result {
            Ok(r) => {
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

        let max = max_objects.unwrap_or(20);
        let prompt = format!(
            "Analyze this image and detect all visible objects.\n\nFor each object, provide:\n1. Object name (be specific)\n2. Bounding box description\n3. Confidence level (high/medium/low)\n4. Brief description of appearance\n\nLimit to the {} most prominent objects.\n\nReturn ONLY a JSON array. Each element: name, location, confidence, description.",
            max
        );

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

        let prompt = "Analyze this image and identify its color palette.\n\nFor each dominant color, provide: name, hex code, approximate percentage, role in composition.\nAlso describe: palette style, color temperature, saturation level.\n\nReturn ONLY a JSON object with fields: colors (array), palette_style, temperature, saturation.";

        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(
                prompt,
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

        let prompt = "Analyze the photographic composition of this image.\n\nEvaluate: focal point, rule of thirds, leading lines, depth of field, perspective, framing, symmetry, negative space.\n\nReturn ONLY a JSON object with these 8 fields.";

        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(
                prompt,
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
            Ok(r) => span
                .ok_json(serde_json::json!({"composition": r.text, "model": "llama-3.2-vision"})),
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

        let style_str = style.unwrap_or_else(|| "descriptive".to_string());
        let prompt = match style_str.as_str() {
            "artistic" => {
                "Write an artistic, evocative description of this image. Use poetic language and focus on mood, emotion, and aesthetic quality. Write 2-3 sentences. Return ONLY the description text."
            }
            "technical" => {
                "Provide a technical description of this image. Note photographic/compositional elements: focal point, depth of field, lighting, color palette, perspective. Write 2-4 sentences. Return ONLY the description text."
            }
            "alt_text" => {
                "Write concise alt text for this image suitable for accessibility. Describe only what is visually present — no interpretation. Keep to 1-2 sentences, max 125 characters. Return ONLY the alt text."
            }
            _ => {
                "Describe this image in detail. Cover the subject, setting, lighting, colors, composition, mood, and any notable details. Write 2-4 sentences. Return ONLY the description text."
            }
        };

        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(
                prompt,
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

        let prompt = if let Some(ref cats) = categories {
            format!(
                "Analyze this image and classify it. Classify into these categories (an image can belong to multiple): {}. For each matching category, provide: category (string), confidence (number 0.0-1.0). Return ONLY a JSON array.",
                cats
            )
        } else {
            "Analyze this image and classify its photographic style. Evaluate: genre (portrait, landscape, street, macro, architecture, documentary, abstract, etc.), era/style (contemporary, vintage, HDR, minimalist, etc.), technique (long exposure, bokeh, black-and-white, etc.). For each matching category, provide: category (string), confidence (number 0.0-1.0). Return ONLY a JSON array.".to_string()
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

        // Resolve all image paths
        let mut paths = Vec::new();
        for idx in &image_indices {
            match self.resolve_image_path(*idx) {
                Ok(p) => paths.push(p),
                Err(e) => return span.error(McpErrorKind::InvalidArgument, e),
            }
        }

        // For now, return the list of resolved paths as a collage manifest
        // Full composition with image crate will be implemented in Phase 2
        span.ok_json(serde_json::json!({
            "status": "collage_manifest",
            "image_count": paths.len(),
            "images": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "message": "Collage composition (image crate layout) will be implemented in Phase 2."
        }))
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

        // ffmpeg subprocess: ffmpeg -ss {start} -to {end} -i {input} -c copy {output}
        // Phase 1: return the clip parameters as a manifest
        span.ok_json(serde_json::json!({
            "status": "clip_manifest",
            "source": video_url,
            "start_sec": start_sec,
            "end_sec": end_sec,
            "duration": end_sec - start_sec,
            "message": "ffmpeg clip execution will be implemented in Phase 2."
        }))
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

        span.ok_json(serde_json::json!({
            "status": "gif_manifest",
            "source": video_url,
            "start_sec": start_sec.unwrap_or(0.0),
            "duration_sec": duration_sec.unwrap_or(5.0),
            "width": width.unwrap_or(480),
            "fps": fps.unwrap_or(10),
            "message": "ffmpeg GIF conversion will be implemented in Phase 2."
        }))
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

        span.ok_json(serde_json::json!({
            "status": "caption_manifest",
            "source": video_url,
            "text": text,
            "position": position.unwrap_or_else(|| "bottom".to_string()),
            "font_size": font_size.unwrap_or(24),
            "message": "ffmpeg drawtext overlay will be implemented in Phase 2."
        }))
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

        span.ok_json(serde_json::json!({
            "status": "remix_manifest",
            "source": video_url,
            "start_sec": start_sec,
            "end_sec": end_sec,
            "duration": end_sec - start_sec,
            "caption": caption_text,
            "pipeline": ["video_clip", "video_add_caption", "video_to_gif"],
            "message": "Remix pipeline will be implemented in Phase 2."
        }))
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
        let params = hkask_types::LLMParameters::default();
        let result = self
            .inference
            .generate_vision(
                "Provide a detailed caption for this image.",
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
        vec![],
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
