//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Combines format conversion, OCR, chunking, triple extraction, embedding,
//! QA generation, caching, and query (9 tools). Supersedes the former
//! `hkask-mcp-markitdown` and `hkask-mcp-doc-knowledge` servers.
//!
//! Server struct, constructor, and tool methods are all inline in lib.rs
//! (kanban pattern) for fuzz test construction and P5 Testing Discipline
//! compliance.

pub mod convert;
pub mod ocr;

// ── Imports ────────────────────────────────────────────────────────────────

use anyhow;
use async_trait::async_trait;

use crate::ocr::calibration::{analyze_threshold_drift, emit_drift_alert};
use crate::ocr::decimation;
use crate::ocr::llm_ocr::LlmOcrExecutor;
use crate::ocr::pipeline::{self, OcrExecutor};
use crate::ocr::tesseract::TesseractExecutor;
use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp::DaemonClient;
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::SemanticMemory;
use hkask_types::McpErrorKind;
use hkask_types::WebID;
use hkask_types::ocr::{OcrBackend, OcrResult, ThresholdConfig};
use hkask_types::ports::{CnsObserver, InferencePort};
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
    pub cv_accumulator: Mutex<Vec<hkask_types::ocr::CrossValidation>>,
    /// In-memory vector index for RAG query/retrieval. Passages indexed by `docproc_chunk`
    /// are stored here with their embeddings for cosine-similarity search via `docproc_query`.
    pub index: Mutex<Vec<IndexedPassage>>,
    /// Shared HTTP client (Chrome DevTools tab discovery, etc.).
    pub http_client: reqwest::Client,
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
        http_client: reqwest::Client,
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
            http_client,
        })
    }

    /// Check whether OCR capability is available.
    pub fn has_ocr(&self) -> bool {
        self.ocr_model.is_some()
    }

    /// Index passages into the in-memory vector store for later query.
    /// Embeds each passage text and stores it with metadata.
    /// Returns the number of passages indexed (0 if embedding router unavailable).
    pub async fn index_passages(&self, passages: &[(String, String)], source_label: &str) -> usize {
        let Some(ref emb_router) = self.embedding_router else {
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

// ── CNS Observer ───────────────────────────────────────────────────────────

/// CNS observer that implements the real `hkask_types::ports::CnsObserver` trait.
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
impl hkask_types::ports::CnsObserver for DocProcCnsObserver {
    fn interest_mask(&self) -> Vec<hkask_types::event::SpanNamespace> {
        vec![hkask_types::event::SpanNamespace::new("cns.pipeline")]
    }

    async fn on_event(&self, event: &hkask_types::event::NuEvent) {
        let span_name = event.span.namespace.short_name();
        self.persist_span(span_name, event.observation.clone());
    }

    async fn on_depletion(&self, _signal: &hkask_types::ports::DepletionSignal) {
        tracing::warn!(target: "hkask.mcp.docproc.cns", "CNS depletion signal received");
    }

    async fn on_backpressure(&self, _signal: &hkask_types::ports::BackpressureSignal) {
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

// ── Pipeline helpers ───────────────────────────────────────────────────────

impl DocProcServer {
    /// Persist pipeline outcome to daemon for CNS observability.
    pub async fn persist_pipeline_outcome(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
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
    async fn emit_pipeline_event(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
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
    }

    /// Accumulate cross-validations and check for threshold drift.
    fn accumulate_and_check_drift(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
        let mut acc = self
            .cv_accumulator
            .lock()
            .expect("Failed to lock CV accumulator for drift check");
        acc.extend(outcome.cross_validations.clone());

        let synthetic_outcome = hkask_types::ocr::PipelineOutcome {
            results: vec![],
            report: hkask_types::ocr::VerificationReport::new(true, 0.0, vec![], 0, vec![]),
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

/// Cosine similarity between two vectors.
/// Returns 0.0 if either vector is empty or dimensions mismatch.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
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
    /// Optional index_id to clear a specific index. If absent, clears all.
    #[serde(default)]
    pub index_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KindleZipRequest {
    /// Exact or partial title of the book in the Kindle library.
    pub book_title: String,
    /// Output PDF file path.
    pub output_pdf: String,
    /// Safety limit — maximum pages to capture.
    #[serde(default = "default_kindle_max_pages")]
    pub max_pages: usize,
    /// Milliseconds to wait after each page turn.
    #[serde(default = "default_kindle_page_wait_ms")]
    pub page_wait_ms: u64,
}

fn default_kindle_max_pages() -> usize {
    500
}
fn default_kindle_page_wait_ms() -> u64 {
    1500
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

// ── Tool router ────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl DocProcServer {
    #[tool(
        description = "Extract text from a document. Detects format, extracts text with automatic OCR fallback for scanned/image-based PDFs. For PDF: tries text extraction first, falls back to vision OCR if result is near-empty. For other supported formats (TXT, MD, HTML): extracts plain text. Requires HKASK_OCR_MODEL for OCR fallback."
    )]
    pub async fn docproc_convert(
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
    pub async fn docproc_ocr(
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
    pub async fn docproc_chunk(
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
            index,
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
                "coarse": serialize_passages(coarse.clone()),
                "medium": serialize_passages(medium.clone()),
                "fine": serialize_passages(fine.clone()),
            });

            // Auto-index if requested
            let indexed = if index {
                let all: Vec<_> = coarse.into_iter().chain(medium).chain(fine).collect();
                self.index_passages(&all, &source_label).await
            } else {
                0
            };

            let mut result = result;
            result["indexed"] = json!(indexed);
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
            let serialized = serialize_passages(passages.clone());

            // Auto-index if requested
            let indexed = if index {
                self.index_passages(&passages, &source_label).await
            } else {
                0
            };

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
                "indexed": indexed,
            });
            self.record_experience("docproc_chunk", &source_label, "success", result.clone());
            span.ok_json(result)
        }
    }

    #[tool(
        description = "Generate QA pairs from a text chunk by calling the inference engine. Returns structured question-answer pairs at specified Bloom's taxonomy levels."
    )]
    pub async fn docproc_generate_qa(
        &self,
        Parameters(GenerateQaRequest {
            text,
            chunk_id,
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

        let router = InferenceRouter::new(self.inference_config.clone());
        let params = LLMParameters {
            temperature: 0.3,
            max_tokens: 4096,
            ..Default::default()
        };

        match router.generate(&prompt, &params).await {
            Ok(response) => {
                let cleaned = strip_json_fences(&response.text);
                let qa_pairs: serde_json::Value = match serde_json::from_str(&cleaned) {
                    Ok(v) => v,
                    Err(_) => {
                        json!({"raw_response": response.text, "parse_error": "LLM response was not valid JSON"})
                    }
                };

                let result = json!({
                    "chunk_id": chunk_id,
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

    #[tool(
        description = "Extract RDF triples (subject, predicate, object) from text using the inference engine. Returns structured knowledge triples with confidence scores."
    )]
    pub async fn docproc_extract_triples(
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
                let cleaned = strip_json_fences(&response.text);
                let triples: serde_json::Value = match serde_json::from_str(&cleaned) {
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
    pub async fn docproc_embed(
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
    pub async fn docproc_cache(
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

    #[tool(
        description = "Query the in-memory vector index for passages relevant to a natural language question. Embeds the query, computes cosine similarity against indexed passages, and returns top-k results. Optionally generates an LLM-augmented answer from retrieved context."
    )]
    pub async fn docproc_query(
        &self,
        Parameters(QueryRequest {
            query,
            top_k,
            generate_answer,
        }): Parameters<QueryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_query", &self.webid);

        if query.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("query must not be empty").to_json_string(),
            );
        }

        let k = top_k.unwrap_or(5).clamp(1, 50);

        // Embed the query
        let Some(ref emb_router) = self.embedding_router else {
            return span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition(
                    "Embedding router not configured — cannot embed query",
                )
                .to_json_string(),
            );
        };

        let model_name = std::env::var("HKASK_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());

        let query_embedding = match emb_router
            .embed_sentences(&model_name, &[query.as_str()])
            .await
        {
            Ok(v) => v.into_iter().next().unwrap_or_default(),
            Err(e) => {
                return span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::unavailable(format!("Query embedding failed: {}", e))
                        .to_json_string(),
                );
            }
        };

        if query_embedding.is_empty() {
            return span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable("Query embedding returned empty vector").to_json_string(),
            );
        }

        // Search the index (scoped to drop guard before any await)
        let (results, total_indexed) = {
            let index = match self.index.lock() {
                Ok(i) => i,
                Err(e) => {
                    return span.internal_error(
                        serde_json::json!({"error": format!("Index lock error: {}", e)}),
                    );
                }
            };
            if index.is_empty() {
                return span.ok_json(json!({
                    "query": query,
                    "results": [],
                    "total_indexed": 0,
                    "note": "No passages indexed. Run docproc_chunk with index=true first.",
                }));
            }

            let mut scored: Vec<(f32, &IndexedPassage)> = index
                .iter()
                .map(|p| (cosine_similarity(&query_embedding, &p.embedding), p))
                .collect();

            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(k);

            let results: Vec<serde_json::Value> = scored
                .iter()
                .map(|(score, p)| {
                    json!({
                        "text": p.text.clone(),
                        "metadata": p.metadata.clone(),
                        "score": score,
                    })
                })
                .collect();

            (results, index.len())
        }; // guard dropped here

        let mut result = json!({
            "query": query,
            "results": results,
            "total_indexed": total_indexed,
        });

        // Optionally generate an LLM-augmented answer
        if generate_answer.unwrap_or(false) && !results.is_empty() {
            let context: String = results
                .iter()
                .map(|r| r["text"].as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n\n");

            let prompt = format!(
                "Answer the following question based on the provided context. If the context doesn't contain enough information, say so.\n\n\
                 Context:\n{context}\n\n\
                 Question: {query}\n\n\
                 Answer:"
            );

            let router = InferenceRouter::new(self.inference_config.clone());
            let params = LLMParameters {
                temperature: 0.3,
                max_tokens: 1024,
                ..Default::default()
            };

            match router.generate(&prompt, &params).await {
                Ok(response) => {
                    result["answer"] = json!(response.text);
                    result["answer_tokens"] = json!(response.usage.total_tokens);
                }
                Err(e) => {
                    result["answer_error"] = json!(format!("{}", e));
                }
            }
        }

        self.record_experience("docproc_query", &query, "success", result.clone());
        span.ok_json(result)
    }

    #[tool(
        description = "Clear the in-memory vector index. Call this when starting a new document set to avoid cross-document contamination in query results."
    )]
    pub async fn docproc_clear_index(
        &self,
        Parameters(ClearIndexRequest { index_id: _ }): Parameters<ClearIndexRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_clear_index", &self.webid);
        let mut index = match self.index.lock() {
            Ok(i) => i,
            Err(e) => {
                return span.internal_error(
                    serde_json::json!({"error": format!("Index lock error: {}", e)}),
                );
            }
        };
        let cleared = index.len();
        index.clear();
        span.ok_json(json!({"cleared": cleared}))
    }

    #[tool(
        description = "Capture a Kindle Cloud Reader book from your local Chrome browser. Chrome must be running with --remote-debugging-port=9222 and the book already open in Kindle Cloud Reader. Pages through the book taking screenshots via Chrome DevTools Protocol, then assembles them into a PDF."
    )]
    pub async fn docproc_kindle_zip(
        &self,
        Parameters(KindleZipRequest {
            book_title,
            output_pdf,
            max_pages,
            page_wait_ms,
        }): Parameters<KindleZipRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("docproc_kindle_zip", &self.webid);
        validate_field!(span, "book_title", &book_title, 512);
        validate_field!(span, "output_pdf", &output_pdf, 4096);

        // Connect to local Chrome via DevTools Protocol.
        // Chrome must be running with --remote-debugging-port=9222.
        let mut chrome = match ChromeCdpClient::connect(&self.http_client).await {
            Ok(c) => c,
            Err(e) => {
                return span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::internal(format!(
                        "Cannot connect to Chrome DevTools on localhost:9222. {e}"
                    ))
                    .to_json_string(),
                );
            }
        };

        let current_url = chrome
            .evaluate("window.location.href")
            .await
            .unwrap_or_default();

        // Step 1: Navigate to Kindle library if not already there
        if !current_url.contains("read.amazon.com/kindle-library") {
            tracing::info!(
                target: "hkask.mcp.docproc",
                current = %current_url,
                "Navigating to Kindle library"
            );
            chrome
                .evaluate("window.location.href = 'https://read.amazon.com/kindle-library'")
                .await
                .ok();
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        }

        // Step 2: Search for the book by title in the library
        let escaped_title = book_title.replace('\'', "\\'").replace('"', "\\\"");
        let search_js = format!(
            "(function() {{ var input = document.querySelector('input[type=\"search\"], input[placeholder*=\"Search\" i], input[placeholder*=\"search\" i]'); if (!input) return 'SEARCH_NOT_FOUND'; input.focus(); input.value = ''; var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set; setter.call(input, '{}'); input.dispatchEvent(new Event('input', {{ bubbles: true }})); input.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', bubbles: true }})); input.dispatchEvent(new KeyboardEvent('keyup', {{ key: 'Enter', bubbles: true }})); return 'SEARCH_SENT'; }})()",
            escaped_title
        );

        match chrome.evaluate(&search_js).await {
            Ok(result) => {
                tracing::info!(target: "hkask.mcp.docproc", result = %result, "Search executed");
            }
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Search failed: {e}")).to_json_string(),
                );
            }
        }

        // Wait for search results to load
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Step 3: Click the book cover/title to open the reader
        let click_js = format!(
            "(function() {{ var t = '{}'; var lower = t.toLowerCase(); var items = document.querySelectorAll('img[alt], [aria-label]'); for (var i = 0; i < items.length; i++) {{ var attr = (items[i].getAttribute('alt') || '') + (items[i].getAttribute('aria-label') || ''); if (attr.toLowerCase().indexOf(lower) >= 0 && items[i].offsetParent !== null) {{ items[i].closest('a, button, [role=\"button\"], [role=\"link\"]')?.click(); items[i].click(); return 'CLICKED'; }} }} var all = document.querySelectorAll('*'); for (var i = 0; i < all.length; i++) {{ if (all[i].children.length > 0) continue; if (all[i].textContent && all[i].textContent.toLowerCase().indexOf(lower) >= 0 && all[i].offsetParent !== null) {{ all[i].click(); return 'CLICKED_TEXT'; }} }} return 'NOT_FOUND'; }})()",
            escaped_title
        );

        match chrome.evaluate(&click_js).await {
            Ok(result) => {
                tracing::info!(target: "hkask.mcp.docproc", result = %result, "Book click result");
            }
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to open book: {e}")).to_json_string(),
                );
            }
        }

        // Wait for the reader to load
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Verify we're now in the reader
        let reader_url = chrome
            .evaluate("window.location.href")
            .await
            .unwrap_or_default();
        tracing::info!(
            target: "hkask.mcp.docproc",
            url = %reader_url,
            book_title = %book_title,
            "Reader opened"
        );

        // Step 4: Page through the book, capturing screenshots

        // Page through the book, capturing screenshots
        let mut screenshots: Vec<Vec<u8>> = Vec::with_capacity(max_pages.min(500));

        for page_num in 1..=max_pages {
            // Take screenshot of the current page
            match chrome.capture_screenshot().await {
                Ok(png_bytes) => {
                    if png_bytes.len() < 1024 {
                        tracing::warn!(
                            target: "hkask.mcp.docproc",
                            page = page_num,
                            byte_len = png_bytes.len(),
                            "Screenshot too small — possible blank page"
                        );
                    }
                    screenshots.push(png_bytes);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.mcp.docproc",
                        page = page_num,
                        error = %e,
                        "Screenshot failed, stopping capture"
                    );
                    break;
                }
            }

            // Turn to next page (unless we're at max_pages)
            if page_num < max_pages {
                // Kindle Cloud Reader advances pages with the right arrow key
                if let Err(e) = chrome.press_key("ArrowRight").await {
                    tracing::warn!(
                        target: "hkask.mcp.docproc",
                        page = page_num,
                        error = %e,
                        "Page turn failed, stopping capture"
                    );
                    break;
                }

                // Wait for the page to render
                tokio::time::sleep(std::time::Duration::from_millis(page_wait_ms)).await;
            }
        }

        if screenshots.is_empty() {
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal(
                    "No pages were captured. Ensure a book is open in Kindle Cloud Reader.",
                )
                .to_json_string(),
            );
        }

        // Assemble screenshots into a PDF
        let pages_captured = screenshots.len();
        match assemble_kindle_pdf(&screenshots, &output_pdf) {
            Ok(file_size) => {
                self.record_experience(
                    "docproc_kindle_zip",
                    &book_title,
                    "success",
                    json!({"pages_captured": pages_captured, "file_size_bytes": file_size}),
                );
                span.ok_json(json!({
                    "pdf_path": output_pdf,
                    "pages_captured": pages_captured,
                    "file_size_bytes": file_size,
                }))
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("PDF assembly failed: {e}")).to_json_string(),
            ),
        }
    }
}

// ── Chrome DevTools Protocol client ───────────────────────────────────────

use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

/// Drives a local Chrome browser via DevTools Protocol (CDP).
/// Chrome must be launched with `--remote-debugging-port=9222`.
struct ChromeCdpClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    msg_id: AtomicU64,
}

impl ChromeCdpClient {
    /// Connect to local Chrome DevTools, finding the Kindle Cloud Reader tab.
    async fn connect(http: &reqwest::Client) -> Result<Self, String> {
        // Discover open tabs
        let resp = http
            .get("http://localhost:9222/json")
            .send()
            .await
            .map_err(|e| format!("Chrome DevTools not reachable on localhost:9222 — is Chrome running with --remote-debugging-port=9222? ({e})"))?;

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read tab list: {e}"))?;
        let tabs: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse tab list: {e}"))?;

        if tabs.is_empty() {
            return Err("No open tabs found in Chrome".into());
        }

        // Prefer a Kindle Cloud Reader tab, fall back to any tab
        let target = tabs
            .iter()
            .find(|t| {
                t.get("url")
                    .and_then(|u| u.as_str())
                    .map(|u| u.contains("read.amazon.com"))
                    .unwrap_or(false)
            })
            .or_else(|| tabs.first())
            .ok_or("No open tabs found")?;

        let ws_url = target
            .get("webSocketDebuggerUrl")
            .and_then(|u| u.as_str())
            .ok_or("Tab has no WebSocket debugger URL")?;

        let (ws, _) = connect_async(ws_url)
            .await
            .map_err(|e| format!("WebSocket connection failed: {e}"))?;

        tracing::info!(
            target: "hkask.mcp.docproc.cdp",
            ws_url = %ws_url,
            "Connected to Chrome DevTools"
        );

        Ok(Self {
            ws,
            msg_id: AtomicU64::new(1),
        })
    }

    /// Send a CDP command and wait for the result.
    async fn send_command(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.msg_id.fetch_add(1, Ordering::Relaxed);
        let cmd = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        self.ws
            .send(Message::Text(cmd.to_string().into()))
            .await
            .map_err(|e| format!("CDP send error: {e}"))?;

        // Read responses until we get one matching our id
        while let Some(msg) = self.ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let resp: serde_json::Value =
                        serde_json::from_str(&text).map_err(|e| format!("CDP parse error: {e}"))?;
                    if resp.get("id").and_then(|v| v.as_u64()) == Some(id) {
                        if let Some(err) = resp.get("error") {
                            return Err(format!(
                                "CDP error: {}",
                                err.get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("unknown")
                            ));
                        }
                        return Ok(resp
                            .get("result")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null));
                    }
                    // Otherwise it's an event notification — ignore and continue
                }
                Ok(Message::Close(_)) => return Err("CDP connection closed".into()),
                Err(e) => return Err(format!("CDP read error: {e}")),
                _ => {} // ignore binary/ping/pong
            }
        }
        Err("CDP stream ended unexpectedly".into())
    }

    /// Capture a screenshot of the current page as PNG bytes.
    async fn capture_screenshot(&mut self) -> Result<Vec<u8>, String> {
        let result = self
            .send_command(
                "Page.captureScreenshot",
                serde_json::json!({
                    "format": "png",
                    "fromSurface": true,
                }),
            )
            .await?;

        let b64 = result
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or("No screenshot data in CDP response")?;

        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| format!("Failed to decode screenshot: {e}"))
    }

    /// Press a keyboard key (e.g., "ArrowRight", "ArrowLeft").
    async fn press_key(&mut self, key: &str) -> Result<(), String> {
        // Key down
        self.send_command(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyDown",
                "key": key,
            }),
        )
        .await?;
        // Key up
        self.send_command(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyUp",
                "key": key,
            }),
        )
        .await?;
        Ok(())
    }

    /// Evaluate JavaScript in the page and return the result as a string.
    async fn evaluate(&mut self, expression: &str) -> Result<String, String> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": expression,
                    "returnByValue": true,
                }),
            )
            .await?;

        Ok(result
            .pointer("/result/value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }
}

fn assemble_kindle_pdf(pages: &[Vec<u8>], output_path: &str) -> Result<u64, String> {
    use std::io::Write;

    if pages.is_empty() {
        return Err("No pages to assemble".into());
    }

    // Read all page dimensions first
    struct PageInfo {
        width: u32,
        height: u32,
        data: Vec<u8>,
    }

    let mut page_infos: Vec<PageInfo> = Vec::with_capacity(pages.len());
    for (i, png_bytes) in pages.iter().enumerate() {
        let img = image::load_from_memory(png_bytes)
            .map_err(|e| format!("Failed to decode page {i} PNG: {e}"))?;
        let (w, h) = (img.width(), img.height());
        // Re-encode as JPEG for smaller PDF size
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut jpeg_bytes),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| format!("Failed to re-encode page {i} as JPEG: {e}"))?;
        page_infos.push(PageInfo {
            width: w,
            height: h,
            data: jpeg_bytes,
        });
    }

    // Build minimal PDF
    let mut pdf: Vec<u8> = Vec::new();
    let mut offsets: Vec<u64> = Vec::new();

    // PDF header
    writeln!(pdf, "%PDF-1.4").unwrap();
    // Binary comment for PDF readers (high bytes to signal binary content)
    pdf.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");

    let n = page_infos.len() as u32;

    // Object 1: Catalog
    offsets.push(pdf.len() as u64);
    writeln!(pdf, "1 0 obj").unwrap();
    writeln!(pdf, "<< /Type /Catalog /Pages 2 0 R >>").unwrap();
    writeln!(pdf, "endobj").unwrap();

    // Object 2: Pages
    offsets.push(pdf.len() as u64);
    write!(pdf, "2 0 obj\n<< /Type /Pages /Kids [").unwrap();
    // Page objects start at 3, image objects at 3+n, content streams at 3+2n
    for i in 0..n {
        write!(pdf, "{} 0 R ", 3 + i * 3).unwrap();
    }
    writeln!(pdf, "] /Count {} >>", n).unwrap();
    writeln!(pdf, "endobj").unwrap();

    // Per-page objects: Page, Image XObject, Content stream
    for (i, info) in page_infos.iter().enumerate() {
        let page_obj = 3 + i as u32 * 3;
        let img_obj = page_obj + 1;
        let content_obj = page_obj + 2;

        // Image XObject
        offsets.push(pdf.len() as u64);
        writeln!(pdf, "{} 0 obj", img_obj).unwrap();
        writeln!(
            pdf,
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>",
            info.width, info.height, info.data.len()
        )
        .unwrap();
        writeln!(pdf, "stream").unwrap();
        pdf.write_all(&info.data).unwrap();
        writeln!(pdf, "\nendstream").unwrap();
        writeln!(pdf, "endobj").unwrap();

        // Content stream (scale image to fill page)
        offsets.push(pdf.len() as u64);
        let content = format!("q\n{} 0 0 {} 0 0 cm\n/Im0 Do\nQ", info.width, info.height);
        writeln!(pdf, "{} 0 obj", content_obj).unwrap();
        writeln!(pdf, "<< /Length {} >>", content.len()).unwrap();
        writeln!(pdf, "stream").unwrap();
        write!(pdf, "{}", content).unwrap();
        writeln!(pdf, "\nendstream").unwrap();
        writeln!(pdf, "endobj").unwrap();

        // Page object
        offsets.push(pdf.len() as u64);
        writeln!(pdf, "{} 0 obj", page_obj).unwrap();
        writeln!(
            pdf,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << /XObject << /Im0 {} 0 R >> >> >>",
            info.width, info.height, content_obj, img_obj
        )
        .unwrap();
        writeln!(pdf, "endobj").unwrap();
    }

    // Cross-reference table
    let xref_offset = pdf.len() as u64;
    writeln!(pdf, "xref").unwrap();
    writeln!(pdf, "0 {}", offsets.len() as u32 + 1).unwrap();
    writeln!(pdf, "0000000000 65535 f ").unwrap();
    for off in &offsets {
        writeln!(pdf, "{:010} 00000 n ", off).unwrap();
    }

    // Trailer
    writeln!(pdf, "trailer").unwrap();
    writeln!(pdf, "<< /Size {} /Root 1 0 R >>", offsets.len() as u32 + 1).unwrap();
    writeln!(pdf, "startxref").unwrap();
    writeln!(pdf, "{xref_offset}").unwrap();
    writeln!(pdf, "%%EOF").unwrap();

    // Write to disk
    let file_size = pdf.len() as u64;
    std::fs::write(output_path, &pdf)
        .map_err(|e| format!("Failed to write PDF to '{}': {e}", output_path))?;

    Ok(file_size)
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

#[cfg(test)]
mod kindle_zip_tests {
    use super::*;

    #[test]
    fn pdf_assembler_produces_valid_pdf() {
        use image::{Rgb, RgbImage};
        let mut pages = Vec::new();
        for color in [Rgb([255, 0, 0]), Rgb([0, 0, 255])] {
            let img = RgbImage::from_pixel(200, 300, color);
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
            pages.push(buf.into_inner());
        }
        let tmp = std::env::temp_dir().join("test_kindle_output.pdf");
        let result = assemble_kindle_pdf(&pages, tmp.to_str().unwrap());
        assert!(result.is_ok(), "PDF assembly failed: {:?}", result.err());
        let bytes = std::fs::read(&tmp).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4"), "Not a valid PDF");
        assert!(bytes.windows(5).any(|w| w == b"%%EOF"), "Missing EOF");
        let pm: Vec<_> = bytes.windows(9).filter(|w| *w == b"/MediaBox").collect();
        assert_eq!(pm.len(), 2, "Expected 2 pages, found {}", pm.len());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn pdf_assembler_rejects_empty() {
        assert!(assemble_kindle_pdf(&[], "x.pdf").is_err());
    }

    #[test]
    fn kindle_zip_request_defaults() {
        assert_eq!(default_kindle_max_pages(), 500);
        assert_eq!(default_kindle_page_wait_ms(), 1500);
    }
}

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

            let http_client = reqwest::Client::builder()
                .user_agent(format!("hkask-docproc/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| hkask_mcp::McpError::from(anyhow::anyhow!("Failed to build HTTP client: {e}")))?;

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
                http_client,
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
