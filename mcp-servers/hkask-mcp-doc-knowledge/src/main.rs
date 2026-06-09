//! hKask MCP Doc Knowledge — Document knowledge extraction and QA generation MCP server
//!
//! 8 tools:
//! - `doc_knowledge_ping` — Liveness check
//! - `doc_knowledge_chunk` — Chunk text at configurable token granularity
//! - `doc_knowledge_detect_format` — Detect document format from path/extension
//! - `doc_knowledge_extract_markdown` — Extract text and image refs from markdown
//! - `doc_knowledge_extract_html` — Extract text from HTML
//! - `doc_knowledge_parse` — Parse document into IR with multi-tier chunking
//! - `doc_knowledge_generate_qa` — Generate QA pairs from text chunk (returns structured prompt)
//! - `doc_knowledge_store_qa` — Store QA items with provenance

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::SemanticMemory;
use hkask_storage::Triple;
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

const SERVER_VERSION: &str = "0.1.0";

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

/// Validate a non-empty field; returns `(span, field_value)` or early-returns error JSON.
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

/// Detect document format from extension. Returns (format_name, supported, note).
fn detect_format(path: &str) -> (&'static str, bool, Option<&'static str>) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" | "markdown" => ("markdown", true, None),
        "html" | "htm" => ("html", true, None),
        "txt" => ("plain", true, None),
        "pdf" => (
            "pdf",
            false,
            Some("Use markitdown_convert for PDF text extraction + OCR"),
        ),
        _ => ("unknown", false, None),
    }
}

/// Strip YAML frontmatter (delimited by ---) from content.
fn strip_frontmatter(content: &str) -> String {
    if content.starts_with("---") {
        content
            .splitn(3, "---")
            .nth(2)
            .unwrap_or(content)
            .trim()
            .to_string()
    } else {
        content.to_string()
    }
}

/// Strip HTML tags and extract visible text content.
///
/// Removes script/style elements entirely, preserves word boundaries
/// for block-level elements (p, div, h1-h6, li, etc.), and collapses
/// consecutive whitespace.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_strip_tag = false;
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();

    /// Block-level tags that should insert a space when stripped.
    const BLOCK_TAGS: &[&str] = &[
        "p",
        "div",
        "br",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "li",
        "tr",
        "table",
        "blockquote",
        "pre",
        "section",
        "article",
        "header",
        "footer",
        "main",
        "aside",
        "nav",
        "figure",
    ];

    let mut i = 0;
    while i < len {
        let ch = chars[i];

        if ch == '<' {
            let remaining: String = chars[i..].iter().collect();
            let lower_remaining = remaining.to_lowercase();

            // Check for closing script/style tags
            if lower_remaining.starts_with("</script") || lower_remaining.starts_with("</style") {
                if in_strip_tag
                    && !result.is_empty()
                    && !result.chars().last().is_none_or(|c| c.is_whitespace())
                {
                    result.push(' ');
                }
                in_strip_tag = false;
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            // Check for opening script/style tags
            if lower_remaining.starts_with("<script") || lower_remaining.starts_with("<style") {
                if !result.is_empty() && !result.chars().last().is_none_or(|c| c.is_whitespace()) {
                    result.push(' ');
                }
                in_strip_tag = true;
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            // For regular tags, check if it's a block-level tag
            let tag_name = remaining
                .trim_start_matches('<')
                .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .next()
                .unwrap_or("")
                .to_lowercase();
            let is_block = BLOCK_TAGS.contains(&tag_name.as_str());

            if is_block
                && !result.is_empty()
                && !result.chars().last().is_none_or(|c| c.is_whitespace())
            {
                result.push(' ');
            }

            in_tag = true;
            i += 1;
            continue;
        }

        if ch == '>' {
            in_tag = false;
            i += 1;
            continue;
        }

        if !in_tag && !in_strip_tag {
            result.push(ch);
        }

        i += 1;
    }

    result.split_whitespace().collect::<Vec<&str>>().join(" ")
}

// ── Request structs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChunkRequest {
    pub text: String,
    pub entity_ref_prefix: String,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub overlap_tokens: Option<usize>,
    #[serde(default)]
    pub strip_gutenberg: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DetectFormatRequest {
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractMarkdownRequest {
    pub content: String,
    pub label: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractHtmlRequest {
    pub content: String,
    pub label: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseRequest {
    pub path: String,
    #[serde(default)]
    pub coarse_max_tokens: Option<usize>,
    #[serde(default)]
    pub medium_max_tokens: Option<usize>,
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

// ── Server ───────────────────────────────────────────────────────────────

pub struct DocKnowledgeServer {
    webid: WebID,
    semantic: Option<SemanticMemory>,
}

impl DocKnowledgeServer {
    pub fn new(webid: WebID, semantic: Option<SemanticMemory>) -> Result<Self, anyhow::Error> {
        Ok(Self { webid, semantic })
    }

    fn has_semantic(&self) -> bool {
        self.semantic.is_some()
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl DocKnowledgeServer {
    #[tool(description = "Liveness check for doc-knowledge server")]
    async fn doc_knowledge_ping(&self) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_ping", &self.webid);
        span.ok_json(json!({
            "status": "ok",
            "server": "hkask-mcp-doc-knowledge",
            "version": SERVER_VERSION,
            "semantic_available": self.has_semantic(),
        }))
    }

    #[tool(
        description = "Chunk text at configurable token granularity (delegates to SemanticMemory::chunk_text)"
    )]
    async fn doc_knowledge_chunk(
        &self,
        Parameters(ChunkRequest {
            text,
            entity_ref_prefix,
            max_tokens,
            overlap_tokens,
            strip_gutenberg,
        }): Parameters<ChunkRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_chunk", &self.webid);

        validate_non_empty!(span, McpErrorKind::InvalidArgument, "text", text);
        validate_non_empty!(
            span,
            McpErrorKind::InvalidArgument,
            "entity_ref_prefix",
            entity_ref_prefix
        );
        validate_field!(span, "entity_ref_prefix", &entity_ref_prefix, 256);

        let (max_words, min_words) = chunk_word_bounds(max_tokens, overlap_tokens);
        let boundary = ".!? ".to_string();

        let processed = if strip_gutenberg.unwrap_or(false) {
            SemanticMemory::strip_gutenberg_headers(&text)
        } else {
            text.clone()
        };

        let passages = SemanticMemory::chunk_text(
            &processed,
            &entity_ref_prefix,
            min_words,
            max_words,
            &boundary,
        );

        let total_passages = passages.len();
        let serialized = serialize_passages(passages);

        span.ok_json(json!({
            "total_passages": total_passages, "passages": serialized,
            "max_tokens": max_tokens.unwrap_or(512), "overlap_tokens": overlap_tokens.unwrap_or(64),
            "max_words": max_words, "min_words": min_words, "sentence_boundary": boundary,
            "stripped_gutenberg": strip_gutenberg.unwrap_or(false),
        }))
    }

    #[tool(description = "Detect document format from path/extension")]
    async fn doc_knowledge_detect_format(
        &self,
        Parameters(DetectFormatRequest { path }): Parameters<DetectFormatRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_detect_format", &self.webid);

        validate_non_empty!(span, McpErrorKind::InvalidArgument, "path", path);

        let (format, supported, note) = detect_format(&path);
        let mut result = json!({"format": format, "path": path, "supported": supported});
        if let Some(note) = note {
            result["note"] = json!(note);
        }

        span.ok_json(result)
    }

    #[tool(description = "Extract text and image refs from markdown")]
    async fn doc_knowledge_extract_markdown(
        &self,
        Parameters(ExtractMarkdownRequest { content, label }): Parameters<ExtractMarkdownRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_extract_markdown", &self.webid);

        validate_non_empty!(span, McpErrorKind::InvalidArgument, "content", content);

        let text = strip_frontmatter(&content);

        // Extract image refs: ![alt](url)
        let mut images = Vec::new();
        let mut i = 0;
        let bytes = text.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'!'
                && i + 1 < bytes.len()
                && bytes[i + 1] == b'['
                && let Some(bracket_end) = text[i + 2..].find(']')
            {
                let paren_start = i + 2 + bracket_end + 1;
                if paren_start < bytes.len()
                    && bytes[paren_start] == b'('
                    && let Some(paren_end) = text[paren_start + 1..].find(')')
                {
                    let url = &text[paren_start + 1..paren_start + 1 + paren_end];
                    if !url.is_empty() {
                        images.push(url.to_string());
                    }
                    i = paren_start + 1 + paren_end + 1;
                    continue;
                }
            }
            i += 1;
        }

        span.ok_json(json!({
            "text": text,
            "images": images,
            "label": label,
        }))
    }

    #[tool(
        description = "Extract text from HTML. Removes script/style tags and preserves word boundaries for block-level elements."
    )]
    async fn doc_knowledge_extract_html(
        &self,
        Parameters(ExtractHtmlRequest { content, label }): Parameters<ExtractHtmlRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_extract_html", &self.webid);

        validate_non_empty!(span, McpErrorKind::InvalidArgument, "content", content);

        let text = strip_html(&content);

        span.ok_json(json!({
            "text": text,
            "label": label,
        }))
    }

    #[tool(description = "Parse document into IR with multi-tier chunking (coarse/medium/fine)")]
    async fn doc_knowledge_parse(
        &self,
        Parameters(ParseRequest {
            path,
            coarse_max_tokens,
            medium_max_tokens,
            fine_max_tokens,
        }): Parameters<ParseRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_parse", &self.webid);

        validate_non_empty!(span, McpErrorKind::InvalidArgument, "path", path);

        let (format, _, _) = detect_format(&path);
        if format == "pdf" {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "PDF is not supported by doc_knowledge_parse. Use markitdown_convert for PDF text extraction (with OCR fallback for scanned PDFs), then pass the extracted text to doc_knowledge_chunk. Path: '{}'",
                    path
                )).to_json_string(),
            );
        }
        if format == "unknown" {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Unsupported document format '{}' for path '{}'",
                    format, path
                ))
                .to_json_string(),
            );
        }

        // Read file
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return span.internal_error(json!({
                    "error": format!("Failed to read file '{}': {}", path, e),
                }));
            }
        };

        let text = match format {
            "markdown" => strip_frontmatter(&content),
            "html" => strip_html(&content),
            _ => content,
        };

        let entity_base = path.replace(['/', '\\', '.', ' '], "_");
        let boundary = ".!? ";

        let chunk_tier = |tier: &str, max_tok: Option<usize>, default: usize| -> Vec<_> {
            let w = tokens_to_words(max_tok.unwrap_or(default));
            SemanticMemory::chunk_text(&text, &format!("{entity_base}:{tier}"), w / 4, w, boundary)
        };

        let coarse = chunk_tier("coarse", coarse_max_tokens, 2048);
        let medium = chunk_tier("medium", medium_max_tokens, 512);
        let fine = chunk_tier("fine", fine_max_tokens, 128);

        span.ok_json(json!({
            "format": format, "path": path,
            "coarse_max_tokens": coarse_max_tokens.unwrap_or(2048),
            "medium_max_tokens": medium_max_tokens.unwrap_or(512),
            "fine_max_tokens": fine_max_tokens.unwrap_or(128),
            "coarse": serialize_passages(coarse), "medium": serialize_passages(medium),
            "fine": serialize_passages(fine),
        }))
    }

    #[tool(
        description = "Generate QA prompt from text chunk (returns structured prompt for LLM; actual LLM call routed through inference engine)"
    )]
    async fn doc_knowledge_generate_qa(
        &self,
        Parameters(GenerateQaRequest {
            text,
            chunk_id,
            strategy,
            bloom_levels,
        }): Parameters<GenerateQaRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_generate_qa", &self.webid);

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

        span.ok_json(json!({
            "prompt": prompt, "chunk_id": chunk_id, "strategy": strat, "bloom_levels": levels,
            "note": "Route this prompt through the inference engine for LLM completion",
        }))
    }

    #[tool(description = "Store QA items with provenance")]
    async fn doc_knowledge_store_qa(
        &self,
        Parameters(StoreQaRequest {
            qa_items,
            source_document,
        }): Parameters<StoreQaRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_store_qa", &self.webid);

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
            span.ok_json(json!({ "stored": stored, "source_document": source_document }))
        } else {
            span.internal_error(
                json!({ "stored": stored, "errors": errors, "source_document": source_document }),
            )
        }
    }
}

// ── Server entry point ───────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-doc-knowledge",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let semantic = match ctx.credentials.get("HKASK_MEMORY_DB") {
                Some(path) => {
                    let passphrase =
                        ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                            anyhow::anyhow!("HKASK_MEMORY_DB set but HKASK_DB_PASSPHRASE missing")
                        })?;
                    let db = hkask_storage::Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open memory database: {}", e))?;
                    let conn = db.conn_arc();
                    let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                    let embedding_store = hkask_storage::EmbeddingStore::new(conn);
                    Some(hkask_memory::SemanticMemory::new(
                        triple_store,
                        embedding_store,
                    ))
                }
                None => None,
            };
            DocKnowledgeServer::new(ctx.webid, semantic)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_MEMORY_DB",
                "Path to per-agent memory database for QA storage (in-memory if absent)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the database (required if HKASK_MEMORY_DB is set)",
            ),
        ],
    )
    .await
}
