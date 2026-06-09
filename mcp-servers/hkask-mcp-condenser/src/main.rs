//! hKask MCP Condenser — Context condensation for tool outputs
//!
//! Loop: Episodic (Loop 2) — Confirmed. Context condensation operates on the active
//! conversation window, which is episodic in nature. The condenser compresses and persists
//! tool outputs within the episodic memory boundary.
//!
//! Provides compression algorithms (rtk_style, saliency_rank, flashrank) for reducing
//! tool output size while preserving essential information. Phase 1 implements local
//! CPU-only algorithms with no LLM dependency. Phase 2 adds LLM-assisted
//! thread summarization via Okapi's local inference.
//!
//! When `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` are provided, the condenser can
//! persist compressed outputs to episodic memory via the `condenser:persist` tool.
//! Without those credentials, the server operates in memory-only mode (graceful
//! degradation).
//!
//! When `OKAPI_URL` is provided, the `condenser_thread_summary` tool calls Okapi's
//! local inference to summarize conversation history for context compaction.

mod algorithms;
mod types;

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_memory::EpisodicMemory;
use hkask_storage::{Database, Triple};
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::sync::{Arc, Mutex};

use algorithms::AlgorithmRegistry;
use types::*;

struct CondenserEngine {
    registry: AlgorithmRegistry,
    profile: Profile,
    stats: CondenserStats,
}

impl CondenserEngine {
    fn new() -> Self {
        Self {
            registry: AlgorithmRegistry::new(),
            profile: Profile::Normal,
            stats: CondenserStats::default(),
        }
    }

    fn compress(
        &mut self,
        tool_name: &str,
        output: &str,
        category: Option<ContextCategory>,
    ) -> CompressedOutput {
        let cat = category.unwrap_or_else(|| classify_tool(tool_name));
        let algo = self.registry.select(cat);
        let algorithm_name = algo.name().to_string();

        let compressed_content = algo.compress(output, self.profile, cat);

        let original_lines = output.lines().count();
        let compressed_lines = compressed_content.lines().count();
        let original_bytes = output.len();
        let compressed_bytes = compressed_content.len();
        let reduction_pct = if original_bytes == 0 {
            0.0
        } else {
            (1.0 - (compressed_bytes as f64 / original_bytes as f64)) * 100.0
        };

        *self
            .stats
            .algorithm_usage
            .entry(algorithm_name.clone())
            .or_insert(0) += 1;
        *self
            .stats
            .category_usage
            .entry(cat.label().to_string())
            .or_insert(0) += 1;
        self.stats.total_compressions += 1;
        self.stats.total_original_bytes += original_bytes as u64;
        self.stats.total_compressed_bytes += compressed_bytes as u64;

        CompressedOutput {
            content: compressed_content,
            algorithm: algorithm_name,
            category: cat.label().to_string(),
            profile: self.profile.to_string(),
            original_lines,
            compressed_lines,
            original_bytes,
            compressed_bytes,
            reduction_pct,
        }
    }

    fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        self.stats.current_profile = profile.to_string();
    }

    fn get_stats(&self) -> &CondenserStats {
        &self.stats
    }
}

pub struct CondenserServer {
    webid: WebID,
    engine: Mutex<CondenserEngine>,
    episodic: Option<Arc<EpisodicMemory>>,
    okapi_url: Option<String>,
    okapi_model: String,
    okapi_api_key: Option<String>,
    http_client: reqwest::Client,
}

impl CondenserServer {
    fn new(
        webid: WebID,
        episodic: Option<EpisodicMemory>,
        okapi_url: Option<String>,
        okapi_model: String,
        okapi_api_key: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            webid,
            engine: Mutex::new(CondenserEngine::new()),
            episodic: episodic.map(Arc::new),
            okapi_url,
            okapi_model,
            okapi_api_key,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()?,
        })
    }

    fn has_persistence(&self) -> bool {
        self.episodic.is_some()
    }

    fn has_okapi(&self) -> bool {
        self.okapi_url.is_some()
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
            "okapi": self.has_okapi(),
            "okapi_url": self.okapi_url,
            "okapi_model": self.okapi_model,
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
        let cat = category
            .as_deref()
            .and_then(|c| c.parse::<ContextCategory>().ok());
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
            "compressed_output",
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
                "attribute": "compressed_output",
                "perspective": self.webid.to_string(),
            })),
            Err(e) =>
                span.internal_error(serde_json::json!({"error": format!("Failed to persist to episodic memory: {}", e)})),
        }
    }

    #[tool(
        description = "Summarize conversation history using Okapi local inference for context compaction. Call when approaching context window limits to condense older messages."
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

        let Some(okapi_url) = &self.okapi_url else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Okapi not configured — set OKAPI_URL to enable thread summarization",
                )
                .to_json_string(),
            );
        };

        // Parse the messages JSON
        let parsed: Vec<serde_json::Value> = match serde_json::from_str(&messages) {
            Ok(v) => v,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "messages must be a JSON array of {{role, content}} objects: {e}"
                    ))
                    .to_json_string(),
                );
            }
        };

        let msg_count = parsed.len();
        if msg_count == 0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("messages array is empty").to_json_string(),
            );
        }

        // Build the conversation text for summarization
        let mut conversation_text = String::new();
        for msg in &parsed {
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown");
            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
            conversation_text.push_str(&format!("[{role}]: {content}\n\n"));
        }

        let max_tok = max_tokens.unwrap_or(500);

        // Build the Okapi chat request
        let summarization_prompt = format!(
            "Summarize this conversation history for context compaction. \
             Preserve: key decisions, file paths mentioned, error states encountered, \
             code changes made, and the current task goal. \
             Discard: verbose tool output, intermediate file reads, repeated information, \
             and anything not directly relevant to the current task.\n\n\
             Current task: {current_query}\n\n\
             Conversation history:\n{conversation_text}"
        );

        let chat_request = serde_json::json!({
            "model": self.okapi_model,
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

        // Build the request URL
        let url = format!("{}/api/chat", okapi_url.trim_end_matches('/'));

        // Send the request
        let mut req_builder = self.http_client.post(&url).json(&chat_request);

        if let Some(api_key) = &self.okapi_api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = match req_builder.send().await {
            Ok(r) => r,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Okapi request failed: {e}")).to_json_string(),
                );
            }
        };

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Okapi returned HTTP {status}: {body}"))
                    .to_json_string(),
            );
        }

        let resp_body: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to parse Okapi response: {e}"))
                        .to_json_string(),
                );
            }
        };

        let summary = resp_body
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("(summary generation failed)")
            .to_string();

        let summary_tokens_approx = summary.split_whitespace().count();

        let result = ThreadSummaryOutput {
            summary,
            original_message_count: msg_count,
            summary_tokens_approx,
            okapi_model: self.okapi_model.clone(),
            okapi_url: url,
        };

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

            let okapi_url = ctx.credentials.get("OKAPI_URL").cloned();
            let okapi_model = ctx
                .credentials
                .get("OKAPI_MODEL")
                .cloned()
                .unwrap_or_else(|| "qwen3:8b".to_string());
            let okapi_api_key = ctx.credentials.get("OKAPI_API_KEY").cloned();

            CondenserServer::new(ctx.webid, episodic, okapi_url, okapi_model, okapi_api_key)
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
                "OKAPI_URL",
                "Okapi inference engine URL for thread summarization (e.g. http://127.0.0.1:11435)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OKAPI_MODEL",
                "Okapi model for summarization (default: qwen3:8b)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OKAPI_API_KEY",
                "Okapi API key if authentication is enabled",
            ),
        ],
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CondenserEngine ──

    // REQ: CondenserEngine starts with Normal profile
    #[test]
    fn engine_default_profile_is_normal() {
        let engine = CondenserEngine::new();
        assert_eq!(engine.profile, Profile::Normal);
    }

    // REQ: CondenserEngine starts with zero compression stats
    #[test]
    fn engine_starts_with_zero_stats() {
        let engine = CondenserEngine::new();
        let stats = engine.get_stats();
        assert_eq!(stats.total_compressions, 0);
        assert_eq!(stats.total_original_bytes, 0);
        assert_eq!(stats.total_compressed_bytes, 0);
    }

    // REQ: CondenserEngine.compress updates stats counters
    #[test]
    fn engine_compress_updates_stats() {
        let mut engine = CondenserEngine::new();
        let input = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        engine.compress("git_status", &input, None);
        let stats = engine.get_stats();
        assert_eq!(stats.total_compressions, 1);
        assert!(stats.total_original_bytes > 0);
        assert!(stats.total_compressed_bytes > 0);
        assert!(stats.algorithm_usage.contains_key("rtk_style"));
        assert!(stats.category_usage.contains_key("shell_command"));
    }

    // REQ: CondenserEngine.compress auto-classifies tool name when category is None
    #[test]
    fn engine_compress_auto_classifies() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress("git_status", "some output", None);
        assert_eq!(result.category, "shell_command");
    }

    // REQ: CondenserEngine.compress uses provided category when given
    #[test]
    fn engine_compress_uses_explicit_category() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress(
            "git_status",
            "some output",
            Some(ContextCategory::LogOutput),
        );
        assert_eq!(result.category, "log_output");
    }

    // REQ: CondenserEngine.compress returns correct algorithm name
    #[test]
    fn engine_compress_returns_algorithm_name() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress("git_status", "some output", None);
        assert_eq!(result.algorithm, "rtk_style");
    }

    // REQ: CondenserEngine.compress tracks profile in output
    #[test]
    fn engine_compress_tracks_profile() {
        let mut engine = CondenserEngine::new();
        let result = engine.compress("git_status", "some output", None);
        assert_eq!(result.profile, "normal");
    }

    // REQ: CondenserEngine.compress reports reduction_pct > 0 for long input under Heavy
    #[test]
    fn engine_compress_reports_reduction() {
        let mut engine = CondenserEngine::new();
        engine.set_profile(Profile::Heavy);
        let input = (0..500)
            .map(|i| format!("line {i} with content"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = engine.compress("git_status", &input, None);
        assert!(
            result.reduction_pct > 0.0,
            "should report positive reduction for long input under heavy profile"
        );
    }

    // REQ: CondenserEngine.compress reports 0% reduction for short input that passes through
    #[test]
    fn engine_compress_no_reduction_for_passthrough() {
        let mut engine = CondenserEngine::new();
        let input = "short";
        let result = engine.compress("git_status", input, None);
        assert_eq!(result.reduction_pct, 0.0);
    }

    // REQ: CondenserEngine.compress reports 0% reduction for empty-ish input
    #[test]
    fn engine_compress_zero_bytes_reduction() {
        let mut engine = CondenserEngine::new();
        // Empty string has 0 bytes; reduction_pct should be 0.0 per the guard
        let result = engine.compress("git_status", "", None);
        assert_eq!(result.reduction_pct, 0.0);
    }

    // REQ: CondenserEngine.set_profile changes the active profile
    #[test]
    fn engine_set_profile_changes_profile() {
        let mut engine = CondenserEngine::new();
        engine.set_profile(Profile::Heavy);
        assert_eq!(engine.profile, Profile::Heavy);
        assert_eq!(engine.stats.current_profile, "heavy");
    }

    // REQ: CondenserEngine multiple compressions accumulate stats
    #[test]
    fn engine_multiple_compressions_accumulate() {
        let mut engine = CondenserEngine::new();
        let input = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        engine.compress("git_status", &input, None);
        engine.compress("pytest_run", &input, None);
        engine.compress("log_journal", &input, None);
        let stats = engine.get_stats();
        assert_eq!(stats.total_compressions, 3);
        assert!(stats.algorithm_usage.contains_key("rtk_style"));
        assert!(stats.algorithm_usage.contains_key("saliency_rank"));
    }

    // REQ: CompressedOutput line counts are accurate
    #[test]
    fn engine_compress_line_counts_accurate() {
        let mut engine = CondenserEngine::new();
        let input = "line1\nline2\nline3";
        let result = engine.compress("git_status", input, None);
        assert_eq!(result.original_lines, 3);
        assert_eq!(result.original_bytes, input.len());
    }

    // REQ: CompressedOutput byte counts match content.len()
    #[test]
    fn engine_compress_byte_counts_match() {
        let mut engine = CondenserEngine::new();
        let input = (0..200)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = engine.compress("git_status", &input, None);
        assert_eq!(result.compressed_bytes, result.content.len());
        assert_eq!(result.original_bytes, input.len());
    }
}
