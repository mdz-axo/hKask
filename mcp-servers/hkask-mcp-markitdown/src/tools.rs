//! MCP tools for document format conversion and OCR
//!
//! Three tools exposed via MCP protocol:
//! - `markitdown_convert` — Detect format, extract text, OCR fallback for scanned docs
//! - `markitdown_detect_format` — Detect document format from path/extension
//! - `markitdown_ocr` — Explicitly OCR a document using local vision model
//!
//! OCR requires `HKASK_OCR_MODEL` to be set to a vision-capable model name
//! that exists in the inference catalog (e.g., a model with vision support).
//! Use `InferenceRouter::list_models()` to discover available models.

use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_types::ocr::{OcrBackend, OcrResult, ThresholdConfig};
use hkask_types::{LLMParameters, McpErrorKind, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::convert;
use crate::ocr::decimation;
use crate::ocr::pipeline::{self, OcrExecutor};

/// Minimum word count threshold for PDF text extraction results.
/// Below this, we consider the PDF to be scanned/image-based and fall back to OCR.
const OCR_FALLBACK_WORD_THRESHOLD: usize = 50;

const OCR_SYSTEM_PROMPT: &str = "Extract all text from this document. Output the text exactly as it appears, preserving the document structure and layout as closely as possible. If the document contains tables, preserve them in a readable format. Do not add commentary or description — only the extracted text.";

// ── Request structs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConvertRequest {
    /// Path to the document file to convert.
    pub path: String,
    /// If true, skip text extraction and go directly to OCR.
    #[serde(default)]
    pub force_ocr: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DetectFormatRequest {
    /// Path to the document file to detect format for.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OcrRequest {
    /// Path to the document file to OCR.
    pub path: String,
    /// Vision model to use for OCR (must be available in the inference catalog).
    /// If not set, uses HKASK_OCR_MODEL from environment.
    #[serde(default)]
    pub model: Option<String>,
    /// Maximum tokens for OCR output.
    #[serde(default = "default_ocr_max_tokens")]
    pub max_tokens: u32,
}

fn default_ocr_max_tokens() -> u32 {
    8192
}

// ── Server ───────────────────────────────────────────────────────────────

pub struct MarkitdownServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<hkask_mcp::DaemonClient>,
    /// Configured OCR model (from HKASK_OCR_MODEL env var). None means OCR is unavailable.
    ocr_model: Option<String>,
    /// Inference configuration for the router.
    inference_config: InferenceConfig,
    /// OCR pipeline thresholds (loaded from settings.json).
    ocr_thresholds: ThresholdConfig,
    /// Embedding router for semantic cross-validation (None if unavailable).
    embedding_router: Option<EmbeddingRouter>,
}

impl MarkitdownServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        ocr_model: Option<String>,
        inference_config: InferenceConfig,
        ocr_thresholds: ThresholdConfig,
        embedding_router: Option<EmbeddingRouter>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            webid,
            replicant,
            daemon,
            ocr_model,
            inference_config,
            ocr_thresholds,
            embedding_router,
        })
    }

    /// Persist pipeline outcome to daemon for CNS observability.
    /// Routes through the Curator's NuEvent → NuEventStore → CurationLoop path.
    fn persist_pipeline_outcome(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
        if let Some(ref daemon) = self.daemon {
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let data = serde_json::json!({
                "total_pages": outcome.results.len(),
                "error_count": outcome.errors.len(),
                "verification_passed": outcome.report.passed,
                "page_count_match": outcome.report.page_count_match,
                "empty_pages": outcome.report.empty_pages,
                "word_count_delta_pct": outcome.report.word_count_delta_pct,
                "cross_validations": outcome.cross_validations.len(),
                "backend_distribution": outcome.results.iter()
                    .fold(std::collections::HashMap::new(), |mut acc, r| {
                        *acc.entry(r.backend.label().to_string()).or_insert(0) += 1;
                        acc
                    }),
            });
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(
                        &replicant,
                        "ocr_pipeline",
                        "verification",
                        &data,
                        Some(0.85),
                    )
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.markitdown.cns", "Pipeline outcome persisted to daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.markitdown.cns", response = ?other, "Unexpected daemon response");
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.markitdown.cns", error = %e, "Failed to persist pipeline outcome");
                    }
                }
            });
        }
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
                "tool": tool, "input": input_summary, "outcome": outcome,
                "detail": detail, "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let tool_name = tool.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(&replicant, "mcp_session", "observed", &value, Some(0.85))
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.markitdown.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.markitdown.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.markitdown.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }

    /// Resolve OCR model: explicit override > HKASK_OCR_MODEL env.
    /// Validates that the model is likely vision-capable via the inference router.
    /// Returns error guidance if no model is configured or model lacks vision support.
    async fn resolve_ocr_model(&self, override_model: Option<&str>) -> Result<String, String> {
        let model = if let Some(m) = override_model
            && !m.is_empty()
        {
            m.to_string()
        } else {
            self.ocr_model.clone().ok_or_else(|| {
                "No OCR model configured. Set HKASK_OCR_MODEL env var to a vision-capable model, or pass the 'model' parameter. Use inference_models to discover available models.".to_string()
            })?
        };

        // Validate vision support via model listing heuristic
        let router = InferenceRouter::new(self.inference_config.clone());
        let vision_models = router.list_vision_models().await;
        let is_vision = vision_models
            .iter()
            .any(|m| m.model == model || m.prefixed_name == model);

        if !is_vision {
            // Check if the model exists at all
            let all_models = router.list_models().await;
            let exists = all_models
                .iter()
                .any(|m| m.model == model || m.prefixed_name == model);
            if exists {
                return Err(format!(
                    "Model '{}' exists but may not support vision input. Use a vision-capable model (e.g., llava, minicpm-v, lighton). Run 'kask inference models' to list available models.",
                    model
                ));
            }
            // Model not found — still allow attempt (may be a model not yet pulled)
        }

        Ok(model)
    }

    /// Perform OCR by sending base64-encoded bytes to a vision model.
    ///
    /// Creates an `InferenceRouter` from the server's config and dispatches
    /// via `generate_vision` with a model override.
    async fn do_ocr(
        &self,
        file_bytes: &[u8],
        model: &str,
        max_tokens: u32,
    ) -> Result<String, String> {
        if file_bytes.is_empty() {
            return Err("File is empty".to_string());
        }

        let b64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, file_bytes);

        let router = InferenceRouter::new(self.inference_config.clone());

        let params = LLMParameters {
            temperature: 0.1, // Low temperature for faithful extraction
            max_tokens,
            ..Default::default()
        };

        let result = router
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], &params, Some(model))
            .await
            .map_err(|e| format!("OCR inference failed: {}", e))?;

        Ok(result.text)
    }
}

// ── OcrExecutor implementation for MarkitdownServer ─────────────────────

use crate::ocr::llm_ocr::LlmOcrExecutor;
use crate::ocr::tesseract::TesseractExecutor;

#[async_trait::async_trait]
impl OcrExecutor for MarkitdownServer {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        match backend {
            OcrBackend::Tesseract => TesseractExecutor::new().is_available(backend),
            OcrBackend::LlmOcr(_) => self.ocr_model.is_some(),
        }
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &image::DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String> {
        match backend {
            OcrBackend::Tesseract => {
                TesseractExecutor::new()
                    .execute(page_index, backend, image, is_fallback)
                    .await
            }
            _ => {
                LlmOcrExecutor::new(self.inference_config.clone())
                    .execute(page_index, backend, image, is_fallback)
                    .await
            }
        }
    }
}

// ── CnsObserver implementation (tracing-based) ──────────────────────────

/// CNS observer that implements the real `hkask_types::ports::CnsObserver` trait.
/// Scaffolding for future NuEvent → NuEventStore → CurationLoop integration.
/// Currently unused — OCR pipeline emits tracing spans directly.
#[allow(dead_code)]
struct MarkitdownCnsObserver {
    daemon: Option<hkask_mcp::DaemonClient>,
    replicant: String,
}

#[allow(dead_code)]
impl MarkitdownCnsObserver {
    #[allow(dead_code)]
    fn new(daemon: Option<hkask_mcp::DaemonClient>, replicant: &str) -> Self {
        Self {
            daemon,
            replicant: replicant.to_string(),
        }
    }

    fn persist_span(&self, span_type: &str, data: serde_json::Value) {
        if let Some(ref daemon) = self.daemon {
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let span_name = span_type.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(
                        &replicant,
                        "ocr_pipeline",
                        span_name.as_str(),
                        &data,
                        Some(0.85),
                    )
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.markitdown.cns", span = %span_name, "CNS span persisted");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.markitdown.cns", span = %span_name, response = ?other, "Unexpected daemon response");
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.markitdown.cns", span = %span_name, error = %e, "Failed to persist CNS span");
                    }
                }
            });
        }
    }
}

#[async_trait::async_trait]
impl hkask_types::ports::CnsObserver for MarkitdownCnsObserver {
    fn interest_mask(&self) -> Vec<hkask_types::event::SpanNamespace> {
        vec![hkask_types::event::SpanNamespace::new("cns.pipeline")]
    }

    async fn on_event(&self, _event: &hkask_types::event::NuEvent) {
        // OCR pipeline events are emitted via tracing spans (cns.pipeline.ocr target).
        // Full NuEvent → NuEventStore → CurationLoop integration is deferred
        // until the pipeline has access to CNS infrastructure.
    }

    async fn on_depletion(&self, _signal: &hkask_types::ports::DepletionSignal) {
        tracing::warn!(target: "hkask.mcp.markitdown.cns", "CNS depletion signal received");
    }

    async fn on_backpressure(&self, _signal: &hkask_types::ports::BackpressureSignal) {
        tracing::warn!(target: "hkask.mcp.markitdown.cns", "CNS backpressure signal received");
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl MarkitdownServer {
    #[tool(
        description = "Extract text from a document. Detects format, extracts text with automatic OCR fallback for scanned/image-based PDFs. For PDF: tries text extraction first, falls back to vision OCR if result is near-empty. For other supported formats (TXT, MD, HTML): extracts plain text. Requires HKASK_OCR_MODEL for OCR fallback."
    )]
    async fn markitdown_convert(
        &self,
        Parameters(ConvertRequest { path, force_ocr }): Parameters<ConvertRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("markitdown_convert", &self.webid);
        let path_clone = path.clone();
        validate_field!(span, "path", &path, 4096);

        let format = convert::detect_format(&path);

        // Read the file
        let file_bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                return span.internal_error(serde_json::json!({
                    "error": format!("Failed to read file '{}': {}", path, e),
                }));
            }
        };

        if file_bytes.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("File '{}' is empty", path))
                    .to_json_string(),
            );
        }

        // When force_ocr is set, skip text extraction entirely.
        // Try pipeline path for image formats; fall back to raw bytes OCR for PDFs.
        if force_ocr {
            // Attempt to decode as image for typed pipeline path
            if let Ok(image) = image::load_from_memory(&file_bytes) {
                let model = match self.resolve_ocr_model(None).await {
                    Ok(m) => m,
                    Err(guidance) => {
                        return span.error(
                            McpErrorKind::FailedPrecondition,
                            McpToolError::failed_precondition(guidance).to_json_string(),
                        );
                    }
                };

                let emb = self.embedding_router.as_ref().map(|r| {
                    (
                        r,
                        self.ocr_model
                            .as_deref()
                            .unwrap_or("DI/Qwen/Qwen3-Embedding-0.6B"),
                    )
                });

                let outcome = pipeline::run_pipeline(
                    vec![image],
                    1,
                    self,
                    &self.ocr_thresholds,
                    Some(&model),
                    emb,
                )
                .await;

                // Persist to daemon for CNS → Curator observability
                self.persist_pipeline_outcome(&outcome);

                let text = outcome
                    .results
                    .first()
                    .map(|r| r.text.clone())
                    .unwrap_or_default();
                let result = serde_json::json!({
                    "format": format,
                    "path": path,
                    "method": "ocr_pipeline",
                    "model": model,
                    "text": text,
                    "word_count": text.split_whitespace().count(),
                    "verification_passed": outcome.report.passed,
                    "page_count_match": outcome.report.page_count_match,
                    "empty_pages": outcome.report.empty_pages,
                    "error_count": outcome.errors.len(),
                });
                self.record_experience(
                    "markitdown_convert",
                    &path_clone,
                    "success",
                    result.clone(),
                );
                return span.ok_json(result);
            }

            // Not an image — fall back to raw bytes OCR (PDFs, etc.)
            match self.resolve_ocr_model(None).await {
                Ok(model) => match self
                    .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                    .await
                {
                    Ok(text) => {
                        let result = serde_json::json!({
                            "format": format,
                            "path": path,
                            "method": "ocr",
                            "model": model,
                            "text": text,
                            "word_count": text.split_whitespace().count(),
                        });
                        self.record_experience(
                            "markitdown_convert",
                            &path_clone,
                            "success",
                            result.clone(),
                        );
                        return span.ok_json(result);
                    }
                    Err(e) => {
                        return span.error(
                            McpErrorKind::Unavailable,
                            McpToolError::unavailable(e).to_json_string(),
                        );
                    }
                },
                Err(guidance) => {
                    return span.error(
                        McpErrorKind::FailedPrecondition,
                        McpToolError::failed_precondition(guidance).to_json_string(),
                    );
                }
            }
        }

        // Extract text based on format
        let extract_result = match format {
            "pdf" => {
                // Try typed pipeline with decimation first (if OCR model is configured)
                if let Ok(model) = self.resolve_ocr_model(None).await
                    && let Ok(page_images) =
                        decimation::pdf_to_images(std::path::Path::new(&path), 200)
                {
                    let expected = page_images.len();
                    let emb = self.embedding_router.as_ref().map(|r| {
                        (
                            r,
                            self.ocr_model
                                .as_deref()
                                .unwrap_or("DI/Qwen/Qwen3-Embedding-0.6B"),
                        )
                    });

                    let outcome = pipeline::run_pipeline(
                        page_images,
                        expected,
                        self,
                        &self.ocr_thresholds,
                        Some(&model),
                        emb,
                    )
                    .await;

                    // Persist to daemon for CNS → Curator observability
                    self.persist_pipeline_outcome(&outcome);

                    let text = outcome
                        .results
                        .iter()
                        .map(|r| r.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let word_count = text.split_whitespace().count();

                    let result = serde_json::json!({
                        "format": format,
                        "path": path,
                        "method": "ocr_pipeline",
                        "model": model,
                        "text": text,
                        "word_count": word_count,
                        "pages": expected,
                        "verification_passed": outcome.report.passed,
                        "page_count_match": outcome.report.page_count_match,
                        "empty_pages": outcome.report.empty_pages,
                        "error_count": outcome.errors.len(),
                        "cross_validations": outcome.cross_validations.len(),
                    });
                    self.record_experience(
                        "markitdown_convert",
                        &path_clone,
                        "success",
                        result.clone(),
                    );
                    return span.ok_json(result);
                }
                // Decimation failed — fall through to pdf-extract
                // No OCR model or decimation unavailable — fall through

                // Try pdf-extract first; fall back to OCR if near-empty
                match pdf_extract::extract_text(&path) {
                    Ok(text) => {
                        let word_count = text.split_whitespace().count();
                        if word_count < OCR_FALLBACK_WORD_THRESHOLD {
                            // Near-empty — likely a scanned PDF
                            ExtractOutcome::NeedsOcr {
                                partial_text: text,
                                word_count,
                            }
                        } else {
                            ExtractOutcome::Success { text, word_count }
                        }
                    }
                    Err(_) => {
                        // pdf-extract failed entirely — try OCR
                        ExtractOutcome::NeedsOcr {
                            partial_text: String::new(),
                            word_count: 0,
                        }
                    }
                }
            }
            "plain" => match std::str::from_utf8(&file_bytes) {
                Ok(text) => ExtractOutcome::Success {
                    text: text.to_string(),
                    word_count: text.split_whitespace().count(),
                },
                Err(e) => {
                    return span.internal_error(serde_json::json!({
                        "error": format!("Failed to decode text file '{}': {}", path, e),
                    }));
                }
            },
            "markdown" => {
                match std::str::from_utf8(&file_bytes) {
                    Ok(content) => {
                        // Strip YAML frontmatter if present
                        let text = if content.starts_with("---") {
                            content
                                .splitn(3, "---")
                                .nth(2)
                                .unwrap_or(content)
                                .trim()
                                .to_string()
                        } else {
                            content.to_string()
                        };
                        let word_count = text.split_whitespace().count();
                        ExtractOutcome::Success { text, word_count }
                    }
                    Err(e) => {
                        return span.internal_error(serde_json::json!({
                            "error": format!("Failed to decode markdown file '{}': {}", path, e),
                        }));
                    }
                }
            }
            "html" | "htm" => match std::str::from_utf8(&file_bytes) {
                Ok(content) => {
                    let text = convert::strip_html(content);
                    let word_count = text.split_whitespace().count();
                    ExtractOutcome::Success { text, word_count }
                }
                Err(e) => {
                    return span.internal_error(serde_json::json!({
                        "error": format!("Failed to decode HTML file '{}': {}", path, e),
                    }));
                }
            },
            other => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Format '{}' is not supported for text extraction. Supported formats: pdf, markdown, html, plain. \
                         For DOCX/PPTX/XLSX/CSV/RTF, install the corresponding Rust crates. Path: '{}'",
                        other, path
                    ))
                    .to_json_string(),
                );
            }
        };

        match extract_result {
            ExtractOutcome::Success { text, word_count } => {
                let result = serde_json::json!({
                    "format": format,
                    "path": path,
                    "method": "text_extraction",
                    "text": text,
                    "word_count": word_count,
                });
                self.record_experience(
                    "markitdown_convert",
                    &path_clone,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            ExtractOutcome::NeedsOcr {
                partial_text,
                word_count,
            } => {
                // Fall back to OCR
                match self.resolve_ocr_model(None).await {
                    Ok(model) => {
                        match self
                            .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                            .await
                        {
                            Ok(ocr_text) => {
                                let ocr_word_count = ocr_text.split_whitespace().count();
                                // Use OCR result if it yielded more text than extraction
                                let (final_text, final_word_count, method) =
                                    if ocr_word_count > word_count {
                                        (ocr_text, ocr_word_count, "ocr")
                                    } else {
                                        (
                                            partial_text,
                                            word_count,
                                            "text_extraction_ocr_fallback_insufficient",
                                        )
                                    };
                                let result = serde_json::json!({
                                    "format": format,
                                    "path": path,
                                    "method": method,
                                    "model": model,
                                    "text": final_text,
                                    "word_count": final_word_count,
                                    "extraction_word_count": word_count,
                                });
                                self.record_experience(
                                    "markitdown_convert",
                                    &path_clone,
                                    "success",
                                    result.clone(),
                                );
                                span.ok_json(result)
                            }
                            Err(e) => {
                                // OCR also failed — return whatever text extraction got
                                if word_count > 0 {
                                    span.ok_json(serde_json::json!({
                                        "format": format,
                                        "path": path,
                                        "method": "text_extraction_ocr_failed",
                                        "text": partial_text,
                                        "word_count": word_count,
                                        "ocr_error": e,
                                    }))
                                } else {
                                    span.error(
                                        McpErrorKind::Unavailable,
                                        McpToolError::unavailable(format!(
                                            "Text extraction returned near-empty result and OCR failed: {}",
                                            e
                                        ))
                                        .to_json_string(),
                                    )
                                }
                            }
                        }
                    }
                    Err(guidance) => {
                        // No OCR model configured — return extraction result with warning
                        if word_count > 0 {
                            span.ok_json(serde_json::json!({
                                "format": format,
                                "path": path,
                                "method": "text_extraction_no_ocr_available",
                                "text": partial_text,
                                "word_count": word_count,
                                "ocr_available": false,
                                "ocr_guidance": guidance,
                            }))
                        } else {
                            span.error(
                                McpErrorKind::FailedPrecondition,
                                McpToolError::failed_precondition(format!(
                                    "PDF text extraction returned no text and no OCR model is configured. {}",
                                    guidance
                                ))
                                .to_json_string(),
                            )
                        }
                    }
                }
            }
        }
    }

    #[tool(
        description = "Detect the document format from a file path/extension. Returns format name, whether text extraction is supported, and note for unsupported formats."
    )]
    async fn markitdown_detect_format(
        &self,
        Parameters(DetectFormatRequest { path }): Parameters<DetectFormatRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("markitdown_detect_format", &self.webid);

        let format = convert::detect_format(&path);
        let supported = convert::is_format_supported(format);

        let note = if !supported && format != "unknown" {
            Some(format!(
                "Format '{}' is recognized but not yet supported for text extraction. Supported formats: pdf, markdown, html, plain",
                format
            ))
        } else {
            None
        };

        let mut result = serde_json::json!({
            "path": path,
            "format": format,
            "supported": supported,
        });
        if let Some(n) = note {
            result["note"] = serde_json::json!(n);
        }

        span.ok_json(result)
    }

    #[tool(
        description = "OCR a document using a local vision model. Requires HKASK_OCR_MODEL env var or explicit model parameter. The model must be a vision-capable model available in the inference catalog."
    )]
    async fn markitdown_ocr(
        &self,
        Parameters(OcrRequest {
            path,
            model,
            max_tokens,
        }): Parameters<OcrRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("markitdown_ocr", &self.webid);
        let path_clone = path.clone();
        validate_field!(span, "path", &path, 4096);

        let model = match self.resolve_ocr_model(model.as_deref()).await {
            Ok(m) => m,
            Err(guidance) => {
                return span.error(
                    McpErrorKind::FailedPrecondition,
                    McpToolError::failed_precondition(guidance).to_json_string(),
                );
            }
        };

        let file_bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                return span.internal_error(serde_json::json!({
                    "error": format!("Failed to read file '{}': {}", path, e),
                }));
            }
        };

        match self.do_ocr(&file_bytes, &model, max_tokens).await {
            Ok(text) => {
                let result = serde_json::json!({
                    "path": path,
                    "model": model,
                    "text": text,
                    "word_count": text.split_whitespace().count(),
                });
                self.record_experience("markitdown_ocr", &path_clone, "success", result.clone());
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(e).to_json_string(),
            ),
        }
    }
}

/// Internal outcome of text extraction, used to decide OCR fallback.
enum ExtractOutcome {
    /// Text extraction succeeded with sufficient content.
    Success { text: String, word_count: usize },
    /// Text extraction yielded too little — needs OCR fallback.
    NeedsOcr {
        partial_text: String,
        word_count: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::decimation;
    use crate::ocr::pipeline;

    /// Minimal valid PDF with one page containing "Hello World".
    fn minimal_pdf() -> Vec<u8> {
        b"%PDF-1.4\n\
          1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
          2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
          3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n\
          4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
          5 0 obj<</Length 44>>stream\n\
          BT /F1 24 Tf 100 700 Td (Hello World) Tj ET\n\
          endstream\n\
          endobj\n\
          xref\n\
          0 6\n\
          0000000000 65535 f \n\
          0000000009 00000 n \n\
          0000000058 00000 n \n\
          0000000115 00000 n \n\
          0000000277 00000 n \n\
          0000000349 00000 n \n\
          trailer<</Size 6/Root 1 0 R>>\n\
          startxref\n\
          441\n\
          %%EOF\n"
            .to_vec()
    }

    /// Check if pdftoppm is available.
    fn pdftoppm_available() -> bool {
        std::process::Command::new("pdftoppm")
            .arg("-v")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if tesseract is available.
    fn tesseract_available() -> bool {
        std::process::Command::new("tesseract")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn make_server() -> MarkitdownServer {
        MarkitdownServer::new(
            WebID::new(),
            "test-replicant".into(),
            None,
            None,
            InferenceConfig::from_env(),
            ThresholdConfig::default(),
            None,
        )
        .expect("failed to create test server")
    }

    // REQ:ocr-integration-01 — Pipeline runs successfully with a real PDF
    #[tokio::test]
    async fn pipeline_with_real_pdf() {
        if !pdftoppm_available() {
            eprintln!("SKIP: pdftoppm not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, minimal_pdf()).unwrap();

        let server = make_server();
        let t = server.ocr_thresholds;

        // Decimate and run full pipeline
        let images = decimation::pdf_to_images(&pdf_path, 150).unwrap();
        assert_eq!(images.len(), 1, "one-page PDF should produce one image");

        let outcome = pipeline::run_pipeline(images, 1, &server, &t, None, None).await;

        assert_eq!(outcome.results.len(), 1);
        assert!(!outcome.results[0].text.is_empty());
        assert!(outcome.report.page_count_match);
    }

    // REQ:ocr-integration-02 — Pipeline with tesseract backend
    #[tokio::test]
    async fn pipeline_with_tesseract_available() {
        if !pdftoppm_available() || !tesseract_available() {
            eprintln!("SKIP: pdftoppm or tesseract not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, minimal_pdf()).unwrap();

        let server = make_server();
        let t = server.ocr_thresholds;

        let images = decimation::pdf_to_images(&pdf_path, 150).unwrap();
        let outcome = pipeline::run_pipeline(images, 1, &server, &t, None, None).await;

        assert_eq!(outcome.results.len(), 1);
        // Tesseract should be used for Simple pages
        let result = &outcome.results[0];
        assert_eq!(result.backend, OcrBackend::Tesseract);
        assert!(!result.text.is_empty(), "tesseract should extract text");
        assert!(result.text.to_lowercase().contains("hello"));
    }

    // REQ:ocr-integration-03 — Verification catches missing pages
    #[tokio::test]
    async fn verification_catches_missing_pages() {
        if !pdftoppm_available() {
            eprintln!("SKIP: pdftoppm not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, minimal_pdf()).unwrap();

        let server = make_server();
        let t = server.ocr_thresholds;

        let images = decimation::pdf_to_images(&pdf_path, 150).unwrap();
        assert_eq!(images.len(), 1);

        // Pass expected_pages=2 but only 1 page exists
        let outcome = pipeline::run_pipeline(
            images, 2, // mismatch!
            &server, &t, None, None,
        )
        .await;

        assert!(!outcome.report.page_count_match);
        assert!(!outcome.report.passed);
    }

    // REQ:ocr-integration-04 — Empty PDF returns empty result
    #[tokio::test]
    async fn empty_pdf_is_handled() {
        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("empty.pdf");
        // Write a minimal empty-page PDF
        std::fs::write(
            &pdf_path,
            b"%PDF-1.4\n\
              1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
              2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
              3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj\n\
              xref\n\
              0 4\n\
              0000000000 65535 f \n\
              0000000009 00000 n \n\
              0000000058 00000 n \n\
              0000000115 00000 n \n\
              trailer<</Size 4/Root 1 0 R>>\n\
              startxref\n\
              167\n\
              %%EOF\n",
        )
        .unwrap();

        let server = make_server();
        let outcome = decimation::pdf_to_images(&pdf_path, 150);

        if pdftoppm_available() {
            // pdftoppm should produce an image for the empty page
            let images = outcome.unwrap();
            assert_eq!(images.len(), 1, "empty page should still produce an image");

            let outcome =
                pipeline::run_pipeline(images, 1, &server, &server.ocr_thresholds, None, None)
                    .await;

            // Empty page should flag as empty
            assert!(!outcome.report.empty_pages.is_empty());
        } else {
            assert!(outcome.is_err());
        }
    }
}
