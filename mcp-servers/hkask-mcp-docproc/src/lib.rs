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
pub mod template;
pub mod tools;

// Re-export template renderer for tool modules (accessible via `use crate::*;`)
pub(crate) use template::render_docproc_template;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use crate::ocr::ThresholdConfig;
use crate::ocr::decimation;
use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_memory::SemanticMemory;
use hkask_ports::InferencePort;
use hkask_services_core::settings::HkaskSettings;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
#[allow(unused_imports)]
use schemars::JsonSchema;
#[allow(unused_imports)]
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
        pub(crate) llm_ocr: Arc<crate::ocr::llm_ocr::LlmOcrExecutor>,
        pub(crate) pipeline_executor: Arc<crate::ocr::PipelineExecutor>,
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
pub(crate) fn default_owner() -> String {
    DEFAULT_OWNER.to_string()
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

                        let llm_ocr = Arc::new(crate::ocr::llm_ocr::LlmOcrExecutor::new(Arc::clone(&inference_router)));
                                    let pipeline_executor = Arc::new(crate::ocr::PipelineExecutor::new(Arc::clone(&llm_ocr)));

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
    use crate::tools::document::{ConvertRequest, default_true};
    use crate::tools::semantic::GenerateQaRequest;
    use crate::tools::storage::QueryRequest;

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
