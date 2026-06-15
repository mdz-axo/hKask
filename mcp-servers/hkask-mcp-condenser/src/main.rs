//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Loop: Episodic (Loop 2) — Confirmed. Context condensation operates on the active
//! conversation window, which is episodic in nature. The condenser compresses and persists
//! tool outputs within the episodic memory boundary.
//!
//! Provides compression algorithms (rtk_style, saliency_rank, flashrank) for reducing
//! tool output size while preserving essential information. Phase 1 implements local
//! CPU-only algorithms with no LLM dependency. Phase 2 adds LLM-assisted
//! thread summarization via the centralized hKask inference router.
//!
//! When `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` environment variables are set,
//! the condenser can persist compressed outputs to episodic memory via the
//! `condenser:persist` tool. Without them, the server operates in memory-only
//! mode (the default — no persistence backend required).
//!
//! The `condenser_thread_summary` tool uses the centralized `InferencePort`
//! (hkask-inference router) for LLM-powered summarization. No standalone
//! HTTP client or inference URL configuration is needed — the router handles
//! provider dispatch (Ollama, Fireworks, DeepInfra) automatically.

use hkask_condenser::engine::CondenserEngine;
use hkask_condenser::inference;
use hkask_condenser::types::*;
use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::server::{CapabilityTier, McpToolError, ToolSpanGuard};
use hkask_memory::EpisodicMemory;
use hkask_storage::{Database, Triple};
use hkask_types::ports::InferencePort;
use hkask_types::{LLMParameters, McpErrorKind, Visibility, WebID, now_rfc3339};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::{Arc, Mutex};

/// System prompt for the thread-summary inference request.
const THREAD_SUMMARY_SYSTEM_PROMPT: &str = "You are a context condensation assistant. Produce structured summaries that \
     preserve technical details (file paths, error messages, decisions) while \
     eliminating verbosity. Use bullet points. Be concise.";

pub struct CondenserServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<hkask_mcp::DaemonClient>,
    engine: Mutex<CondenserEngine>,
    episodic: Option<Arc<EpisodicMemory>>,
    /// Centralized inference port (hkask-inference router)
    inference_port: Arc<dyn InferencePort>,
    /// Default model for thread summarization (e.g., "qwen3:8b")
    default_model: String,
    capability_tier: CapabilityTier,
}

impl CondenserServer {
    fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        episodic: Option<EpisodicMemory>,
        inference_port: Arc<dyn InferencePort>,
        default_model: String,
        capability_tier: CapabilityTier,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            engine: Mutex::new(CondenserEngine::new()),
            episodic: episodic.map(Arc::new),
            inference_port,
            default_model,
            capability_tier,
        }
    }

    fn has_persistence(&self) -> bool {
        self.episodic.is_some()
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool,
                "input": input_summary,
                "outcome": outcome,
                "detail": detail,
                "timestamp": now_rfc3339(),
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
                        tracing::debug!(target: "hkask.mcp.condenser.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.condenser.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.condenser.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
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
                "inference": true,
                "keystore": self.capability_tier.keystore_available,
                "cns": self.capability_tier.cns_available(),
            },
            "profile": engine.stats.current_profile,
            "algorithms": engine.registry.list_algorithms(),
            "health": health,
            "default_model": self.default_model,
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
                    return span.error(
                        McpErrorKind::InvalidArgument,
                        McpToolError::invalid_argument(e).to_json_string(),
                    );
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

        self.record_experience(
            "condenser_compress",
            &tool_name,
            "success",
            serde_json::json!({ "original_size": output.len(), "compressed_size": result.content.len() }),
        );

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
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e).to_json_string(),
                );
            }
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
        description = "Summarize conversation history using the centralized hKask inference router for context condensation. Call when approaching context window limits to condense older messages."
    )]
    async fn condenser_thread_summary(
        &self,
        Parameters(ThreadSummaryRequest {
            messages,
            current_query,
            max_tokens,
            model,
        }): Parameters<ThreadSummaryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("condenser_thread_summary", &self.webid);

        let effective_model = model.as_deref().unwrap_or(&self.default_model);

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

        // Compose the full prompt: system + user
        let full_prompt = format!(
            "{}\n\nUser: {}",
            THREAD_SUMMARY_SYSTEM_PROMPT, summarization_prompt
        );

        let params = LLMParameters {
            temperature: 0.3,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: max_tok,
            seed: None,
            disable_thinking: true,
        };

        let result = match self
            .inference_port
            .generate_with_model(&full_prompt, &params, Some(effective_model))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Inference failed: {e}")).to_json_string(),
                );
            }
        };

        let summary = result.text;
        if summary.trim().is_empty() {
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal("Inference engine returned an empty summary")
                    .to_json_string(),
            );
        }
        let summary_len = summary.len();

        let output = inference::build_summary_output(
            summary,
            &conversation_text,
            msg_count,
            effective_model.to_string(),
        );

        self.record_experience(
            "condenser_thread_summary",
            &format!("{} messages", msg_count),
            "success",
            serde_json::json!({"model": effective_model.to_string(), "summary_length": summary_len}),
        );

        span.ok_json(
            serde_json::to_value(&output).expect("ThreadSummaryOutput serialization is infallible"),
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.condenser", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    // Build the centralized inference router from environment.
    let inference_config = InferenceConfig::from_env();
    let inference_router = InferenceRouter::new(inference_config);
    let inference_port: Arc<dyn InferencePort> = Arc::new(inference_router);

    hkask_mcp::run_server(
        "hkask-mcp-condenser",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let episodic = {
                let db_path = ctx
                    .credentials
                    .get("HKASK_DB_PATH")
                    .cloned()
                    .or_else(|| std::env::var("HKASK_DB_PATH").ok());
                match db_path {
                    Some(path) => {
                        let passphrase = ctx
                            .credentials
                            .get("HKASK_DB_PASSPHRASE")
                            .cloned()
                            .or_else(|| std::env::var("HKASK_DB_PASSPHRASE").ok())
                            .ok_or_else(|| {
                                anyhow::anyhow!("HKASK_DB_PATH set but HKASK_DB_PASSPHRASE missing")
                            })?;
                        let db = Database::open(&path, &passphrase).map_err(|e| {
                            anyhow::anyhow!("Failed to open condenser database: {}", e)
                        })?;
                        let triple_store = hkask_storage::TripleStore::new(db.conn_arc());
                        Some(hkask_memory::EpisodicMemory::new(triple_store))
                    }
                    None => None,
                }
            };

            let default_model = ctx
                .credentials
                .get("INFERENCE_MODEL")
                .cloned()
                .or_else(|| std::env::var("INFERENCE_MODEL").ok())
                .unwrap_or_else(|| "qwen3:8b".to_string());

            Ok(CondenserServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                episodic,
                Arc::clone(&inference_port),
                default_model,
                ctx.capability_tier,
            ))
        },
        credential_requirements(),
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(
        &client,
        replicant,
        "condenser",
        &[
            "compress",
            "classify",
            "set_profile",
            "stats",
            "ping",
            "persist",
            "thread_summary",
        ],
    )
    .await?;
    tracing::info!(target: "hkask.mcp.condenser", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}

fn credential_requirements() -> Vec<hkask_mcp::CredentialRequirement> {
    // HKASK_DB_PATH, HKASK_DB_PASSPHRASE, and INFERENCE_MODEL are handled
    // directly by the factory closure with sensible defaults (in-memory,
    // qwen3:8b) and optional env-var overrides. No credentials require
    // operator-level warnings.
    Vec::new()
}
