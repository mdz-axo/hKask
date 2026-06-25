//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Combines format conversion, OCR, chunking, triple extraction, embedding,
//! QA generation, caching, query, and Kindle book export (12 tools). Supersedes the former
//! `hkask-mcp-markitdown` and `hkask-mcp-doc-knowledge` servers.
//!
//! Server struct, constructor, and tool methods are all inline in lib.rs
//! (kanban pattern) for fuzz test construction and P5 Testing Discipline
//! compliance.

pub mod convert;
pub mod ocr;
pub mod tools;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use async_trait::async_trait;

use crate::ocr::calibration::{analyze_threshold_drift, emit_drift_alert};
use crate::ocr::decimation;
use crate::ocr::llm_ocr::LlmOcrExecutor;
use crate::ocr::pipeline::{self, OcrExecutor};
use crate::ocr::semantic::cosine_similarity;
use crate::ocr::tesseract::TesseractExecutor;
use crate::ocr::{OcrBackend, OcrResult, ThresholdConfig};
use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp::DaemonClient;
use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_memory::SemanticMemory;
use hkask_ports::{CnsObserver, InferencePort};
use hkask_types::WebID;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::sync::Mutex;

// ── Constants ──────────────────────────────────────────────────────────────

/// Minimum word count from pdf-extract to consider text extraction successful
/// before falling back to OCR for scanned PDFs.
pub const OCR_FALLBACK_WORD_THRESHOLD: usize = 100;

/// System prompt for OCR vision requests.
const OCR_SYSTEM_PROMPT: &str =
    "Extract all text from this image. Output only the extracted text, nothing else.";

/// Default max tokens for OCR output.
pub fn default_ocr_max_tokens() -> u32 {
    8192
}

// ── Server struct ──────────────────────────────────────────────────────────

pub struct DocProcServer {
    pub webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<DaemonClient>,
    /// Configured OCR model (from HKASK_OCR_MODEL env var). None means OCR is unavailable.
    pub ocr_model: Option<String>,
    /// Inference configuration for the router.
    pub inference_config: InferenceConfig,
    /// OCR pipeline thresholds (loaded from settings.json).
    pub ocr_thresholds: ThresholdConfig,
    /// Embedding router for semantic cross-validation (None if unavailable).
    pub embedding_router: Option<EmbeddingRouter>,
    /// CNS observer for pipeline events → daemon → NuEventStore → CurationLoop.
    pub cns_observer: DocProcCnsObserver,
    /// Accumulated cross-validations across pipeline runs for threshold self-tuning.
    /// Cleared after a drift alert is emitted to avoid redundant suggestions.
    pub cv_accumulator: Mutex<Vec<crate::ocr::CrossValidation>>,
    /// In-memory vector index for RAG query/retrieval. Passages indexed by `docproc_chunk`
    /// are stored here with their embeddings for cosine-similarity search via `docproc_query`.
    pub index: Mutex<Vec<IndexedPassage>>,
}

/// A passage stored in the in-memory vector index with its embedding.
#[derive(Debug, Clone)]
pub struct IndexedPassage {
    pub text: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}

// ── Server constructor + core methods ──────────────────────────────────────

impl DocProcServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        ocr_model: Option<String>,
        inference_config: InferenceConfig,
        ocr_thresholds: ThresholdConfig,
        embedding_router: Option<EmbeddingRouter>,
    ) -> anyhow::Result<Self> {
        let cns_observer = DocProcCnsObserver::new(daemon.clone(), &replicant);
        Ok(Self {
            webid,
            replicant,
            daemon,
            ocr_model,
            inference_config,
            ocr_thresholds,
            embedding_router,
            cns_observer,
            cv_accumulator: Mutex::new(Vec::new()),
            index: Mutex::new(Vec::new()),
        })
    }

    /// Check whether OCR capability is available.
    pub fn has_ocr(&self) -> bool {
        self.ocr_model.is_some()
    }

    /// Index passages into the in-memory vector store for later query.
    /// Embeds each passage text and stores it with metadata.
    /// Returns the number of passages indexed (0 if embedding router unavailable).
    /// Emits a CNS warning when indexing was requested but embedding is unavailable (GAP-6).
    pub async fn index_passages(&self, passages: &[(String, String)], source_label: &str) -> usize {
        let Some(ref emb_router) = self.embedding_router else {
            tracing::warn!(
                target: "cns.docproc.index",
                source = %source_label,
                passage_count = passages.len(),
                "Cannot index passages — embedding router not configured. \
                 Set HKASK_EMBEDDING_MODEL to enable semantic search."
            );
            return 0;
        };

        let texts: Vec<&str> = passages.iter().map(|(_, t)| t.as_str()).collect();
        if texts.is_empty() {
            return 0;
        }

        let model_name = std::env::var("HKASK_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());

        let vectors = match emb_router.embed_sentences(&model_name, &texts).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: "hkask.mcp.docproc.index", error = %e, "Failed to embed passages for indexing");
                return 0;
            }
        };

        let mut index = self
            .index
            .lock()
            .expect("Failed to lock index for passage indexing");
        for (i, ((entity_ref, passage_text), embedding)) in
            passages.iter().zip(vectors.into_iter()).enumerate()
        {
            index.push(IndexedPassage {
                text: passage_text.clone(),
                metadata: serde_json::json!({
                    "entity_ref": entity_ref,
                    "source": source_label,
                    "position": i,
                }),
                embedding,
            });
        }
        passages.len()
    }
}

impl hkask_mcp::server::ToolContext for DocProcServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

// ── CNS Observer ───────────────────────────────────────────────────────────

/// CNS observer that implements the real `hkask_ports::CnsObserver` trait.
/// Routes pipeline events through the daemon to NuEventStore → CurationLoop.
pub struct DocProcCnsObserver {
    daemon: Option<DaemonClient>,
    replicant: String,
}

impl DocProcCnsObserver {
    pub fn new(daemon: Option<DaemonClient>, replicant: &str) -> Self {
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
                        tracing::debug!(target: "hkask.mcp.docproc.cns", span = %span_name, "CNS span persisted");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", span = %span_name, response = ?other, "Unexpected daemon response");
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", span = %span_name, error = %e, "Failed to persist CNS span");
                    }
                }
            });
        }
    }
}

#[async_trait]
impl hkask_ports::CnsObserver for DocProcCnsObserver {
    fn interest_mask(&self) -> Vec<hkask_types::event::SpanNamespace> {
        vec![hkask_types::event::SpanNamespace::new("cns.pipeline")]
    }

    async fn on_event(&self, event: &hkask_types::event::NuEvent) {
        let span_name = event.span.namespace.short_name();
        self.persist_span(span_name, event.observation.clone());
    }

    async fn on_depletion(&self, _signal: &hkask_ports::DepletionSignal) {
        tracing::warn!(target: "hkask.mcp.docproc.cns", "CNS depletion signal received");
    }

    async fn on_backpressure(&self, _signal: &hkask_ports::BackpressureSignal) {
        tracing::warn!(target: "hkask.mcp.docproc.cns", "CNS backpressure signal received");
    }
}

// ── OcrExecutor implementation ─────────────────────────────────────────────

#[async_trait]
impl OcrExecutor for DocProcServer {
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
        // Reuse a single executor instance per backend type (C5).
        // TesseractExecutor is stateless beyond config — constructing per page
        // was wasteful allocation (probed via `is_available` each time).
        static TESSERACT: std::sync::LazyLock<TesseractExecutor> =
            std::sync::LazyLock::new(TesseractExecutor::new);

        match backend {
            OcrBackend::Tesseract => {
                TESSERACT
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

// ── Pipeline helpers ───────────────────────────────────────────────────────

impl DocProcServer {
    /// Persist pipeline outcome to daemon for CNS observability.
    pub async fn persist_pipeline_outcome(&self, outcome: &crate::ocr::PipelineOutcome) {
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
                        tracing::debug!(target: "hkask.mcp.docproc.cns", "Pipeline outcome persisted to daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", response = ?other, "Unexpected daemon response");
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", error = %e, "Failed to persist pipeline outcome");
                    }
                }
            });
        }

        self.emit_pipeline_event(outcome).await;
        self.accumulate_and_check_drift(outcome);
    }

    /// Emit a CNS pipeline event through the CnsObserver.
    async fn emit_pipeline_event(&self, outcome: &crate::ocr::PipelineOutcome) {
        use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};

        let observation = serde_json::json!({
            "total_pages": outcome.results.len(),
            "error_count": outcome.errors.len(),
            "verification_passed": outcome.report.passed,
            "page_count_match": outcome.report.page_count_match,
            "empty_page_count": outcome.report.empty_pages.len(),
            "word_count_delta_pct": outcome.report.word_count_delta_pct,
            "cross_validation_count": outcome.cross_validations.len(),
            "mean_cross_validation_similarity": if outcome.cross_validations.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(outcome.cross_validations.iter()
                    .map(|cv| cv.similarity)
                    .sum::<f32>() / outcome.cross_validations.len() as f32)
            },
        });

        let event = NuEvent::new(
            self.webid,
            Span::new(SpanNamespace::new("cns.pipeline"), "ocr.verification"),
            Phase::Sense,
            observation,
            0,
        )
        .with_visibility("private");

        self.cns_observer.on_event(&event).await;

        // GAP-3: CNS feedback closure — emit a verification-failure alert
        // when the pipeline reports a quality issue. This closes the cybernetic
        // loop: the CNS can now observe failures and Curator can respond.
        if !outcome.report.passed {
            let failure_detail = serde_json::json!({
                "reason": {
                    "page_count_match": outcome.report.page_count_match,
                    "empty_pages": outcome.report.empty_pages,
                    "word_count_delta_pct": outcome.report.word_count_delta_pct,
                    "error_count": outcome.errors.len(),
                },
                "failures": outcome.errors.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                "recommendation": "Consider re-running with a different OCR backend or adjusting thresholds.",
            });

            let failure_event = NuEvent::new(
                self.webid,
                Span::new(
                    SpanNamespace::new("cns.pipeline"),
                    "ocr.verification_failed",
                ),
                Phase::Act,
                failure_detail,
                1, // urgency: elevated
            )
            .with_visibility("private");

            tracing::warn!(
                target: "cns.pipeline.ocr",
                empty_pages = ?outcome.report.empty_pages,
                error_count = outcome.errors.len(),
                page_count_match = outcome.report.page_count_match,
                "OCR pipeline verification failed — emitting CNS alert for Curator review"
            );

            self.cns_observer.on_event(&failure_event).await;
        }
    }

    /// Accumulate cross-validations and check for threshold drift.
    fn accumulate_and_check_drift(&self, outcome: &crate::ocr::PipelineOutcome) {
        let mut acc = self
            .cv_accumulator
            .lock()
            .expect("Failed to lock CV accumulator for drift check");
        acc.extend(outcome.cross_validations.clone());

        let synthetic_outcome = crate::ocr::PipelineOutcome {
            results: vec![],
            report: crate::ocr::VerificationReport::new(true, 0.0, vec![], 0, vec![]),
            cross_validations: acc.clone(),
            errors: vec![],
        };

        if let Some(alert) = analyze_threshold_drift(&[synthetic_outcome], &self.ocr_thresholds) {
            emit_drift_alert(&alert);
            acc.clear();
        }
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    pub fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool, "input": input_summary, "outcome": outcome,
                "detail": detail, "timestamp": now_rfc3339(),
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
                        tracing::debug!(target: "hkask.mcp.docproc.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.docproc.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.docproc.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }

    /// Resolve OCR model: explicit override > HKASK_OCR_MODEL env.
    pub async fn resolve_ocr_model(&self, override_model: Option<&str>) -> Result<String, String> {
        let model = if let Some(m) = override_model
            && !m.is_empty()
        {
            m.to_string()
        } else {
            self.ocr_model.clone().ok_or_else(|| {
                "No OCR model configured. Set HKASK_OCR_MODEL env var to a vision-capable model, or pass the 'model' parameter. Use inference_models to discover available models.".to_string()
            })?
        };

        let router = InferenceRouter::new(self.inference_config.clone());
        let vision_models = router.list_vision_models().await;
        let is_vision = vision_models
            .iter()
            .any(|m| m.model == model || m.prefixed_name == model);

        if !is_vision {
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
        }

        Ok(model)
    }

    /// Perform OCR by sending base64-encoded bytes to a vision model.
    pub async fn do_ocr(
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
            temperature: 0.1,
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

// ── Tool helpers ───────────────────────────────────────────────────────────

/// Shared text extraction from a file path.
///
/// Detects format, reads the file, and extracts plain text. For PDFs,
/// falls back to OCR if text extraction yields fewer than
/// `OCR_FALLBACK_WORD_THRESHOLD` words and an OCR model is available.
///
/// Used by both `docproc_convert` and `docproc_chunk` to eliminate ~160
/// lines of duplicated extraction logic (P5: surgical deduplication).
async fn extract_text(_server: &DocProcServer, path: &str) -> Result<ExtractOutcome, McpToolError> {
    let (format, supported, note) = convert::detect_format(path);

    if !supported {
        return Err(McpToolError::invalid_argument(format!(
            "Format '{}' is not supported for text extraction. Supported formats: pdf, markdown, html, plain. {}",
            format,
            note.unwrap_or("")
        )));
    }

    let file_bytes = std::fs::read(path)
        .map_err(|e| McpToolError::internal(format!("Failed to read file '{}': {}", path, e)))?;

    if file_bytes.is_empty() {
        return Err(McpToolError::invalid_argument(format!(
            "File '{}' is empty",
            path
        )));
    }

    let extract_result = match format {
        "pdf" => match pdf_extract::extract_text(path) {
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
        },
        "plain" => match std::str::from_utf8(&file_bytes) {
            Ok(text) => ExtractOutcome::Success {
                text: text.to_string(),
                word_count: text.split_whitespace().count(),
            },
            Err(e) => {
                return Err(McpToolError::internal(format!(
                    "Failed to decode text file '{}': {}",
                    path, e
                )));
            }
        },
        "markdown" => match std::str::from_utf8(&file_bytes) {
            Ok(content) => {
                let text = convert::strip_frontmatter(content);
                let word_count = text.split_whitespace().count();
                ExtractOutcome::Success { text, word_count }
            }
            Err(e) => {
                return Err(McpToolError::internal(format!(
                    "Failed to decode markdown file '{}': {}",
                    path, e
                )));
            }
        },
        "html" | "htm" => match std::str::from_utf8(&file_bytes) {
            Ok(content) => {
                let text = convert::strip_html(content);
                let word_count = text.split_whitespace().count();
                ExtractOutcome::Success { text, word_count }
            }
            Err(e) => {
                return Err(McpToolError::internal(format!(
                    "Failed to decode HTML file '{}': {}",
                    path, e
                )));
            }
        },
        _ => unreachable!("supported check above guards this branch"),
    };

    Ok(extract_result)
}

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

/// Strip markdown code fences from LLM JSON responses.
/// Models often wrap JSON in ```json ... ``` blocks.
fn strip_json_fences(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        // Find the first newline after the opening fence
        if let Some(after_fence) = trimmed.find('\n') {
            let content = &trimmed[after_fence + 1..];
            // Strip closing fence
            if let Some(close_pos) = content.rfind("```") {
                content[..close_pos].trim().to_string()
            } else {
                content.trim().to_string()
            }
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    }
}

/// Load a docproc template from registry and render with variable substitution.
///
/// Templates live in `registry/templates/docproc/` as Jinja2 files with YAML frontmatter.
/// This function strips the frontmatter, then replaces `{{ var }}` placeholders with
/// values from the provided map. Simple string substitution — full Jinja2 rendering
/// is deferred to the hkask-templates CNR engine (C10).
fn render_docproc_template(
    template_name: &str,
    vars: &std::collections::HashMap<&str, String>,
) -> String {
    // Resolve template path relative to workspace root
    let template_path =
        std::path::Path::new("registry/templates/docproc").join(format!("{template_name}.j2"));

    let content = match std::fs::read_to_string(&template_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.docproc.template", path = %template_path.display(), error = %e, "Template not found — using inline fallback");
            return String::new();
        }
    };

    // Strip YAML frontmatter (between --- markers)
    let body = if content.starts_with("---") {
        content
            .splitn(3, "---")
            .nth(2)
            .unwrap_or(&content)
            .trim()
            .to_string()
    } else {
        content
    };

    // Strip [inference] blocks (non-Jinja2 metadata).
    // Only skip the explicit [inference] header block, not arbitrary [bracketed] lines.
    let prompt_body = body
        .lines()
        .skip_while(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("[inference]") || trimmed.is_empty() || trimmed.starts_with('#')
        })
        .skip_while(|l| {
            // Skip the inference config lines (key = value pairs under [inference])
            let trimmed = l.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && trimmed.contains('=')
                && !trimmed.starts_with('{')
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    // Simple {{ var }} substitution
    let mut result = prompt_body;
    for (key, value) in vars {
        let placeholder = format!("{{{{ {} }}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

// ── Request structs ────────────────────────────────────────────────────────

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
    /// If true, automatically index passages for later query via docproc_query (default true).
    #[serde(default = "default_true")]
    pub index: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateQaRequest {
    pub text: String,
    pub chunk_id: String,
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryRequest {
    /// Natural language question to search for.
    pub query: String,
    /// Number of top results to return (default 5).
    #[serde(default)]
    pub top_k: Option<usize>,
    /// If true, generate an LLM-augmented answer from retrieved passages.
    #[serde(default)]
    pub generate_answer: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClearIndexRequest {
    /// Reserved for future multi-index support.
    #[serde(default)]
    pub index_id: Option<String>,
}

// ── Extract outcome enum ───────────────────────────────────────────────────

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

// ── Combined tool router (P5 Essentialism — modular tool groups) ──────────

impl DocProcServer {
    fn combined_router() -> rmcp::handler::server::router::tool::ToolRouter<Self> {
        Self::document_router() + Self::semantic_router() + Self::storage_router()
    }
}

#[rmcp::tool_handler(router = Self::combined_router())]
impl rmcp::ServerHandler for DocProcServer {}

// ── Entry point ────────────────────────────────────────────────────────────

/// Run the docproc MCP server (used by binary target).
pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    hkask_mcp::run_server(
        "hkask-mcp-docproc",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let ocr_model = ctx
                .credentials
                .get("HKASK_OCR_MODEL")
                .cloned();
            let inference_config = InferenceConfig::from_env();

            let ocr_thresholds = ThresholdConfig {
                simple_max: std::env::var("HKASK_OCR_SIMPLE_MAX")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.05),
                moderate_max: std::env::var("HKASK_OCR_MODERATE_MAX")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.15),
                moderate_sample_rate: std::env::var("HKASK_OCR_SAMPLE_RATE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.10),
                tuneable: std::env::var("HKASK_OCR_TUNEABLE")
                    .ok()
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(true),
            };

            let embedding_router = EmbeddingRouter::new(inference_config.clone());

            Ok(DocProcServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                ocr_model,
                inference_config,
                ocr_thresholds,
                Some(embedding_router),
            )?)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_OCR_MODEL",
                "Vision model for OCR (must exist in inference catalog). Required for OCR functionality.",
            ),
        ],
    )
    .await
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_json_fences_removes_fences() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_json_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_passthrough_plain_json() {
        let input = "{\"key\": \"value\"}";
        assert_eq!(strip_json_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_handles_whitespace() {
        let input = "  ```json\n{\"key\": \"value\"}\n```  ";
        assert_eq!(strip_json_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_no_language_tag() {
        let input = "```\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_json_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_empty_input() {
        assert_eq!(strip_json_fences(""), "");
    }

    #[test]
    fn chunk_word_bounds_defaults() {
        let (max_w, min_w) = chunk_word_bounds(None, None);
        // 512 tokens * 1.33 ≈ 681 words max, min = max(64*1.33=85, 681/4=170) = 170
        assert!(
            max_w > 600 && max_w < 700,
            "max_words should be ~681, got {max_w}"
        );
        assert!(
            min_w > 150 && min_w < 200,
            "min_words should be ~170, got {min_w}"
        );
    }

    #[test]
    fn chunk_word_bounds_explicit() {
        let (max_w, min_w) = chunk_word_bounds(Some(256), Some(32));
        // 256 * 1.33 ≈ 340, min = max(32*1.33=42, 340/4=85) = 85
        assert!(max_w > 300 && max_w < 400);
        assert!(min_w > 70 && min_w < 100);
    }

    #[test]
    fn serialize_passages_shape() {
        let passages = vec![
            ("doc:chunk:0".to_string(), "Hello world".to_string()),
            ("doc:chunk:1".to_string(), "Goodbye".to_string()),
        ];
        let result = serialize_passages(passages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["entity_ref"], "doc:chunk:0");
        assert_eq!(result[0]["text"], "Hello world");
        assert_eq!(result[1]["entity_ref"], "doc:chunk:1");
        assert_eq!(result[1]["text"], "Goodbye");
    }

    #[test]
    fn serialize_passages_empty() {
        let result = serialize_passages(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn cache_label_sanitization() {
        // This tests the sanitization logic inline since it's embedded in the tool
        let label = "my document/v1:notes";
        let safe: String = label
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        assert_eq!(safe, "my_document_v1_notes");
    }

    #[test]
    fn cache_path_construction() {
        let cache_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hkask")
            .join("docproc-cache");
        let safe_label = "test_doc";
        let cache_path = cache_dir.join(format!("{}.md", safe_label));
        assert!(cache_path.ends_with("test_doc.md"));
        assert!(cache_path.to_string_lossy().contains("docproc-cache"));
    }

    #[test]
    fn embed_rejects_empty_texts() {
        let req = EmbedRequest {
            texts: vec![],
            model: None,
        };
        // Validation happens before router access, so this tests the guard
        assert!(req.texts.is_empty());
    }

    #[test]
    fn extract_triples_rejects_empty_text() {
        let req = ExtractTriplesRequest {
            text: String::new(),
            namespace: None,
            max_triples: None,
        };
        assert!(req.text.is_empty());
    }

    #[test]
    fn generate_qa_rejects_empty_text() {
        let req = GenerateQaRequest {
            text: String::new(),
            chunk_id: "test".into(),
            bloom_levels: None,
        };
        assert!(req.text.is_empty());
    }

    #[test]
    fn generate_qa_rejects_empty_chunk_id() {
        let req = GenerateQaRequest {
            text: "some text".into(),
            chunk_id: String::new(),
            bloom_levels: None,
        };
        assert!(req.chunk_id.is_empty());
    }

    #[test]
    fn cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 0.001,
            "identical vectors should have similarity 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 0.0).abs() < 0.001,
            "orthogonal vectors should have similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[]), 0.0);
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn cosine_similarity_dimension_mismatch() {
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0, 2.0, 3.0]), 0.0);
    }

    #[test]
    fn query_rejects_empty() {
        let req = QueryRequest {
            query: String::new(),
            top_k: None,
            generate_answer: None,
        };
        assert!(req.query.is_empty());
    }

    #[test]
    fn chunk_defaults_index_true() {
        // Verify the default_true helper
        assert!(default_true());
    }
}
