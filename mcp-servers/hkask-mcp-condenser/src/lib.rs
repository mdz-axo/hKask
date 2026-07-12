//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Loop: Episodic (Loop 2) — Confirmed. Context condensation operates on the active
//! conversation window, which is episodic in nature. The condenser compresses and persists
//! tool outputs within the episodic memory boundary.
//!
//! Provides compression algorithms (rtk_style, word_rank, flashrank) for reducing
//! tool output size while preserving essential information. `word_rank` uses
//! TF-IDF bag-of-words compression with ontology anchoring.
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
//! provider dispatch (DeepInfra, Together AI) automatically.

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use hkask_condenser::engine::CondenserEngine;
use hkask_condenser::inference;
use hkask_condenser::types::*;
use hkask_database::sqlite::SqliteDriver;
use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::server::{CapabilityTier, McpToolError, execute_tool};
use hkask_memory::EpisodicMemory;
use hkask_memory::SemanticMemory;
use hkask_ports::InferencePort;
use hkask_storage::{Database, HMem};
use hkask_types::Visibility;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

/// System prompt for the thread-summary inference request.
const THREAD_SUMMARY_SYSTEM_PROMPT: &str = "You are a context condensation assistant. Produce structured summaries that \
     preserve technical details (file paths, error messages, decisions) while \
     eliminating verbosity. Use bullet points. Be concise.";

hkask_mcp::mcp_server!(
    struct CondenserServer {
        pub engine: Mutex<CondenserEngine>,
        pub episodic: Option<Arc<EpisodicMemory>>,
        pub semantic: Option<Arc<SemanticMemory>>,
        pub inference_port: Arc<dyn InferencePort>,
        pub default_model: String,
        pub capability_tier: CapabilityTier,
    }
);

impl CondenserServer {
    /// Return persona keywords for word-frequency saliency scoring.
    /// These are charter-like terms that define what the agent cares about.
    fn persona_keywords(&self) -> Vec<String> {
        vec![
            "curator".into(),
            "monitor".into(),
            "alert".into(),
            "escalation".into(),
            "diagnose".into(),
            "calibrate".into(),
            "threshold".into(),
            "variety".into(),
            "deficit".into(),
            "backpressure".into(),
            "consolidation".into(),
            "semantic".into(),
            "episodic".into(),
        ]
    }

    pub fn has_persistence(&self) -> bool {
        self.episodic.is_some()
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
    pub async fn condenser_ping(&self) -> String {
        execute_tool(self, "condenser_ping", async {
            let engine = self
                .engine
                .lock()
                .map_err(|_| McpToolError::internal("engine lock poisoned"))?;
            let health = engine.check_global_health();
            let mode = if self.capability_tier.embedded {
                "embedded"
            } else {
                "standalone"
            };
            Ok(serde_json::json!({
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
        })
        .await
    }

    #[tool(
        description = "Compress tool output using context-aware algorithms with domain ontology anchoring (P8.1)"
    )]
    pub async fn condenser_compress(
        &self,
        Parameters(CompressRequest {
            tool_name,
            output,
            category,
        }): Parameters<CompressRequest>,
    ) -> String {
        execute_tool(self, "condenser_compress", async {
            if output.is_empty() {
                return Err(McpToolError::invalid_argument("output must not be empty"));
            }
            let cat = match category.as_deref() {
                Some(c) => match c.parse::<ContextCategory>() {
                    Ok(cat) => Some(cat),
                    Err(e) => {
                        return Err(McpToolError::invalid_argument(e));
                    }
                },
                None => None,
            };
            let mut engine = self.engine.lock().map_err(|_| McpToolError::internal("engine lock poisoned"))?;
            let result = engine.compress(&tool_name, &output, cat);

            self.record_experience(
                "condenser_compress",
                &tool_name,
                "success",
                serde_json::json!({ "original_size": output.len(), "compressed_size": result.content.len() }),
            );

            // CompressedOutput contains only strings, integers, and a clamped f64 — never NaN/Inf.
            Ok(
                serde_json::to_value(&result).expect("CompressedOutput serialization is infallible"),
            )
        }).await
    }

    #[tool(description = "Set compression profile (heavy/normal/soft/light)")]
    pub async fn condenser_set_profile(
        &self,
        Parameters(SetProfileRequest { profile }): Parameters<SetProfileRequest>,
    ) -> String {
        execute_tool(self, "condenser_set_profile", async {
            let p = match profile.parse::<Profile>() {
                Ok(p) => p,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(e));
                }
            };
            let mut engine = self
                .engine
                .lock()
                .map_err(|_| McpToolError::internal("engine lock poisoned"))?;
            engine.set_profile(p);
            Ok(serde_json::json!({
                "profile": p.to_string(),
                "retention_pct": p.retention_pct(),
                "max_lines": p.max_lines(),
            }))
        })
        .await
    }

    #[tool(description = "Cumulative compression statistics")]
    pub async fn condenser_stats(&self) -> String {
        execute_tool(self, "condenser_stats", async {
            let engine = self
                .engine
                .lock()
                .map_err(|_| McpToolError::internal("engine lock poisoned"))?;
            // CondenserStats contains only strings and integers — never NaN/Inf.
            Ok(serde_json::to_value(engine.get_stats())
                .expect("CondenserStats serialization is infallible"))
        })
        .await
    }

    #[tool(description = "Classify tool name to context category")]
    pub async fn condenser_classify(
        &self,
        Parameters(ClassifyRequest { tool_name }): Parameters<ClassifyRequest>,
    ) -> String {
        execute_tool(self, "condenser_classify", async {
            let engine = self
                .engine
                .lock()
                .map_err(|_| McpToolError::internal("engine lock poisoned"))?;
            let (category, algorithm) = engine.classify(&tool_name);
            Ok(serde_json::json!({
                "tool_name": tool_name,
                "category": category.label(),
                "algorithm": algorithm,
            }))
        })
        .await
    }

    #[tool(description = "Persist a compressed output to episodic memory")]
    pub async fn condenser_persist(
        &self,
        Parameters(PersistRequest {
            tool_name,
            compressed_output,
            confidence,
        }): Parameters<PersistRequest>,
    ) -> String {
        execute_tool(self, "condenser_persist", async {
            let Some(episodic) = &self.episodic else {
                return Err(McpToolError::permission_denied(
                    "Persistence not available — set HKASK_DB_PATH and HKASK_DB_PASSPHRASE",
                ));
            };

            if compressed_output.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "compressed_output must not be empty",
                ));
            }

            let entity = format!("condenser:{tool_name}");
            let h_mem = HMem::new(
                &entity,
                "content",
                serde_json::Value::String(compressed_output),
                self.webid,
            )
            .with_perspective(self.webid)
            .with_visibility(Visibility::Private)
            .with_confidence(confidence.unwrap_or(1.0));

            match episodic.store(h_mem) {
                Ok(()) => Ok(serde_json::json!({
                    "persisted": true,
                    "entity": entity,
                    "attribute": "content",
                    "perspective": self.webid.to_string(),
                })),
                Err(e) => Err(McpToolError::internal(format!(
                    "Failed to persist to episodic memory: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Summarize conversation history using the centralized hKask inference router for context condensation. Call when approaching context window limits to condense older messages."
    )]
    pub async fn condenser_thread_summary(
        &self,
        Parameters(ThreadSummaryRequest {
            messages,
            current_query,
            max_tokens,
            model,
        }): Parameters<ThreadSummaryRequest>,
    ) -> String {
        execute_tool(self, "condenser_thread_summary", async {
            let effective_model = model.as_deref().unwrap_or(&self.default_model);

            let msg_count = messages.len();
            if msg_count == 0 {
                return Err(McpToolError::invalid_argument("messages array is empty"));
            }

            let conversation_text = inference::format_conversation_text(&messages);
            let max_tok = max_tokens.unwrap_or_else(|| {
                // Fall back to HKASK_CONDENSE_SALIENCY_WINDOW env var as a
                // default hint. Higher saliency = user wants more context
                // preserved → longer summaries. Clamp to [150, 2000].
                let saliency = std::env::var("HKASK_CONDENSE_SALIENCY_WINDOW")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(5);
                (saliency * 100).clamp(150, 2000) as u32
            });

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
                adapter: None,
                bypass_fusion: true,
                fusion_config: None,
            };

            let result = match self
                .inference_port
                .generate_with_model(&full_prompt, &params, Some(effective_model), None)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return Err(McpToolError::internal(format!("Inference failed: {e}")));
                }
            };

            let summary = result.text;
            if summary.trim().is_empty() {
                return Err(McpToolError::internal("Inference engine returned an empty summary"));
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

            Ok(serde_json::to_value(&output).expect("ThreadSummaryOutput serialization is infallible"))
        }).await
    }

    #[tool(
        description = "Score text saliency against persona or memory. Returns 0.0-1.0 where higher = more relevant."
    )]
    pub async fn condenser_score_saliency(
        &self,
        Parameters(req): Parameters<SaliencyRequest>,
    ) -> String {
        execute_tool(self, "condenser_score_saliency", async {
            let score = match req.against.as_deref() {
                Some("memory") => {
                    // If semantic memory is available, query it word-by-word
                    if let Some(ref semantic) = self.semantic {
                        let query_words: Vec<&str> = req.text
                            .split_whitespace()
                            .filter(|w| w.len() > 3)
                            .take(5)
                            .collect();
                        let mut total_results: usize = 0;
                        for word in &query_words {
                            if let Ok(h_mems) = semantic.query_deduped(word) {
                                total_results += h_mems.len();
                            }
                        }
                        if total_results > 0 {
                            (0.5 + total_results as f64 * 0.15).min(1.0)
                        } else {
                            0.2
                        }
                    } else if let Some(ref episodic) = self.episodic {
                        // Fall back to episodic memory
                        let query_words: Vec<&str> = req.text
                            .split_whitespace()
                            .filter(|w| w.len() > 3)
                            .take(5)
                            .collect();
                        let mut total_results: usize = 0;
                        for word in &query_words {
                            if let Ok(h_mems) = episodic.query_for_deduped(word, self.webid) {
                                total_results += h_mems.len();
                            }
                        }
                        if total_results > 0 {
                            (0.5 + total_results as f64 * 0.15).min(1.0)
                        } else {
                            0.2
                        }
                    } else {
                        0.5 // No memory store — neutral
                    }
                }
                _ => {
                    // Score against persona keywords
                    let persona_keywords = self.persona_keywords();
                    hkask_condenser::saliency::score_against_persona(
                        &req.text,
                        &persona_keywords.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    )
                }
            };
            Ok(serde_json::json!({
                "score": score,
                "against": req.against.as_deref().unwrap_or("persona"),
                "method": if req.against.as_deref() == Some("memory") { "semantic_search" } else { "word_frequency" },
            }))
        }).await
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SaliencyRequest {
    pub text: String,
    #[serde(default)]
    pub against: Option<String>, // "persona" or "memory"
}

/// Run the condenser MCP server (used by binary target).
pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    // Build the centralized inference router from environment.
    let inference_config = InferenceConfig::from_env();
    let inference_router = InferenceRouter::new(inference_config);
    let inference_port: Arc<dyn InferencePort> = Arc::new(inference_router);

    hkask_mcp::run_server(
        "hkask-mcp-condenser",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            (|| -> anyhow::Result<CondenserServer> {
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
                                    anyhow::anyhow!(
                                        "HKASK_DB_PATH set but HKASK_DB_PASSPHRASE missing"
                                    )
                                })?;
                            let db = Database::open(&path, &passphrase).map_err(|e| {
                                anyhow::anyhow!("Failed to open condenser database: {}", e)
                            })?;
                            let pool =
                                db.sqlite_pool().map_err(|e| anyhow::anyhow!("pool: {e}"))?;
                            let hmem_driver = Arc::new(SqliteDriver::new(pool));
                            let h_mem_store = hkask_storage::HMemStore::from_driver(hmem_driver);
                            Some(hkask_memory::EpisodicMemory::new(h_mem_store))
                        }
                        None => None,
                    }
                };

                let default_model = ctx
                    .credentials
                    .get("INFERENCE_MODEL")
                    .cloned()
                    .or_else(|| std::env::var("INFERENCE_MODEL").ok())
                    .unwrap_or_else(|| "google/gemma-4-26B-A4B-it".to_string());

                Ok(CondenserServer::new(
                    ctx.webid,
                    replicant.clone(),
                    daemon_client.clone(),
                    Mutex::new(CondenserEngine::new()),
                    episodic.map(Arc::new),
                    None,
                    Arc::clone(&inference_port),
                    default_model,
                    ctx.capability_tier,
                ))
            })()
            .map_err(|e| hkask_mcp::McpError::UnexpectedResponse {
                context: "condenser server init".into(),
                detail: e.to_string(),
            })
        },
        vec![],
    )
    .await
}
