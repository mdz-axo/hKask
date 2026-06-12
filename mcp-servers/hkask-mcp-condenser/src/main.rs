//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Loop: Episodic (Loop 2) — Confirmed. Context condensation operates on the active
//! conversation window, which is episodic in nature. The condenser compresses and persists
//! tool outputs within the episodic memory boundary.
//!
//! Provides compression algorithms (rtk_style, saliency_rank, flashrank) for reducing
//! tool output size while preserving essential information. Phase 1 implements local
//! CPU-only algorithms with no LLM dependency. Phase 2 adds LLM-assisted
//! thread summarization via a local or cloud inference engine.
//! Supports Ollama (/api/chat) and OpenAI-compatible (/v1/chat/completions)
//! endpoints (OpenRouter, LiteLLM, etc.). Format detected from INFERENCE_URL.
//!
//! When `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` are provided, the condenser can
//! persist compressed outputs to episodic memory via the `condenser:persist` tool.
//! Without those credentials, the server operates in memory-only mode (graceful
//! degradation).
//!
//! When `INFERENCE_URL` is provided, the
//! `condenser_thread_summary` tool calls the inference engine
//! to summarize conversation history for context condensation.

use hkask_mcp::server::{CapabilityTier, McpToolError, ToolSpanGuard, api_post};
use hkask_mcp_condenser::engine::CondenserEngine;
use hkask_mcp_condenser::inference::{self, ApiFormat};
use hkask_mcp_condenser::types::*;
use hkask_memory::EpisodicMemory;
use hkask_storage::{Database, Triple};
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::{Arc, Mutex};

/// System prompt for the thread-summary inference request.
const THREAD_SUMMARY_SYSTEM_PROMPT: &str = "You are a context condensation assistant. Produce structured summaries that \
     preserve technical details (file paths, error messages, decisions) while \
     eliminating verbosity. Use bullet points. Be concise.";

/// Context window size passed to the inference engine for thread summarization.
const THREAD_SUMMARY_NUM_CTX: u32 = 8192;

pub struct CondenserServer {
    webid: WebID,
    engine: Mutex<CondenserEngine>,
    episodic: Option<Arc<EpisodicMemory>>,
    inference_url: Option<String>,
    inference_model: String,
    http_client: reqwest::Client,
    capability_tier: CapabilityTier,
}

impl CondenserServer {
    fn new(
        webid: WebID,
        episodic: Option<EpisodicMemory>,
        inference_url: Option<String>,
        inference_model: String,
        inference_api_key: Option<String>,
        inference_timeout_secs: u64,
        capability_tier: CapabilityTier,
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
            capability_tier,
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
        let engine = match self.engine.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return span.internal_error(serde_json::json!({"error": "engine lock poisoned"}));
            }
        };
        let health = engine.check_global_health();
        let mode = if self.capability_tier.embedded {
            "embedded"
        } else {
            "standalone"
        };
        span.ok_json(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "mode": mode,
            "capabilities": {
                "persistence": self.has_persistence(),
                "inference": self.has_inference(),
                "keystore": self.capability_tier.keystore_available,
                "cns": self.capability_tier.cns_available(),
            },
            "profile": engine.stats.current_profile,
            "algorithms": engine.registry.list_algorithms(),
            "health": health,
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
        let mut engine = match self.engine.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return span.internal_error(serde_json::json!({"error": "engine lock poisoned"}));
            }
        };
        let result = engine.compress(&tool_name, &output, cat);
        // CompressedOutput contains only strings, integers, and a clamped f64 — never NaN/Inf.
        span.ok_json(
            serde_json::to_value(&result).expect("CompressedOutput serialization is infallible"),
        )
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
        let mut engine = match self.engine.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return span.internal_error(serde_json::json!({"error": "engine lock poisoned"}));
            }
        };
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
        let engine = match self.engine.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return span.internal_error(serde_json::json!({"error": "engine lock poisoned"}));
            }
        };
        // CondenserStats contains only strings and integers — never NaN/Inf.
        span.ok_json(
            serde_json::to_value(engine.get_stats())
                .expect("CondenserStats serialization is infallible"),
        )
    }

    #[tool(description = "Classify tool name to context category")]
    async fn condenser_classify(
        &self,
        Parameters(ClassifyRequest { tool_name }): Parameters<ClassifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_classify", &self.webid);
        let engine = match self.engine.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return span.internal_error(serde_json::json!({"error": "engine lock poisoned"}));
            }
        };
        let (category, algorithm) = engine.classify(&tool_name);
        span.ok_json(serde_json::json!({
            "tool_name": tool_name,
            "category": category.label(),
            "algorithm": algorithm,
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
        description = "Summarize conversation history using a local inference engine for context condensation. Call when approaching context window limits to condense older messages."
    )]
    async fn condenser_thread_summary(
        &self,
        Parameters(ThreadSummaryRequest {
            messages,
            current_query,
            max_tokens,
            model,
            inference_url,
        }): Parameters<ThreadSummaryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_thread_summary", &self.webid);

        // Use request-provided inference config if given, otherwise fall back to server defaults.
        let effective_url = inference_url.as_deref().or(self.inference_url.as_deref());
        let effective_model = model.as_deref().unwrap_or(&self.inference_model);

        let Some(inference_url) = effective_url else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Inference not configured — set INFERENCE_URL to enable thread summarization",
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

        // Detect API format from the effective URL (may differ from server default).
        let api_format = inference::detect_format(inference_url);

        let chat_request = inference::build_chat_request(
            api_format,
            effective_model,
            &summarization_prompt,
            THREAD_SUMMARY_SYSTEM_PROMPT,
            THREAD_SUMMARY_NUM_CTX,
            max_tok,
        );

        let url = match api_format {
            ApiFormat::Ollama => {
                format!("{}/api/chat", inference_url.trim_end_matches('/'))
            }
            ApiFormat::OpenAi => {
                // Base URL already ends with /v1 (e.g. https://openrouter.ai/api/v1)
                format!("{}/chat/completions", inference_url.trim_end_matches('/'))
            }
        };

        // Use shared api_post with automatic HTTP error classification
        let resp_body = match api_post(&self.http_client, "inference", &url, &chat_request).await {
            Ok(v) => v,
            Err(e) => return span.error(e.kind, e.to_json_string()),
        };

        // Extract and validate the summary content
        let summary = match inference::extract_summary(api_format, &resp_body) {
            Ok(s) => s,
            Err(e) => return span.error(McpErrorKind::Internal, e.to_json_string()),
        };

        let result =
            inference::build_summary_output(summary, msg_count, effective_model.to_string(), url);

        // ThreadSummaryOutput contains only strings and integers — never NaN/Inf.
        span.ok_json(
            serde_json::to_value(&result).expect("ThreadSummaryOutput serialization is infallible"),
        )
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

            // Inference endpoint configuration
            let inference_url = ctx.credentials.get("INFERENCE_URL").cloned();
            let inference_model = ctx
                .credentials
                .get("INFERENCE_MODEL")
                .cloned()
                .unwrap_or_else(|| "qwen3:8b".to_string());
            let inference_api_key = ctx.credentials.get("INFERENCE_API_KEY").cloned();
            let inference_timeout_secs = ctx
                .credentials
                .get("INFERENCE_TIMEOUT_SECS")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);

            CondenserServer::new(
                ctx.webid,
                episodic,
                inference_url,
                inference_model,
                inference_api_key,
                inference_timeout_secs,
                ctx.capability_tier,
            )
        },
        credential_requirements(),
    )
    .await
}

fn credential_requirements() -> Vec<hkask_mcp::CredentialRequirement> {
    let opt = hkask_mcp::CredentialRequirement::optional;
    vec![
        opt(
            "HKASK_DB_PATH",
            "Path to the SQLite database for episodic persistence",
        ),
        opt(
            "HKASK_DB_PASSPHRASE",
            "Passphrase for the database (required if HKASK_DB_PATH is set)",
        ),
        opt(
            "INFERENCE_URL",
            "Inference engine URL for thread summarization (Ollama, Fireworks, DeepInfra, or OpenAI-compatible)",
        ),
        opt(
            "INFERENCE_MODEL",
            "Model for summarization (default: qwen3:8b)",
        ),
        opt(
            "INFERENCE_API_KEY",
            "API key if the inference endpoint requires authentication",
        ),
        opt(
            "INFERENCE_TIMEOUT_SECS",
            "Timeout for inference requests in seconds (default: 30)",
        ),
    ]
}
