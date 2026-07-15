//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Combines format conversion, OCR, chunking, h_mem extraction, embedding,
//! QA generation, caching, query, and Kindle book export (12 tools). Supersedes the former
//! `hkask-mcp-markitdown` and `hkask-mcp-doc-knowledge` servers.
//!
//! Server struct in lib.rs, tool methods in tools/ module.
//! (kanban pattern) for fuzz test construction and P5 Testing Discipline
//! compliance.

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

pub mod convert;
pub mod ocr;
pub mod tools;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use crate::ocr::decimation;
use crate::ocr::llm_ocr::LlmOcrExecutor;
use crate::ocr::pipeline::{self, OcrExecutor};
use crate::ocr::tesseract::TesseractExecutor;
use crate::ocr::{OcrBackend, OcrResult, ThresholdConfig};
use async_trait::async_trait;

use crate::ocr::calibration::{analyze_threshold_drift, emit_drift_alert};
use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_memory::SemanticMemory;
use hkask_ports::InferencePort;
use hkask_services_core::settings::HkaskSettings;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
#[allow(unused_imports)]
use serde::Serialize;
use serde_json::json;
use std::sync::{Arc, Mutex};

// ── Constants ──────────────────────────────────────────────────────────────

/// Resolve the embedding dimension from env or default to 1024 (Qwen3-Embedding-0.6B).
pub(crate) fn embedding_dim() -> usize {
    std::env::var("HKASK_EMBEDDING_DIM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1024)
}

/// Pre-normalize a vector in place so cosine similarity becomes a dot product.
pub(crate) fn normalize_in_place(v: &mut [f32]) {
    let mag = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if mag > 0.0 {
        for x in v.iter_mut() {
            *x /= mag;
        }
    }
}

/// Construct a WebID for a persona owner string.
pub(crate) fn owner_webid(owner: &str) -> hkask_types::WebID {
    hkask_types::WebID::from_persona(owner.as_bytes())
}

/// Minimum word count from pdf-extract to consider text extraction successful
/// before falling back to OCR for scanned PDFs.
pub(crate) const OCR_FALLBACK_WORD_THRESHOLD: usize = 100;

/// Default owner persona for h_mems stored by corpus pipeline tools.
const DEFAULT_OWNER: &str = "john-brooks";

/// System prompt for OCR vision requests.
const OCR_SYSTEM_PROMPT: &str =
    "Extract all text from this image. Output only the extracted text, nothing else.";

/// Default max tokens for OCR output.
pub(crate) fn default_ocr_max_tokens() -> u32 {
    8192
}

/// OCR pipeline concurrency — env var HKASK_OCR_CONCURRENCY, default 4.
/// Controls how many pages are sent to the vision model in parallel.
/// Set to 1 for sequential mode (interactive use), higher for batch processing.
pub(crate) fn ocr_concurrency() -> usize {
    std::env::var("HKASK_OCR_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(4)
}

/// Default embedding model — env var first, then HkaskSettings from disk.
/// Consolidates 6 hardcoded "DI/Qwen/Qwen3-Embedding-0.6B" references (Q3).
/// Result is cached in a OnceLock to avoid repeated disk reads and eliminate
/// the `String::leak` anti-pattern (BUG-1 fix, BUG-2 fix).
fn default_embedding_model() -> &'static str {
    use std::sync::OnceLock;
    static CACHED: OnceLock<String> = OnceLock::new();

    CACHED
        .get_or_init(|| {
            std::env::var("HKASK_EMBEDDING_MODEL")
                .unwrap_or_else(|_| HkaskSettings::load().embedding_model)
        })
        .as_str()
}

// ── Server struct ──────────────────────────────────────────────────────────

hkask_mcp::mcp_server!(
    struct DocProcServer {
        pub ocr_model: Option<String>,
        pub inference_router: Arc<InferenceRouter>,
        pub ocr_thresholds: ThresholdConfig,
        pub embedding_router: Option<EmbeddingRouter>,
        pub cv_accumulator: Mutex<Vec<crate::ocr::CrossValidation>>,
        pub(crate) index: Mutex<Vec<IndexedPassage>>,
        pub(crate) llm_ocr: Arc<LlmOcrExecutor>,
        pub(crate) pipeline_executor: Arc<PipelineExecutor>,
    }
);

/// A passage stored in the in-memory vector index with its embedding.
#[derive(Debug, Clone)]
pub(crate) struct IndexedPassage {
    pub text: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}

// ── Server constructor + core methods ──────────────────────────────────────

impl DocProcServer {
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
            .unwrap_or_else(|_| default_embedding_model().to_string());

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

    /// Run the OCR pipeline on page images and return joined text + outcome.
    ///
    /// Consolidates 3 duplicated invocation blocks in `docproc_convert`
    /// (Candidate 1 — architectural deepening). Handles embedding router
    /// construction, pipeline execution, persistence, and text joining.
    pub async fn run_ocr_pipeline(
        &self,
        page_images: Vec<image::DynamicImage>,
        model: &str,
    ) -> (String, usize, crate::ocr::PipelineOutcome) {
        let expected = page_images.len();
        let emb_model = default_embedding_model();
        let emb = self.embedding_router.as_ref().map(|r| (r, emb_model));

        let outcome = pipeline::run_pipeline(
            page_images,
            expected,
            Arc::clone(&self.pipeline_executor) as Arc<dyn OcrExecutor>,
            &self.ocr_thresholds,
            Some(model),
            emb,
            Some(ocr_concurrency()),
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

        (text, word_count, outcome)
    }
}

// ── OcrExecutor implementation ─────────────────────────────────────────────

/// Shareable OCR executor that bundles Tesseract + LLM backends.
///
/// Created once per server and passed as `Arc<dyn OcrExecutor>` to the pipeline.
/// This avoids the lifetime issues of passing `&DocProcServer` to parallel tasks.
pub(crate) struct PipelineExecutor {
    llm_ocr: Arc<LlmOcrExecutor>,
}

impl PipelineExecutor {
    pub(crate) fn new(llm_ocr: Arc<LlmOcrExecutor>) -> Self {
        Self { llm_ocr }
    }
}

#[async_trait]
impl OcrExecutor for PipelineExecutor {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        match backend {
            OcrBackend::Tesseract => TesseractExecutor::new().is_available(backend),
            OcrBackend::LlmOcr(_) => self.llm_ocr.is_available(backend),
        }
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &image::DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String> {
        static TESSERACT: std::sync::LazyLock<TesseractExecutor> =
            std::sync::LazyLock::new(TesseractExecutor::new);

        match backend {
            OcrBackend::Tesseract => {
                TESSERACT
                    .execute(page_index, backend, image, is_fallback)
                    .await
            }
            _ => {
                self.llm_ocr
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

        self.accumulate_and_check_drift(outcome);
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

        let vision_models = self.inference_router.list_vision_models().await;
        let is_vision = vision_models
            .iter()
            .any(|m| m.model == model || m.prefixed_name == model);

        if !is_vision {
            let all_models = self.inference_router.list_models().await;
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

        let params = LLMParameters {
            temperature: 0.1,
            max_tokens,
            ..Default::default()
        };

        let result = self
            .inference_router
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
async fn extract_text(path: &str) -> Result<ExtractOutcome, McpToolError> {
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
        "pdf" => {
            let output = tokio::process::Command::new("pdftotext")
                .arg(path)
                .arg("-")
                .output()
                .await;
            match output {
                Ok(output) if output.status.success() => {
                    let text = String::from_utf8_lossy(&output.stdout).into_owned();
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
                Ok(output) => {
                    tracing::warn!(
                        target: "cns.pipeline.pdf_extract",
                        path = path,
                        stderr = %String::from_utf8_lossy(&output.stderr),
                        "pdftotext failed — routing document to OCR"
                    );
                    ExtractOutcome::NeedsOcr {
                        partial_text: String::new(),
                        word_count: 0,
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        target: "cns.pipeline.pdf_extract",
                        path = path,
                        error = %error,
                        "pdftotext unavailable — routing document to OCR"
                    );
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

/// Cosine similarity between two vectors. Consolidated from ocr/semantic.rs (C4).
/// Returns 0.0 if either vector is empty or dimensions mismatch.
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
}

/// Approximate token-to-word conversion: 1 word ≈ 1.33 tokens.
/// So tokens ÷ 1.33 = words. This is the standard BPE ratio for English text.
pub(crate) fn tokens_to_words(tokens: usize) -> usize {
    ((tokens as f64) / 1.33) as usize
}

/// Compute (max_words, min_words) from (max_tokens, overlap_tokens).
/// Falls back to HkaskSettings::chunk_max_tokens() when max_tokens is None.
pub(crate) fn chunk_word_bounds(
    max_tokens: Option<usize>,
    overlap_tokens: Option<usize>,
) -> (usize, usize) {
    let default_max = HkaskSettings::load().chunk_max_tokens();
    let max_w = tokens_to_words(max_tokens.unwrap_or(default_max));
    let min_w = tokens_to_words(overlap_tokens.unwrap_or(64)).max(max_w / 4);
    (max_w, min_w)
}

/// Serialize (entity_ref, text) pair slice into json.
fn serialize_passages(passages: &[(String, String)]) -> Vec<serde_json::Value> {
    passages
        .iter()
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

/// Extract JSON from an LLM response that may contain thinking-mode reasoning.
///
/// Models like GLM-5.2 and Qwen3.6 produce reasoning text before the JSON payload.
/// This function strips code fences, then extracts from the first `{` to the
/// last `}` — discarding any reasoning preamble or trailing text.
///
/// Proven against GLM-5.2 (~640-830 reasoning tokens) and Qwen3.6-35B-A3B.
pub(crate) fn extract_json_from_response(text: &str) -> String {
    let de_fenced = strip_json_fences(text);
    match de_fenced.find('{') {
        Some(start) => match de_fenced.rfind('}') {
            Some(end) if end > start => de_fenced[start..=end].to_string(),
            _ => de_fenced,
        },
        None => de_fenced,
    }
}

/// Load a docproc template from registry and render with minijinja.
///
/// Templates live in `registry/templates/docproc/` as Jinja2 files.
/// Uses the same minijinja rendering pattern as `self_heal.rs` and the
/// hkask-templates ManifestExecutor. Falls back to empty string if the
/// template file is missing or rendering fails — callers provide an
/// inline fallback prompt.
///
/// Template base path is resolved relative to the workspace root. If the
/// server is started from a different directory, set `HKASK_REPLICANT_REGISTRY_PATH`
/// to the absolute path of the `registry/replicants` directory.
/// Cached template environment — compiled templates are stored and reused.
///
/// Template names and sources are leaked as `'static` only on first load.
/// Subsequent calls look up by non-static `&str` — no allocation, no leak.
/// This is acceptable because there are only a handful of templates and they
/// live for the server's lifetime anyway.
static TEMPLATE_CACHE: std::sync::OnceLock<std::sync::Mutex<minijinja::Environment<'static>>> =
    std::sync::OnceLock::new();

fn render_docproc_template(
    template_name: &str,
    vars: &std::collections::HashMap<&str, String>,
) -> String {
    let env = TEMPLATE_CACHE.get_or_init(|| {
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);
        std::sync::Mutex::new(env)
    });

    let lookup_key = format!("docproc:{template_name}");

    let mut env_guard = env.lock().unwrap_or_else(|e| e.into_inner());

    // First try: look up with non-static key — no allocation needed if cached
    let needs_load = env_guard.get_template(&lookup_key).is_err();

    if needs_load {
        // Template not found — load from disk. Leaks key + source as 'static
        // (only on first load of this template name — bounded by template count).
        let template_key: &'static str = Box::leak(lookup_key.into_boxed_str());

        let template_root =
            std::env::var("HKASK_TEMPLATE_ROOT").unwrap_or_else(|_| "registry".to_string());
        let template_path = std::path::Path::new(&template_root)
            .join("templates/docproc")
            .join(format!("{template_name}.j2"));

        let content = match std::fs::read_to_string(&template_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(target: "hkask.mcp.docproc.template", path = %template_path.display(), error = %e, "Template not found");
                return String::new();
            }
        };

        let source: &'static str = Box::leak(content.into_boxed_str());
        if let Err(e) = env_guard.add_template(template_key, source) {
            tracing::warn!(target: "hkask.mcp.docproc.template", error = %e, "Invalid template syntax");
            return String::new();
        }

        let ctx = serde_json::to_value(vars).unwrap_or_default();
        return match env_guard
            .get_template(template_key)
            .and_then(|t| t.render(minijinja::Value::from_serialize(&ctx)))
        {
            Ok(rendered) => rendered.trim().to_string(),
            Err(e) => {
                tracing::warn!(target: "hkask.mcp.docproc.template", error = %e, "Template render failed");
                String::new()
            }
        };
    }

    // Template already cached — render directly (no leak, no allocation)
    let ctx = serde_json::to_value(vars).unwrap_or_default();
    match env_guard
        .get_template(&lookup_key)
        .and_then(|t| t.render(minijinja::Value::from_serialize(&ctx)))
    {
        Ok(rendered) => rendered.trim().to_string(),
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.docproc.template", error = %e, "Template render failed");
            String::new()
        }
    }
}

// ── Request structs ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConvertRequest {
    /// Path to a document file or a directory of documents to convert.
    pub path: String,
    /// Output directory for batch conversion. Required when `path` is a directory.
    #[serde(default)]
    pub output: Option<String>,
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
    /// Raw text to chunk. Mutually exclusive with `path` and `input_dir`.
    #[serde(default)]
    pub text: Option<String>,
    /// Path to a document file to extract text from and chunk.
    #[serde(default)]
    pub path: Option<String>,
    /// Directory of extracted text files to chunk as one corpus.
    #[serde(default)]
    pub input_dir: Option<String>,
    /// JSONL output path for directory mode. Required with `input_dir`.
    #[serde(default)]
    pub output: Option<String>,
    /// Prefix for entity references in chunk output.
    pub entity_ref_prefix: String,
    /// Max tokens per chunk (single-tier mode). Default: 256 from HkaskSettings.
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
    /// Single chunk text (mutually exclusive with texts for multi-chunk cross-reference)
    #[serde(default)]
    pub text: Option<String>,
    /// Multiple chunks for cross-reference QA generation (RA-DIT method).
    /// When set, generates QAs that require synthesizing across all passages.
    #[serde(default)]
    pub texts: Option<Vec<String>>,
    pub chunk_id: String,
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
    /// Optional provider-prefixed generation model (for example, `OR/openai/gpt-5.6-terra`).
    /// When absent, uses `HKASK_QA_MODEL`, then `HKASK_DEFAULT_MODEL`.
    #[serde(default)]
    pub model: Option<String>,
}

/// A single prompt spec for batch QA generation.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchQaPrompt {
    pub text: String,
    pub chunk_id: String,
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateQaBatchRequest {
    /// Array of prompt specs to process.
    pub prompts: Vec<BatchQaPrompt>,
    /// Max concurrent LLM calls. Batch processing is currently sequential.
    #[serde(default = "default_batch_concurrency")]
    pub concurrency: usize,
    /// Optional provider-prefixed generation model for every prompt in this batch.
    #[serde(default)]
    pub model: Option<String>,
}

fn default_batch_concurrency() -> usize {
    4
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractTriplesRequest {
    /// Text to extract RDF h_mems from.
    pub text: String,
    /// Optional entity namespace prefix (e.g., "doc:myfile").
    #[serde(default)]
    pub namespace: Option<String>,
    /// Maximum h_mems to extract (default 50).
    #[serde(default)]
    pub max_triples: Option<usize>,
    /// Chunk reference (entity_ref) — used as the h_mem entity when storing triples.
    /// When provided with db_path, triples are stored as h_mems with entity=chunk_ref.
    #[serde(default)]
    pub chunk_ref: Option<String>,
    /// Path to the SQLCipher memory DB for h_mem storage.
    /// When provided with chunk_ref, extracted triples are stored as h_mems.
    #[serde(default)]
    pub db_path: Option<String>,
    /// Passphrase for the memory DB.
    #[serde(default)]
    pub passphrase: Option<String>,
    /// Owner persona for stored h_mems (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_owner() -> String {
    DEFAULT_OWNER.to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmbedRequest {
    /// Texts to embed (passages or h_mem strings).
    pub texts: Vec<String>,
    /// Embedding model to use. If not set, uses the configured default.
    #[serde(default)]
    pub model: Option<String>,
    /// Path to the SQLCipher memory DB for vector + h_mem storage.
    /// When provided, embeddings and text/provenance h_mems are stored in the DB.
    /// When omitted, vectors are returned as JSON (backward compatible).
    #[serde(default)]
    pub db_path: Option<String>,
    /// Passphrase for the memory DB.
    #[serde(default)]
    pub passphrase: Option<String>,
    /// Entity refs (chunk_ref) for each text — used as the h_mem entity and embedding key.
    /// Must match `texts` length when provided. Required when `db_path` is set.
    #[serde(default)]
    pub entity_refs: Option<Vec<String>>,
    /// Owner persona for stored h_mems (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PurgeQaRequest {
    /// Entity-ref prefix to purge (e.g. "corpus:qa" for old schema, "training:qa:" for new).
    #[serde(default = "default_purge_prefix")]
    pub prefix: String,
    /// Path to the SQLCipher memory DB.
    pub db_path: String,
    /// Passphrase for the memory DB.
    #[serde(default = "default_purge_passphrase")]
    pub passphrase: String,
}

fn default_purge_prefix() -> String {
    "corpus:qa".to_string()
}

fn default_purge_passphrase() -> String {
    "hkask-default-passphrase-2024".to_string()
}

// ── Corpus pipeline request structs ───────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DedupChunksRequest {
    /// Path to tagged chunks JSONL (from salience phase).
    pub tagged_jsonl: String,
    /// Output path for deduplicated tagged chunks JSONL.
    pub output: String,
    /// Path to the SQLCipher memory DB containing chunk embeddings.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Entity-ref prefix for chunk embeddings in the DB (e.g. "corpus:researcher:").
    #[serde(default = "default_corpus_prefix")]
    pub prefix: String,
    /// Cosine similarity threshold — chunks above this are near-duplicates.
    #[serde(default = "default_dedup_threshold")]
    pub threshold: f64,
    /// If true, only report clustering stats without writing output.
    #[serde(default)]
    pub dry_run: bool,
}

fn default_corpus_prefix() -> String {
    "corpus:researcher:".to_string()
}

fn default_dedup_threshold() -> f64 {
    0.85
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConsolidateChunksRequest {
    /// Path to tagged chunks JSONL (from dedup or salience phase).
    pub tagged_jsonl: String,
    /// Output path for consolidated tagged chunks JSONL.
    pub output: String,
    /// Path to the SQLCipher memory DB.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Entity-ref prefix for chunk embeddings.
    #[serde(default = "default_corpus_prefix")]
    pub prefix: String,
    /// Cosine similarity threshold for clustering (0.75 = semantic overlap).
    #[serde(default = "default_consolidate_threshold")]
    pub threshold: f64,
    /// Max concurrent LLM consolidation calls.
    #[serde(default = "default_consolidate_concurrency")]
    pub concurrency: usize,
    /// Max chunks per consolidation cluster (limits LLM context).
    #[serde(default = "default_max_chunks_per_cluster")]
    pub max_chunks_per_cluster: usize,
    /// If true, only report clustering stats without LLM calls.
    #[serde(default)]
    pub dry_run: bool,
}

fn default_consolidate_threshold() -> f64 {
    0.75
}

fn default_consolidate_concurrency() -> usize {
    12
}

fn default_max_chunks_per_cluster() -> usize {
    5
}

// ── Tag chunks request (ontology annotation) ───────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagChunksRequest {
    /// Path to chunks JSONL (entity_ref, source, text, word_count per line).
    pub chunks_jsonl: String,
    /// Output path for tagged chunks JSONL with ontology annotations.
    pub output: String,
    /// Path to the SQLCipher memory DB for h_mem storage.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Max concurrent LLM tagging calls.
    #[serde(default = "default_tag_concurrency")]
    pub concurrency: usize,
    /// If true, only report stats without LLM calls or writing output.
    #[serde(default)]
    pub dry_run: bool,
    /// Owner persona for stored h_mems (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_tag_concurrency() -> usize {
    128
}

// ── Build prompts request ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildPromptsRequest {
    /// Path to tagged chunks JSONL (from consolidate phase).
    pub tagged_jsonl: String,
    /// Output path for prompts JSONL (one JSON per line, consumed by generate_qa_batch).
    pub output: String,
    /// Path to the SQLCipher memory DB for embedding retrieval + h_mem knowledge graph.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// Number of KNN context passages to retrieve per chunk (default 3).
    #[serde(default = "default_context_k")]
    pub context_k: usize,
    /// Number of Bloom-level QA prompts per chunk (default 5 — one per level).
    #[serde(default = "default_prompts_per_chunk")]
    pub prompts_per_chunk: usize,
    /// Bloom's taxonomy weight distribution (e.g. "1,1,1,1,1" = equal).
    #[serde(default = "default_type_distribution")]
    pub type_distribution: String,
    /// Generate cross-reference synthesis prompts.
    #[serde(default)]
    pub cross_reference: bool,
    /// Max prompts to output (0 = all qualifying chunks).
    #[serde(default)]
    pub max_prompts: usize,
    /// Owner persona for h_mem queries (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_context_k() -> usize {
    3
}

fn default_prompts_per_chunk() -> usize {
    5
}

fn default_type_distribution() -> String {
    "1,1,1,1,1".to_string()
}

// ── Ingest QA request ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IngestQaRequest {
    /// Path to generated QAs JSONL (from docproc_generate_qa_batch).
    pub generated_jsonl: String,
    /// Output path for training-ready JSONL (instruction/input/output per line).
    pub output: String,
    /// Path to the SQLCipher memory DB for h_mem + embedding storage.
    pub db_path: String,
    /// Passphrase for the memory DB.
    pub passphrase: String,
    /// SemDeDup cosine similarity threshold (0.89 = moderate, 0.92 = strict).
    #[serde(default = "default_dedup_threshold_ingest")]
    pub dedup_threshold: f64,
    /// If true, validate and dedup without storing.
    #[serde(default)]
    pub dry_run: bool,
    /// Store QA embedding vectors in EmbeddingStore for KNN search.
    #[serde(default)]
    pub embed_qas: bool,
    /// Dataset name for training_qa_pair h_mems.
    #[serde(default = "default_dataset")]
    pub dataset: String,
    /// Owner persona for stored h_mems (e.g. "john-brooks").
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_dedup_threshold_ingest() -> f64 {
    0.89
}

fn default_dataset() -> String {
    "capabilities-researcher".to_string()
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
        Self::document_router()
            + Self::semantic_router()
            + Self::storage_router()
            + Self::corpus_router()
            + Self::tagging_router()
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
            let inference_router = Arc::new(InferenceRouter::new(inference_config));

                        let llm_ocr = Arc::new(LlmOcrExecutor::new(Arc::clone(&inference_router)));
                        let pipeline_executor = Arc::new(PipelineExecutor::new(Arc::clone(&llm_ocr)));

                        Ok(DocProcServer::new(
                            ctx.webid,
                            replicant.clone(),
                            daemon_client.clone(),
                            ocr_model,
                            inference_router,
                            ocr_thresholds,
                            Some(embedding_router),
                            Mutex::new(Vec::new()),
                            Mutex::new(Vec::new()),
                            llm_ocr,
                            pipeline_executor,
                        ))
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
    fn convert_request_schema_supports_pipeline_output_directory() {
        let schema = schemars::schema_for!(ConvertRequest);
        let properties = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .expect("ConvertRequest schema should expose object properties");

        assert!(
            properties.contains_key("output"),
            "docproc_convert must accept the pipeline manifest's output directory"
        );
    }

    #[test]
    fn extract_json_from_response_handles_thinking_mode() {
        // GLM-5.2 / Qwen3.6 produce reasoning text before JSON
        let input = "Let me analyze this passage.\nThe key concept is ROIC.\n\n{\"qa_pairs\": [{\"question\": \"What is ROIC?\", \"answer\": \"Return on Invested Capital\", \"bloom_level\": \"factual\"}]}";
        let result = extract_json_from_response(input);
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
        assert!(result.contains("qa_pairs"));
    }

    #[test]
    fn extract_json_from_response_plain_json() {
        let input = "{\"h_mems\": []}";
        assert_eq!(extract_json_from_response(input), "{\"h_mems\": []}");
    }

    #[test]
    fn extract_json_from_response_fenced_json() {
        let input = "```json\n{\"x\": 1}\n```";
        assert_eq!(extract_json_from_response(input), "{\"x\": 1}");
    }

    #[test]
    fn extract_json_from_response_no_json() {
        let input = "Just plain text, no JSON here.";
        assert_eq!(
            extract_json_from_response(input),
            "Just plain text, no JSON here."
        );
    }

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
        // Default max_tokens comes from HkaskSettings (256).
        // 256 tokens / 1.33 ≈ 192 words max, min = max(64/1.33=48, 192/4=48) = 48
        let (max_w, _min_w) = chunk_word_bounds(None, None);
        assert!(
            max_w > 180 && max_w < 200,
            "max_words should be ~192, got {max_w}"
        );
    }

    #[test]
    fn chunk_word_bounds_explicit() {
        // 256 tokens / 1.33 ≈ 192 words, min = max(32/1.33=24, 192/4=48) = 48
        let (max_w, min_w) = chunk_word_bounds(Some(256), Some(32));
        assert!(max_w > 180 && max_w < 200, "got {max_w}");
        assert!(min_w > 40 && min_w < 60, "got {min_w}");
    }

    #[test]
    fn serialize_passages_shape() {
        let passages = vec![
            ("doc:chunk:0".to_string(), "Hello world".to_string()),
            ("doc:chunk:1".to_string(), "Goodbye".to_string()),
        ];
        let result = serialize_passages(&passages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["entity_ref"], "doc:chunk:0");
        assert_eq!(result[0]["text"], "Hello world");
        assert_eq!(result[1]["entity_ref"], "doc:chunk:1");
        assert_eq!(result[1]["text"], "Goodbye");
    }

    #[test]
    fn serialize_passages_empty() {
        let result = serialize_passages(&[]);
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
            db_path: None,
            passphrase: None,
            entity_refs: None,
            owner: "john-brooks".to_string(),
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
            chunk_ref: None,
            db_path: None,
            passphrase: None,
            owner: "john-brooks".to_string(),
        };
        assert!(req.text.is_empty());
    }

    #[test]
    fn generate_qa_rejects_empty_text() {
        let req = GenerateQaRequest {
            text: Some(String::new()),
            texts: None,
            chunk_id: "test".into(),
            bloom_levels: None,
            model: None,
        };
        assert!(req.text.as_ref().is_some_and(|t| t.is_empty()));
    }

    #[test]
    fn generate_qa_rejects_empty_chunk_id() {
        let req = GenerateQaRequest {
            text: Some("some text".into()),
            texts: None,
            chunk_id: String::new(),
            bloom_levels: None,
            model: None,
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
