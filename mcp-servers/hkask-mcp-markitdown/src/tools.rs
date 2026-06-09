//! MCP tools for document format conversion and OCR
//!
//! Three tools exposed via MCP protocol:
//! - `markitdown_convert` — Detect format, extract text, OCR fallback for scanned docs
//! - `markitdown_detect_format` — Detect document format from path/extension
//! - `markitdown_ocr` — Explicitly OCR a document using local vision model
//!
//! OCR requires `HKASK_OCR_MODEL` to be set to a vision-capable model name
//! that exists in the Okapi model catalog (e.g., a model with vision support).
//! Use `InferenceService::list_models()` to discover available models.

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_templates::{OkapiConfig, OkapiInference};
use hkask_types::{LLMParameters, McpErrorKind, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::convert;

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
    /// Vision model to use for OCR (must be available in Okapi catalog).
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
    /// Configured OCR model (from HKASK_OCR_MODEL env var). None means OCR is unavailable.
    ocr_model: Option<String>,
    /// Okapi configuration for inference.
    okapi_config: OkapiConfig,
}

impl MarkitdownServer {
    pub fn new(
        webid: WebID,
        ocr_model: Option<String>,
        okapi_base_url: &str,
    ) -> anyhow::Result<Self> {
        let okapi_config = OkapiConfig {
            base_url: okapi_base_url.to_string(),
            ..OkapiConfig::default()
        };
        Ok(Self {
            webid,
            ocr_model,
            okapi_config,
        })
    }

    /// Resolve OCR model: explicit override > HKASK_OCR_MODEL env.
    /// Returns error guidance if no model is configured.
    fn resolve_ocr_model(&self, override_model: Option<&str>) -> Result<String, String> {
        if let Some(model) = override_model
            && !model.is_empty()
        {
            return Ok(model.to_string());
        }
        self.ocr_model
            .clone()
            .ok_or_else(|| {
                "No OCR model configured. Set HKASK_OCR_MODEL env var to a vision-capable model from the Okapi catalog, or pass the 'model' parameter. Use inference_models to discover available models.".to_string()
            })
    }

    /// Perform OCR by sending base64-encoded bytes to a vision model.
    ///
    /// Reuses the server's `OkapiInference` client with `model_override` so we
    /// don't allocate a new HTTP client per call.
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

        let inference = OkapiInference::new(
            &self.ocr_model.clone().unwrap_or_default(),
            self.okapi_config.clone(),
        )
        .map_err(|e| format!("Failed to create inference client: {}", e))?;

        let params = LLMParameters {
            temperature: 0.1, // Low temperature for faithful extraction
            max_tokens,
            ..Default::default()
        };

        let result = inference
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], Some(model), None, &params)
            .await
            .map_err(|e| format!("OCR inference failed: {}", e))?;

        Ok(result.text)
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

        // When force_ocr is set, skip text extraction entirely
        if force_ocr {
            match self.resolve_ocr_model(None) {
                Ok(model) => match self
                    .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                    .await
                {
                    Ok(text) => {
                        return span.ok_json(serde_json::json!({
                            "format": format,
                            "path": path,
                            "method": "ocr",
                            "model": model,
                            "text": text,
                            "word_count": text.split_whitespace().count(),
                        }));
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
            ExtractOutcome::Success { text, word_count } => span.ok_json(serde_json::json!({
                "format": format,
                "path": path,
                "method": "text_extraction",
                "text": text,
                "word_count": word_count,
            })),
            ExtractOutcome::NeedsOcr {
                partial_text,
                word_count,
            } => {
                // Fall back to OCR
                match self.resolve_ocr_model(None) {
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
                                span.ok_json(serde_json::json!({
                                    "format": format,
                                    "path": path,
                                    "method": method,
                                    "model": model,
                                    "text": final_text,
                                    "word_count": final_word_count,
                                    "extraction_word_count": word_count,
                                }))
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
        description = "OCR a document using a local vision model. Requires HKASK_OCR_MODEL env var or explicit model parameter. The model must be a vision-capable model available in the Okapi catalog."
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
        validate_field!(span, "path", &path, 4096);

        let model = match self.resolve_ocr_model(model.as_deref()) {
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
            Ok(text) => span.ok_json(serde_json::json!({
                "path": path,
                "model": model,
                "text": text,
                "word_count": text.split_whitespace().count(),
            })),
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
