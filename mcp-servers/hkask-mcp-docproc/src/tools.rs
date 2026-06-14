//! MCP tool implementations for docproc server.
//!
//! 8 tools:
//! - `docproc_convert` — Extract text from documents with OCR fallback
//! - `docproc_ocr` — Explicit OCR using vision model
//! - `docproc_chunk` — Chunk text or documents into passages (single or multi-tier)
//! - `docproc_extract_triples` — Extract RDF triples from text via LLM
//! - `docproc_embed` — Generate embedding vectors for passages or triples
//! - `docproc_generate_qa` — Generate QA pairs from text via LLM
//! - `docproc_store_qa` — Store QA items with provenance
//! - `docproc_cache` — Cache processed text for reference

use crate::convert;
use crate::ocr::{decimation, pipeline};
use crate::server::{DocProcServer, OCR_FALLBACK_WORD_THRESHOLD, default_ocr_max_tokens};
use hkask_inference::InferenceRouter;
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::SemanticMemory;
use hkask_storage::Triple;
use hkask_types::LLMParameters;
use hkask_types::McpErrorKind;
use hkask_types::Visibility;
use hkask_types::ports::InferencePort;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Approximate token-to-word conversion: 1 token ≈ 1.33 words.
fn tokens_to_words(tokens: usize) -> usize {
    ((tokens as f64) * 1.33) as usize
}

/// Compute (max_words, min_words) from (max_tokens, overlap_tokens) with defaults.
fn chunk_word_bounds(max_tokens: Option<usize>, overlap_tokens: Option<usize>) -> (usize, usize) {
    let max_w = tokens_to_words(max_tokens.unwrap_or(512));
    let min_w = tokens_to_words(overlap_tokens.unwrap_or(64)).max(max_w / 4);
    (max_w, min_w)
}

/// Serialize (entity_ref, text) pair vec into json.
fn serialize_passages(passages: Vec<(String, String)>) -> Vec<serde_json::Value> {
    passages
        .into_iter()
        .map(|(entity_ref, passage_text)| json!({"entity_ref": entity_ref, "text": passage_text}))
        .collect()
}

/// Validate a non-empty field; returns error JSON or continues.
macro_rules! validate_non_empty {
    ($span:expr, $kind:expr, $field_name:expr, $value:expr) => {
        if $value.is_empty() {
            return $span.error(
                $kind,
                McpToolError::invalid_argument(concat!($field_name, " must not be empty"))
                    .to_json_string(),
            );
        }
    };
}

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
pub struct OcrRequest {
    /// Path to the document file to OCR.
    pub path: String,
    /// Vision model to use for OCR (must be available in the inference catalog).
    #[serde(default)]
    pub model: Option<String>,
    /// Maximum tokens for OCR output.
    #[serde(default = "default_ocr_max_tokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChunkRequest {
    /// Raw text to chunk. Mutually exclusive with `path`.
    #[serde(default)]
    pub text: Option<String>,
    /// Path to a document file to extract text from and chunk. Mutually exclusive with `text`.
    #[serde(default)]
    pub path: Option<String>,
    /// Prefix for entity references in chunk output.
    pub entity_ref_prefix: String,
    /// Max tokens per chunk (single-tier mode, default 512).
    #[serde(default)]
    pub max_tokens: Option<usize>,
    /// Overlap tokens between chunks (single-tier mode, default 64).
    #[serde(default)]
    pub overlap_tokens: Option<usize>,
    /// Strip Project Gutenberg headers from text before chunking.
    #[serde(default)]
    pub strip_gutenberg: Option<bool>,
    /// If true, produce coarse/medium/fine multi-tier output instead of single-tier.
    #[serde(default)]
    pub multi_tier: Option<bool>,
    /// Max tokens for coarse tier (multi-tier mode, default 2048).
    #[serde(default)]
    pub coarse_max_tokens: Option<usize>,
    /// Max tokens for medium tier (multi-tier mode, default 512).
    #[serde(default)]
    pub medium_max_tokens: Option<usize>,
    /// Max tokens for fine tier (multi-tier mode, default 128).
    #[serde(default)]
    pub fine_max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateQaRequest {
    pub text: String,
    pub chunk_id: String,
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QaItem {
    pub question: String,
    pub answer: String,
    #[serde(default)]
    pub bloom_level: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreQaRequest {
    pub qa_items: Vec<QaItem>,
    pub source_document: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractTriplesRequest {
    /// Text to extract RDF triples from.
    pub text: String,
    /// Optional entity namespace prefix (e.g., "doc:myfile").
    #[serde(default)]
    pub namespace: Option<String>,
    /// Maximum triples to extract (default 50).
    #[serde(default)]
    pub max_triples: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmbedRequest {
    /// Texts to embed (passages or triple strings).
    pub texts: Vec<String>,
    /// Embedding model to use. If not set, uses the configured default.
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CacheRequest {
    /// Text content to cache.
    pub content: String,
    /// Label/key for the cached entry.
    pub label: String,
}

// ── Extract outcome enum ─────────────────────────────────────────────────

enum ExtractOutcome {
    Success {
        text: String,
        word_count: usize,
    },
    NeedsOcr {
        partial_text: String,
        word_count: usize,
    },
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl DocProcServer {
    #[tool(
        description = "Extract text from a document. Detects format, extracts text with automatic OCR fallback for scanned/image-based PDFs. For PDF: tries text extraction first, falls back to vision OCR if result is near-empty. For other supported formats (TXT, MD, HTML): extracts plain text. Requires HKASK_OCR_MODEL for OCR fallback."
    )]
    async fn docproc_convert(
        &self,
        Parameters(ConvertRequest { path, force_ocr }): Parameters<ConvertRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_convert", &self.webid);
        let path_clone = path.clone();
        validate_field!(span, "path", &path, 4096);

        let (format, _, _) = convert::detect_format(&path);

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
        if force_ocr {
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

                self.persist_pipeline_outcome(&outcome).await;

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
                self.record_experience("docproc_convert", &path_clone, "success", result.clone());
                return span.ok_json(result);
            }

            // Not an image — try decimation + pipeline for PDFs
            if format == "pdf" {
                match decimation::pdf_to_images(std::path::Path::new(&path), 200).await {
                    Ok(page_images) => {
                        let model = match self.resolve_ocr_model(None).await {
                            Ok(m) => m,
                            Err(guidance) => {
                                return span.error(
                                    McpErrorKind::FailedPrecondition,
                                    McpToolError::failed_precondition(guidance).to_json_string(),
                                );
                            }
                        };
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
                        self.persist_pipeline_outcome(&outcome).await;
                        let text = outcome
                            .results
                            .iter()
                            .map(|r| r.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n\n");
                        let result = serde_json::json!({
                            "format": format, "path": path, "method": "ocr_pipeline",
                            "model": model, "text": text,
                            "word_count": text.split_whitespace().count(),
                            "pages": expected,
                            "verification_passed": outcome.report.passed,
                            "page_count_match": outcome.report.page_count_match,
                            "empty_pages": outcome.report.empty_pages,
                            "error_count": outcome.errors.len(),
                        });
                        self.record_experience(
                            "docproc_convert",
                            &path_clone,
                            "success",
                            result.clone(),
                        );
                        return span.ok_json(result);
                    }
                    Err(_) => {
                        // Decimation failed — fall through to do_ocr
                    }
                }
            }

            // Final fallback: raw bytes OCR
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
                            "docproc_convert",
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

        // ── Text extraction path ──

        let extract_result = match format {
            "pdf" => {
                // Try typed pipeline with decimation first (if OCR model is configured)
                if let Ok(model) = self.resolve_ocr_model(None).await
                    && let Ok(page_images) =
                        decimation::pdf_to_images(std::path::Path::new(&path), 200).await
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

                    self.persist_pipeline_outcome(&outcome).await;

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
                        "docproc_convert",
                        &path_clone,
                        "success",
                        result.clone(),
                    );
                    return span.ok_json(result);
                }

                // Try pdf-extract first; fall back to OCR if near-empty
                match pdf_extract::extract_text(&path) {
                    Ok(text) => {
                        let word_count = text.split_whitespace().count();
                        if word_count < OCR_FALLBACK_WORD_THRESHOLD {
                            ExtractOutcome::NeedsOcr {
                                partial_text: text,
                                word_count,
                            }
                        } else {
                            ExtractOutcome::Success { text, word_count }
                        }
                    }
                    Err(_) => ExtractOutcome::NeedsOcr {
                        partial_text: String::new(),
                        word_count: 0,
                    },
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
            "markdown" => match std::str::from_utf8(&file_bytes) {
                Ok(content) => {
                    let text = convert::strip_frontmatter(content);
                    let word_count = text.split_whitespace().count();
                    ExtractOutcome::Success { text, word_count }
                }
                Err(e) => {
                    return span.internal_error(serde_json::json!({
                        "error": format!("Failed to decode markdown file '{}': {}", path, e),
                    }));
                }
            },
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
                self.record_experience("docproc_convert", &path_clone, "success", result.clone());
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
                                    "docproc_convert",
                                    &path_clone,
                                    "success",
                                    result.clone(),
                                );
                                span.ok_json(result)
                            }
                            Err(e) => {
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
        description = "OCR a document using a local vision model. Requires HKASK_OCR_MODEL env var or explicit model parameter. The model must be a vision-capable model available in the inference catalog."
    )]
    async fn docproc_ocr(
        &self,
        Parameters(OcrRequest {
            path,
            model,
            max_tokens,
        }): Parameters<OcrRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_ocr", &self.webid);
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
                self.record_experience("docproc_ocr", &path_clone, "success", result.clone());
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(e).to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Chunk text into passages at configurable token granularity. Accepts raw text or a file path (extracts text from PDF/MD/HTML/TXT with OCR fallback for scanned PDFs). Supports single-tier or multi-tier (coarse/medium/fine) output."
    )]
    async fn docproc_chunk(
        &self,
        Parameters(ChunkRequest {
            text,
            path,
            entity_ref_prefix,
            max_tokens,
            overlap_tokens,
            strip_gutenberg,
            multi_tier,
            coarse_max_tokens,
            medium_max_tokens,
            fine_max_tokens,
        }): Parameters<ChunkRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_chunk", &self.webid);

        // Exactly one of text or path must be provided
        let has_text = text.as_ref().is_some_and(|t| !t.is_empty());
        let has_path = path.as_ref().is_some_and(|p| !p.is_empty());
        if has_text == has_path {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("Exactly one of 'text' or 'path' must be provided")
                    .to_json_string(),
            );
        }

        validate_non_empty!(
            span,
            McpErrorKind::InvalidArgument,
            "entity_ref_prefix",
            entity_ref_prefix
        );
        validate_field!(span, "entity_ref_prefix", &entity_ref_prefix, 256);

        // Resolve the source text
        let source_text: String;
        let source_label: String;

        if let Some(ref raw_text) = text
            && !raw_text.is_empty()
        {
            source_text = raw_text.clone();
            source_label = entity_ref_prefix.clone();
        } else if let Some(ref file_path) = path
            && !file_path.is_empty()
        {
            let (format, supported, _) = convert::detect_format(file_path);
            if !supported {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Unsupported document format '{}' for path '{}'. Supported formats: pdf, markdown, html, plain",
                        format, file_path
                    ))
                    .to_json_string(),
                );
            }

            source_text = match format {
                "pdf" => match pdf_extract::extract_text(file_path) {
                    Ok(t) => {
                        let wc = t.split_whitespace().count();
                        if wc < OCR_FALLBACK_WORD_THRESHOLD {
                            if let Ok(model) = self.resolve_ocr_model(None).await {
                                let file_bytes = match std::fs::read(file_path) {
                                    Ok(b) => b,
                                    Err(e) => {
                                        return span.internal_error(serde_json::json!({
                                                "error": format!("Failed to read file '{}': {}", file_path, e),
                                            }));
                                    }
                                };
                                match self
                                    .do_ocr(&file_bytes, &model, default_ocr_max_tokens())
                                    .await
                                {
                                    Ok(ocr_text) => ocr_text,
                                    Err(_) => t,
                                }
                            } else {
                                t
                            }
                        } else {
                            t
                        }
                    }
                    Err(_) => {
                        return span.internal_error(serde_json::json!({
                            "error": format!("Failed to extract text from PDF '{}'", file_path),
                        }));
                    }
                },
                "markdown" => match std::fs::read_to_string(file_path) {
                    Ok(content) => convert::strip_frontmatter(&content),
                    Err(e) => {
                        return span.internal_error(serde_json::json!({
                            "error": format!("Failed to read file '{}': {}", file_path, e),
                        }));
                    }
                },
                "html" | "htm" => match std::fs::read_to_string(file_path) {
                    Ok(content) => convert::strip_html(&content),
                    Err(e) => {
                        return span.internal_error(serde_json::json!({
                            "error": format!("Failed to read file '{}': {}", file_path, e),
                        }));
                    }
                },
                _ => match std::fs::read_to_string(file_path) {
                    Ok(content) => content,
                    Err(e) => {
                        return span.internal_error(serde_json::json!({
                            "error": format!("Failed to read file '{}': {}", file_path, e),
                        }));
                    }
                },
            };
            source_label = file_path.replace(['/', '\\', '.', ' '], "_");
        } else {
            // Unreachable — validated above
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("No text or path provided").to_json_string(),
            );
        }

        // Apply Gutenberg stripping if requested
        let processed = if strip_gutenberg.unwrap_or(false) {
            SemanticMemory::strip_gutenberg_headers(&source_text)
        } else {
            source_text
        };

        let boundary = ".!? ";

        if multi_tier.unwrap_or(false) {
            // Multi-tier: coarse / medium / fine
            let chunk_tier = |tier: &str, max_tok: Option<usize>, default: usize| -> Vec<_> {
                let w = tokens_to_words(max_tok.unwrap_or(default));
                SemanticMemory::chunk_text(
                    &processed,
                    &format!("{source_label}:{tier}"),
                    w / 4,
                    w,
                    boundary,
                )
            };

            let coarse = chunk_tier("coarse", coarse_max_tokens, 2048);
            let medium = chunk_tier("medium", medium_max_tokens, 512);
            let fine = chunk_tier("fine", fine_max_tokens, 128);

            let result = json!({
                "source": source_label,
                "multi_tier": true,
                "coarse_max_tokens": coarse_max_tokens.unwrap_or(2048),
                "medium_max_tokens": medium_max_tokens.unwrap_or(512),
                "fine_max_tokens": fine_max_tokens.unwrap_or(128),
                "coarse": serialize_passages(coarse),
                "medium": serialize_passages(medium),
                "fine": serialize_passages(fine),
            });
            self.record_experience("docproc_chunk", &source_label, "success", result.clone());
            span.ok_json(result)
        } else {
            // Single-tier
            let (max_words, min_words) = chunk_word_bounds(max_tokens, overlap_tokens);

            let passages = SemanticMemory::chunk_text(
                &processed,
                &entity_ref_prefix,
                min_words,
                max_words,
                boundary,
            );

            let total_passages = passages.len();
            let serialized = serialize_passages(passages);

            let result = json!({
                "source": source_label,
                "multi_tier": false,
                "total_passages": total_passages,
                "passages": serialized,
                "max_tokens": max_tokens.unwrap_or(512),
                "overlap_tokens": overlap_tokens.unwrap_or(64),
                "max_words": max_words,
                "min_words": min_words,
                "sentence_boundary": boundary,
                "stripped_gutenberg": strip_gutenberg.unwrap_or(false),
            });
            self.record_experience("docproc_chunk", &source_label, "success", result.clone());
            span.ok_json(result)
        }
    }

    #[tool(
        description = "Generate QA pairs from a text chunk by calling the inference engine. Returns structured question-answer pairs at specified Bloom's taxonomy levels."
    )]
    async fn docproc_generate_qa(
        &self,
        Parameters(GenerateQaRequest {
            text,
            chunk_id,
            strategy,
            bloom_levels,
        }): Parameters<GenerateQaRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_generate_qa", &self.webid);

        if text.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("text must not be empty").to_json_string(),
            );
        }

        if chunk_id.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("chunk_id must not be empty").to_json_string(),
            );
        }

        let strat = strategy.unwrap_or_else(|| "default".to_string());
        let levels =
            bloom_levels.unwrap_or_else(|| vec!["factual".to_string(), "conceptual".to_string()]);

        let levels_str = levels.join(", ");
        let prompt = format!(
            "Based on the following text, generate question-answer pairs at these Bloom's taxonomy levels: {levels_str}.\n\n\
             Text (chunk {chunk_id}):\n{text}\n\n\
             For each level, provide:\n\
             - A question that tests understanding at that level\n\
             - A concise, accurate answer derived from the text\n\
             - The bloom_level classification\n\n\
             Respond in JSON format: {{\"qa_pairs\": [{{\"question\": \"...\", \"answer\": \"...\", \"bloom_level\": \"...\"}}]}}"
        );

        // Call the inference engine to generate QA pairs
        let router = InferenceRouter::new(self.inference_config.clone());
        let params = LLMParameters {
            temperature: 0.3,
            max_tokens: 4096,
            ..Default::default()
        };

        match router.generate(&prompt, &params).await {
            Ok(response) => {
                // Try to parse the JSON response
                let qa_pairs: serde_json::Value = match serde_json::from_str(&response.text) {
                    Ok(v) => v,
                    Err(_) => {
                        // If parsing fails, return raw text with a note
                        json!({"raw_response": response.text, "parse_error": "LLM response was not valid JSON"})
                    }
                };

                let result = json!({
                    "chunk_id": chunk_id,
                    "strategy": strat,
                    "bloom_levels": levels,
                    "qa_pairs": qa_pairs,
                    "tokens_used": response.usage.total_tokens,
                });
                self.record_experience("docproc_generate_qa", &chunk_id, "success", result.clone());
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("QA generation failed: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(description = "Store QA items with provenance")]
    async fn docproc_store_qa(
        &self,
        Parameters(StoreQaRequest {
            qa_items,
            source_document,
        }): Parameters<StoreQaRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_store_qa", &self.webid);
        let source_doc = source_document.clone();

        let Some(semantic) = &self.semantic else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                )
                .to_json_string(),
            );
        };

        if qa_items.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("qa_items must not be empty").to_json_string(),
            );
        }

        validate_field!(span, "source_document", &source_document, 256);

        let mut stored = 0;
        let mut errors = Vec::new();

        for (i, qa) in qa_items.iter().enumerate() {
            let entity = format!("qa:{source_document}:{i}");
            let level = qa.bloom_level.as_deref().unwrap_or("factual");
            let value = json!({
                "question": qa.question,
                "answer": qa.answer,
                "bloom_level": level,
                "source_document": source_document,
            });

            let triple = Triple::new(&entity, "qa_pair", value, self.webid)
                .with_visibility(Visibility::Public)
                .with_confidence(1.0);

            match semantic.store(triple) {
                Ok(()) => stored += 1,
                Err(e) => errors.push(format!("Item {i}: {e}")),
            }
        }

        if errors.is_empty() {
            let result = json!({ "stored": stored, "source_document": source_document });
            self.record_experience("docproc_store_qa", &source_doc, "success", result.clone());
            span.ok_json(result)
        } else {
            let result =
                json!({ "stored": stored, "errors": errors, "source_document": source_document });
            self.record_experience("docproc_store_qa", &source_doc, "partial", result.clone());
            span.internal_error(result)
        }
    }

    #[tool(
        description = "Extract RDF triples (subject, predicate, object) from text using the inference engine. Returns structured knowledge triples with confidence scores."
    )]
    async fn docproc_extract_triples(
        &self,
        Parameters(ExtractTriplesRequest {
            text,
            namespace,
            max_triples,
        }): Parameters<ExtractTriplesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_extract_triples", &self.webid);

        if text.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("text must not be empty").to_json_string(),
            );
        }

        let ns = namespace.unwrap_or_else(|| "doc".to_string());
        let limit = max_triples.unwrap_or(50);

        let prompt = format!(
            "Extract up to {limit} factual RDF triples from the following text.\n\n\
             Each triple should be in the form (subject, predicate, object) where:\n\
             - subject: an entity mentioned in the text (prefix with '{ns}:')\n\
             - predicate: a relationship or property (use standard RDF predicates like rdf:type, schema:name, etc.)\n\n\
             - object: another entity, a literal value, or a type\n\n\
             For each triple, also provide a confidence score (0.0-1.0) based on how clearly the text supports it.\n\n\
             Text:\n{text}\n\n\
             Respond in JSON format: {{\"triples\": [{{\"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.95}}]}}"
        );

        let router = InferenceRouter::new(self.inference_config.clone());
        let params = LLMParameters {
            temperature: 0.1,
            max_tokens: 4096,
            ..Default::default()
        };

        match router.generate(&prompt, &params).await {
            Ok(response) => {
                let triples: serde_json::Value = match serde_json::from_str(&response.text) {
                    Ok(v) => v,
                    Err(_) => {
                        json!({"raw_response": response.text, "parse_error": "LLM response was not valid JSON"})
                    }
                };

                let result = json!({
                    "namespace": ns,
                    "max_triples": limit,
                    "triples": triples,
                    "tokens_used": response.usage.total_tokens,
                });
                self.record_experience("docproc_extract_triples", &ns, "success", result.clone());
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Triple extraction failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Generate embedding vectors for a list of texts (passages or triples). Uses the configured embedding model via the inference router."
    )]
    async fn docproc_embed(
        &self,
        Parameters(EmbedRequest { texts, model }): Parameters<EmbedRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_embed", &self.webid);

        if texts.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("texts must not be empty").to_json_string(),
            );
        }

        let Some(ref emb_router) = self.embedding_router else {
            return span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition(
                    "Embedding router not configured — inference config may be missing",
                )
                .to_json_string(),
            );
        };

        let model_name = model.unwrap_or_else(|| {
            std::env::var("HKASK_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string())
        });

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        match emb_router.embed_sentences(&model_name, &text_refs).await {
            Ok(vectors) => {
                let result = json!({
                    "count": texts.len(),
                    "dimensions": vectors.first().map(|v| v.len()).unwrap_or(0),
                    "vectors": vectors,
                    "model": model_name,
                });
                self.record_experience(
                    "docproc_embed",
                    &format!("{} texts", texts.len()),
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Embedding failed: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Cache processed document text for reference. Stores content keyed by label in the docproc cache directory (~/.config/hkask/docproc-cache/)."
    )]
    async fn docproc_cache(
        &self,
        Parameters(CacheRequest { content, label }): Parameters<CacheRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_cache", &self.webid);

        if content.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("content must not be empty").to_json_string(),
            );
        }

        if label.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("label must not be empty").to_json_string(),
            );
        }

        // Resolve cache directory
        let cache_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hkask")
            .join("docproc-cache");

        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            return span.internal_error(json!({
                "error": format!("Failed to create cache directory '{}': {}", cache_dir.display(), e),
            }));
        }

        // Sanitize label for filesystem
        let safe_label: String = label
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let cache_path = cache_dir.join(format!("{}.md", safe_label));

        match std::fs::write(&cache_path, &content) {
            Ok(()) => {
                let result = json!({
                    "label": label,
                    "path": cache_path.display().to_string(),
                    "size_bytes": content.len(),
                });
                self.record_experience("docproc_cache", &label, "success", result.clone());
                span.ok_json(result)
            }
            Err(e) => span.internal_error(json!({
                "error": format!("Failed to write cache file '{}': {}", cache_path.display(), e),
            })),
        }
    }
}
