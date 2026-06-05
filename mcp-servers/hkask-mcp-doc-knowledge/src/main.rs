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

        if text.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("text must not be empty").to_json_string(),
            );
        }

        if entity_ref_prefix.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("entity_ref_prefix must not be empty")
                    .to_json_string(),
            );
        }

        validate_field!(span, "entity_ref_prefix", &entity_ref_prefix, 256);

        let max_tok = max_tokens.unwrap_or(512);
        let overlap_tok = overlap_tokens.unwrap_or(64);
        let max_words = tokens_to_words(max_tok);
        let min_words = tokens_to_words(overlap_tok).max(max_words / 4);
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
        let serialized: Vec<serde_json::Value> = passages
            .into_iter()
            .map(|(entity_ref, passage_text)| {
                json!({
                    "entity_ref": entity_ref,
                    "text": passage_text,
                })
            })
            .collect();

        span.ok_json(json!({
            "total_passages": total_passages,
            "passages": serialized,
            "max_tokens": max_tok,
            "overlap_tokens": overlap_tok,
            "max_words": max_words,
            "min_words": min_words,
            "sentence_boundary": boundary,
            "stripped_gutenberg": strip_gutenberg.unwrap_or(false),
        }))
    }

    #[tool(description = "Detect document format from path/extension")]
    async fn doc_knowledge_detect_format(
        &self,
        Parameters(DetectFormatRequest { path }): Parameters<DetectFormatRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_detect_format", &self.webid);

        if path.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("path must not be empty").to_json_string(),
            );
        }

        let ext = std::path::Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let (format, supported) = match ext.as_str() {
            "md" | "markdown" => ("markdown", true),
            "html" | "htm" => ("html", true),
            "txt" => ("plain", true),
            "pdf" => ("pdf", false),
            _ => ("unknown", false),
        };

        span.ok_json(json!({
            "format": format,
            "path": path,
            "supported": supported,
        }))
    }

    #[tool(description = "Extract text and image refs from markdown")]
    async fn doc_knowledge_extract_markdown(
        &self,
        Parameters(ExtractMarkdownRequest { content, label }): Parameters<ExtractMarkdownRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_extract_markdown", &self.webid);

        if content.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("content must not be empty").to_json_string(),
            );
        }

        // Split on --- frontmatter
        let text = if content.starts_with("---") {
            content
                .splitn(3, "---")
                .nth(2)
                .unwrap_or(&content)
                .trim()
                .to_string()
        } else {
            content.clone()
        };

        // Extract image refs: ![alt](url)
        let mut images = Vec::new();
        let mut i = 0;
        let bytes = text.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'!' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                if let Some(bracket_end) = text[i + 2..].find(']') {
                    let paren_start = i + 2 + bracket_end + 1;
                    if paren_start < bytes.len() && bytes[paren_start] == b'(' {
                        if let Some(paren_end) = text[paren_start + 1..].find(')') {
                            let url = &text[paren_start + 1..paren_start + 1 + paren_end];
                            if !url.is_empty() {
                                images.push(url.to_string());
                            }
                            i = paren_start + 1 + paren_end + 1;
                            continue;
                        }
                    }
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

    #[tool(description = "Extract text from HTML")]
    async fn doc_knowledge_extract_html(
        &self,
        Parameters(ExtractHtmlRequest { content, label }): Parameters<ExtractHtmlRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("doc_knowledge_extract_html", &self.webid);

        if content.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("content must not be empty").to_json_string(),
            );
        }

        // Simple state machine to strip HTML tags
        let mut result = String::with_capacity(content.len());
        let mut in_tag = false;
        for ch in content.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }

        // Collapse multiple whitespace
        let text = result.split_whitespace().collect::<Vec<&str>>().join(" ");

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

        if path.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("path must not be empty").to_json_string(),
            );
        }

        // Detect format
        let ext = std::path::Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let (format, supported) = match ext.as_str() {
            "md" | "markdown" => ("markdown", true),
            "html" | "htm" => ("html", true),
            "txt" => ("plain", true),
            "pdf" => ("pdf", false),
            _ => ("unknown", false),
        };

        if !supported {
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

        // Extract text based on format
        let text = match format {
            "markdown" => {
                if content.starts_with("---") {
                    content
                        .splitn(3, "---")
                        .nth(2)
                        .unwrap_or(&content)
                        .trim()
                        .to_string()
                } else {
                    content
                }
            }
            "html" => {
                let mut result = String::with_capacity(content.len());
                let mut in_tag = false;
                for ch in content.chars() {
                    match ch {
                        '<' => in_tag = true,
                        '>' => in_tag = false,
                        _ if !in_tag => result.push(ch),
                        _ => {}
                    }
                }
                result.split_whitespace().collect::<Vec<&str>>().join(" ")
            }
            _ => content,
        };

        // Multi-tier chunking
        let coarse_max = coarse_max_tokens.unwrap_or(2048);
        let medium_max = medium_max_tokens.unwrap_or(512);
        let fine_max = fine_max_tokens.unwrap_or(128);

        let entity_base = path.replace(['/', '\\', '.', ' '], "_");

        let coarse = SemanticMemory::chunk_text(
            &text,
            &format!("{}:coarse", entity_base),
            tokens_to_words(coarse_max / 4),
            tokens_to_words(coarse_max),
            ".!? ",
        );

        let medium = SemanticMemory::chunk_text(
            &text,
            &format!("{}:medium", entity_base),
            tokens_to_words(medium_max / 4),
            tokens_to_words(medium_max),
            ".!? ",
        );

        let fine = SemanticMemory::chunk_text(
            &text,
            &format!("{}:fine", entity_base),
            tokens_to_words(fine_max / 4),
            tokens_to_words(fine_max),
            ".!? ",
        );

        let serialize_passages = |passages: Vec<(String, String)>| -> Vec<serde_json::Value> {
            passages
                .into_iter()
                .map(|(entity_ref, passage_text)| {
                    json!({
                        "entity_ref": entity_ref,
                        "text": passage_text,
                    })
                })
                .collect()
        };

        span.ok_json(json!({
            "format": format,
            "path": path,
            "coarse_max_tokens": coarse_max,
            "medium_max_tokens": medium_max,
            "fine_max_tokens": fine_max,
            "coarse": serialize_passages(coarse),
            "medium": serialize_passages(medium),
            "fine": serialize_passages(fine),
        }))
    }

    #[tool(
        description = "Generate QA prompt from text chunk (returns structured prompt for LLM; actual LLM call routed through hkask-mcp-inference)"
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
            "prompt": prompt,
            "chunk_id": chunk_id,
            "strategy": strat,
            "bloom_levels": levels,
            "note": "Route this prompt through hkask-mcp-inference for LLM completion",
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
                .with_visibility(Visibility::Shared)
                .with_confidence(1.0);

            match semantic.store(triple) {
                Ok(()) => stored += 1,
                Err(e) => errors.push(format!("Item {i}: {e}")),
            }
        }

        if errors.is_empty() {
            span.ok_json(json!({
                "stored": stored,
                "source_document": source_document,
            }))
        } else {
            span.internal_error(json!({
                "stored": stored,
                "errors": errors,
                "source_document": source_document,
            }))
        }
    }
}

// ── Server entry point ───────────────────────────────────────────────────

hkask_mcp::mcp_server_main!(
    "hkask-mcp-doc-knowledge",
    factory: |ctx: hkask_mcp::ServerContext| {
        let semantic = match ctx.credentials.get("HKASK_MEMORY_DB") {
            Some(path) => {
                let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                    anyhow::anyhow!("HKASK_MEMORY_DB set but HKASK_DB_PASSPHRASE missing")
                })?;
                let db = hkask_storage::Database::open(path, passphrase)
                    .map_err(|e| anyhow::anyhow!("Failed to open memory database: {}", e))?;
                let conn = db.conn_arc();
                let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                let embedding_store = hkask_storage::EmbeddingStore::new(conn);
                Some(hkask_memory::SemanticMemory::new(triple_store, embedding_store))
            }
            None => None,
        };
        DocKnowledgeServer::new(ctx.webid, semantic)
    },
    credentials: vec![
        hkask_mcp::CredentialRequirement::optional(
            "HKASK_MEMORY_DB",
            "Path to per-agent memory database for QA storage (in-memory if absent)",
        ),
        hkask_mcp::CredentialRequirement::optional(
            "HKASK_DB_PASSPHRASE",
            "Passphrase for the database (required if HKASK_MEMORY_DB is set)",
        ),
    ]
);
