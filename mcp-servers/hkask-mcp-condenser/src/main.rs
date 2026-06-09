//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Loop: Episodic (Loop 2) — Confirmed. Context condensation operates on the active
//! conversation window, which is episodic in nature. The condenser compresses and persists
//! tool outputs within the episodic memory boundary.
//!
//! Provides compression algorithms (rtk_style, saliency_rank, flashrank) for reducing
//! tool output size while preserving essential information. Phase 1 implements local
//! CPU-only algorithms with no LLM dependency. Phase 2 adds LLM-assisted
//! thread summarization via a local inference engine (Okapi, Ollama, or any
//! /api/chat-compatible endpoint).
//!
//! When `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` are provided, the condenser can
//! persist compressed outputs to episodic memory via the `condenser:persist` tool.
//! Without those credentials, the server operates in memory-only mode (graceful
//! degradation).
//!
//! When `INFERENCE_URL` (or legacy `OKAPI_URL`) is provided, the
//! `condenser_thread_summary` tool calls the inference engine
//! to summarize conversation history for context compaction.

mod algorithms;
mod engine;
mod inference;
mod types;

use hkask_mcp::server::{McpToolError, ToolSpanGuard, api_post};
use hkask_memory::EpisodicMemory;
use hkask_storage::{Database, Triple};
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::{Arc, Mutex};

use engine::CondenserEngine;
use types::*;

pub struct CondenserServer {
    webid: WebID,
    engine: Mutex<CondenserEngine>,
    episodic: Option<Arc<EpisodicMemory>>,
    inference_url: Option<String>,
    inference_model: String,
    http_client: reqwest::Client,
}

impl CondenserServer {
    fn new(
        webid: WebID,
        episodic: Option<EpisodicMemory>,
        inference_url: Option<String>,
        inference_model: String,
        inference_api_key: Option<String>,
        inference_timeout_secs: u64,
    ) -> Result<Self, anyhow::Error> {
        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(inference_timeout_secs));

        if let Some(api_key) = &inference_api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            let auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))?;
            headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            client_builder = client_builder.default_headers(headers);
        }

        Ok(Self {
            webid,
            engine: Mutex::new(CondenserEngine::new()),
            episodic: episodic.map(Arc::new),
            inference_url,
            inference_model,
            http_client: client_builder.build()?,
        })
    }

    fn has_persistence(&self) -> bool {
        self.episodic.is_some()
    }

    fn has_inference(&self) -> bool {
        self.inference_url.is_some()
    }
}

#[tool_router(server_handler)]
impl CondenserServer {
    #[tool(description = "Liveness and profile info")]
    async fn condenser_ping(&self) -> String {
        let span = ToolSpanGuard::new("condenser_ping", &self.webid);
        let engine = self.engine.lock().unwrap();
        span.ok_json(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "profile": engine.stats.current_profile,
            "algorithms": engine.registry.list_algorithms(),
            "persistence": self.has_persistence(),
            "inference": self.has_inference(),
            "inference_url": self.inference_url,
            "inference_model": self.inference_model,
        }))
    }

    #[tool(description = "Compress tool output using context-aware algorithms")]
    async fn condenser_compress(
        &self,
        Parameters(CompressRequest {
            tool_name,
            output,
            category,
        }): Parameters<CompressRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_compress", &self.webid);
        if output.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("output must not be empty").to_json_string(),
            );
        }
        let cat = match category.as_deref() {
            Some(c) => match c.parse::<ContextCategory>() {
                Ok(cat) => Some(cat),
                Err(e) => {
                    return span.error(McpErrorKind::InvalidArgument, e.to_json_string());
                }
            },
            None => None,
        };
        let mut engine = self.engine.lock().unwrap();
        let result = engine.compress(&tool_name, &output, cat);
        span.ok_json(serde_json::to_value(&result).unwrap_or_default())
    }

    #[tool(description = "Set compression profile (heavy/normal/soft/light)")]
    async fn condenser_set_profile(
        &self,
        Parameters(SetProfileRequest { profile }): Parameters<SetProfileRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_set_profile", &self.webid);
        let p = match profile.parse::<Profile>() {
            Ok(p) => p,
            Err(e) => return span.error(McpErrorKind::InvalidArgument, e.to_json_string()),
        };
        let mut engine = self.engine.lock().unwrap();
        engine.set_profile(p);
        span.ok_json(serde_json::json!({
            "profile": p.to_string(),
            "retention_pct": p.retention_pct(),
            "max_lines": p.max_lines(),
        }))
    }

    #[tool(description = "Cumulative compression statistics")]
    async fn condenser_stats(&self) -> String {
        let span = ToolSpanGuard::new("condenser_stats", &self.webid);
        let engine = self.engine.lock().unwrap();
        span.ok_json(serde_json::to_value(engine.get_stats()).unwrap_or_default())
    }

    #[tool(description = "Classify tool name to context category")]
    async fn condenser_classify(
        &self,
        Parameters(ClassifyRequest { tool_name }): Parameters<ClassifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_classify", &self.webid);
        let category = classify_tool(&tool_name);
        let engine = self.engine.lock().unwrap();
        let algo = engine.registry.select(category);
        span.ok_json(serde_json::json!({
            "tool_name": tool_name,
            "category": category.label(),
            "algorithm": algo.name(),
        }))
    }

    #[tool(description = "Persist a compressed output to episodic memory")]
    async fn condenser_persist(
        &self,
        Parameters(PersistRequest {
            tool_name,
            compressed_output,
            confidence,
        }): Parameters<PersistRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_persist", &self.webid);

        let Some(episodic) = &self.episodic else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Persistence not available — set HKASK_DB_PATH and HKASK_DB_PASSPHRASE",
                )
                .to_json_string(),
            );
        };

        if compressed_output.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("compressed_output must not be empty")
                    .to_json_string(),
            );
        }

        let entity = format!("condenser:{tool_name}");
        let triple = Triple::new(
            &entity,
            "content",
            serde_json::Value::String(compressed_output),
            self.webid,
        )
        .with_perspective(self.webid)
        .with_visibility(Visibility::Private)
        .with_confidence(confidence.unwrap_or(1.0));

        match episodic.store(triple) {
            Ok(()) => span.ok_json(serde_json::json!({
                "persisted": true,
                "entity": entity,
                "attribute": "content",
                "perspective": self.webid.to_string(),
            })),
            Err(e) =>
                span.internal_error(serde_json::json!({"error": format!("Failed to persist to episodic memory: {}", e)})),
        }
    }

    #[tool(
        description = "Summarize conversation history using a local inference engine for context compaction. Call when approaching context window limits to condense older messages."
    )]
    async fn condenser_thread_summary(
        &self,
        Parameters(ThreadSummaryRequest {
            messages,
            current_query,
            max_tokens,
        }): Parameters<ThreadSummaryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_thread_summary", &self.webid);

        let Some(inference_url) = &self.inference_url else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Inference not configured — set INFERENCE_URL (or OKAPI_URL) to enable thread summarization",
                )
                .to_json_string(),
            );
        };

        let msg_count = messages.len();
        if msg_count == 0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("messages array is empty").to_json_string(),
            );
        }

        let conversation_text = inference::format_conversation_text(&messages);
        let max_tok = max_tokens.unwrap_or(500);

        let summarization_prompt =
            inference::build_summarization_prompt(&conversation_text, &current_query);

        let chat_request = serde_json::json!({
            "model": self.inference_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a context condensation assistant. Produce structured summaries that preserve technical details (file paths, error messages, decisions) while eliminating verbosity. Use bullet points. Be concise."
                },
                {
                    "role": "user",
                    "content": summarization_prompt
                }
            ],
            "stream": false,
            "think": false,
            "options": {
                "num_ctx": 8192,
                "num_predict": max_tok
            }
        });

        let url = format!("{}/api/chat", inference_url.trim_end_matches('/'));

        // Use shared api_post with automatic HTTP error classification
        let resp_body = match api_post(&self.http_client, "inference", &url, &chat_request).await {
            Ok(v) => v,
            Err(e) => return span.error(e.kind, e.to_json_string()),
        };

        // Extract and validate the summary content
        let summary = match inference::extract_summary(&resp_body) {
            Ok(s) => s,
            Err((kind, err)) => return span.error(kind, err.to_json_string()),
        };

        let result =
            inference::build_summary_output(summary, msg_count, self.inference_model.clone(), url);

        span.ok_json(serde_json::to_value(&result).unwrap_or_default())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-condenser",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let episodic = match ctx.credentials.get("HKASK_DB_PATH") {
                Some(path) => {
                    let passphrase =
                        ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                            anyhow::anyhow!("HKASK_DB_PATH set but HKASK_DB_PASSPHRASE missing")
                        })?;
                    let db = Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open condenser database: {}", e))?;
                    let triple_store = hkask_storage::TripleStore::new(db.conn_arc());
                    Some(hkask_memory::EpisodicMemory::new(triple_store))
                }
                None => None,
            };

            // Inference endpoint configuration (INFERENCE_URL preferred, OKAPI_URL for backward compat)
            let inference_url = ctx.credentials.get("INFERENCE_URL")
                .cloned()
                .or_else(|| ctx.credentials.get("OKAPI_URL").cloned());
            let inference_model = ctx
                .credentials
                .get("INFERENCE_MODEL")
                .or_else(|| ctx.credentials.get("OKAPI_MODEL"))
                .cloned()
                .unwrap_or_else(|| "qwen3:8b".to_string());
            let inference_api_key = ctx.credentials.get("INFERENCE_API_KEY")
                .or_else(|| ctx.credentials.get("OKAPI_API_KEY"))
                .cloned();
            let inference_timeout_secs = ctx
                .credentials
                .get("INFERENCE_TIMEOUT_SECS")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);

            CondenserServer::new(ctx.webid, episodic, inference_url, inference_model, inference_api_key, inference_timeout_secs)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PATH",
                "Path to the SQLite database for episodic persistence (in-memory if absent)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the database (required if HKASK_DB_PATH is set)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "INFERENCE_URL",
                "Inference engine URL for thread summarization (e.g. http://127.0.0.1:11435). OKAPI_URL also accepted for backward compatibility.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "INFERENCE_MODEL",
                "Model for summarization (default: qwen3:8b). OKAPI_MODEL also accepted.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "INFERENCE_API_KEY",
                "API key if authentication is enabled. OKAPI_API_KEY also accepted.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "INFERENCE_TIMEOUT_SECS",
                "Timeout for inference requests in seconds (default: 30)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OKAPI_URL",
                "[Legacy] Alias for INFERENCE_URL. Prefer INFERENCE_URL for new deployments.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OKAPI_MODEL",
                "[Legacy] Alias for INFERENCE_MODEL. Prefer INFERENCE_MODEL for new deployments.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OKAPI_API_KEY",
                "[Legacy] Alias for INFERENCE_API_KEY. Prefer INFERENCE_API_KEY for new deployments.",
            ),
        ],
    )
    .await
}
