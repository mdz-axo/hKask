//! hkask-acp — ACP (Agent Client Protocol) replicant binary
//!
//! Presents hKask coding agents in IDEs (Zed, VS Code, etc.) via the
//! Agent Client Protocol (agentclientprotocol.com). The replicant:
//!
//! 1. Connects to the hKask daemon for auth, capability, and memory
//! 2. Implements ACP JSON-RPC 2.0 over stdio
//! 3. Routes prompts through hKask's inference router
//! 4. Encodes interactions as episodic memory triples
//! 5. Registers CNS spans for observability
//!
//! The ACP wire protocol is JSON-RPC 2.0 over stdin/stdout. Message types
//! follow the Agent Client Protocol specification v1. When the
//! `agent-client-protocol` crate's simpler `Agent` trait ships, this
//! manual implementation will be replaced by the trait impl.
//!
//! # Startup
//!
//! ```text
//! HKASK_REPLICANT=<name> hkask-acp
//! ```

mod protocol;

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::daemon::DaemonClient;
use hkask_mcp::startup::{StartupGateResult, verify_startup_gates};
use hkask_types::cns::CnsSpan;
use hkask_types::ports::{InferencePort, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use protocol::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

const ENV_REPLICANT: &str = "HKASK_REPLICANT";

struct SessionState {
    #[allow(dead_code)]
    session_id: String,
    #[allow(dead_code)]
    cwd: String,
    #[allow(dead_code)]
    created_at: i64,
}

struct HkaskAcpAgent {
    replicant: String,
    daemon: DaemonClient,
    inference: Arc<InferenceRouter>,
    default_model: String,
    sessions: Mutex<HashMap<String, SessionState>>,
    _gate_result: StartupGateResult,
}

impl HkaskAcpAgent {
    async fn build() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "hkask.acp=info".into()),
            )
            .init();

        let replicant = std::env::var(ENV_REPLICANT).unwrap_or_else(|_| {
            warn!("HKASK_REPLICANT not set, using default 'acp-replicant'");
            "acp-replicant".to_string()
        });

        let daemon = DaemonClient::new();
        let gate_result = verify_startup_gates(&daemon, &replicant, "acp", &["inference:call"])
            .await
            .map_err(|e| anyhow::anyhow!("Startup gates failed: {}", e))?;

        info!(
            target: "hkask.acp",
            replicant = %replicant,
            "P4 gates verified — {} tool(s) denied: {:?}",
            gate_result.denied_tools.len(),
            gate_result.denied_tools
        );

        let inference = Arc::new(InferenceRouter::new(InferenceConfig::from_env()));
        let default_model = std::env::var("HKASK_MODEL").unwrap_or_else(|_| "qwen3:8b".to_string());

        cns_emit(CnsSpan::AcpIdeConnectionState, &replicant, "connected");

        Ok(Self {
            replicant,
            daemon,
            inference,
            default_model,
            sessions: Mutex::new(HashMap::new()),
            _gate_result: gate_result,
        })
    }

    async fn run_inference_stream(
        &self,
        prompt: &str,
        session_id: &str,
        stdout: &mut tokio::io::Stdout,
    ) -> Result<String, String> {
        use futures_util::StreamExt;

        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 4096,
            ..Default::default()
        };

        let port: Arc<dyn InferencePort> = Arc::clone(&self.inference) as Arc<dyn InferencePort>;
        let start = std::time::Instant::now();
        let mut stream =
            port.generate_stream_with_model(prompt, &params, Some(&self.default_model));

        let mut total_text = String::new();
        let mut finish_reason = String::from("end_turn");
        let mut total_tokens = 0u32;
        let message_id = format!("msg-{}", uuid::Uuid::new_v4());
        let mut tool_call_counter = 0u32;

        while let Some(chunk_result) = stream.next().await {
            let chunk: InferenceStreamChunk =
                chunk_result.map_err(|e| format!("Stream error: {}", e))?;

            // Tool calls in this chunk
            for tc in &chunk.tool_calls {
                tool_call_counter += 1;
                let tc_id = format!("tc-{}-{}", session_id, tool_call_counter);
                let kind = map_tool_kind(tc);

                let notif = tool_call_notification(
                    session_id,
                    &tc_id,
                    &format!("{} {}", tc.server, tc.tool),
                    &kind,
                );
                write_notification(stdout, &notif)
                    .await
                    .map_err(|e| format!("Write error: {}", e))?;

                // Mark in-progress
                let update = tool_call_update(session_id, &tc_id, "in_progress", None);
                write_notification(stdout, &update)
                    .await
                    .map_err(|e| format!("Write error: {}", e))?;

                // Mark completed
                let update = tool_call_update(
                    session_id,
                    &tc_id,
                    "completed",
                    Some(&format!("Tool call: {} {}", tc.server, tc.tool)),
                );
                write_notification(stdout, &update)
                    .await
                    .map_err(|e| format!("Write error: {}", e))?;
            }

            // Text content
            if !chunk.text_delta.is_empty() {
                total_text.push_str(&chunk.text_delta);
                let notif = agent_message_chunk(session_id, &message_id, &chunk.text_delta);
                write_notification(stdout, &notif)
                    .await
                    .map_err(|e| format!("Write error: {}", e))?;
            }

            // Track usage and finish reason from final chunk
            if let Some(ref usage) = chunk.usage {
                total_tokens = usage.total_tokens;
            }
            if let Some(ref fr) = chunk.finish_reason {
                finish_reason = fr.clone();
            }
        }

        let latency_ms = start.elapsed().as_millis() as u64;

        // Usage update notification
        let usage_notif = usage_update(session_id, total_tokens, total_tokens);
        write_notification(stdout, &usage_notif)
            .await
            .map_err(|e| format!("Write error: {}", e))?;

        info!(
            target: "hkask.acp",
            session_id = %session_id,
            latency_ms = %latency_ms,
            tokens = %total_tokens,
            finish_reason = %finish_reason,
            text_len = total_text.len(),
            tool_calls = tool_call_counter,
            "Inference stream complete"
        );

        // Encode memory
        let entity = format!("session:{}:prompt", session_id);
        let _ = self
            .daemon
            .store_experience(
                &self.replicant,
                &entity,
                "response",
                &serde_json::json!({
                    "response_len": total_text.len(),
                    "tokens": total_tokens,
                    "model": &self.default_model,
                    "finish": &finish_reason,
                    "tool_calls": tool_call_counter,
                }),
                Some(0.9),
            )
            .await;

        // Map finish_reason to ACP StopReason
        let stop_reason = match finish_reason.as_str() {
            "stop" | "end_turn" => "end_turn",
            "length" => "max_tokens",
            "tool_calls" => "end_turn",
            _ => "end_turn",
        };

        Ok(stop_reason.to_string())
    }

    // run_inference removed — replaced by run_inference_stream
}

/// Map a StructuredToolCall to an ACP tool kind string.
fn map_tool_kind(tc: &hkask_types::ports::inference_types::StructuredToolCall) -> String {
    match tc.tool.as_str() {
        "web_search" | "brave_search" | "tavily_search" => "search".into(),
        "web_extract" | "fetch" | "scrape" => "fetch".into(),
        "execute" | "run" | "shell" => "execute".into(),
        "read" | "cat" => "read".into(),
        "write" | "edit" | "patch" => "edit".into(),
        "delete" | "rm" => "delete".into(),
        "think" | "reason" | "plan" => "think".into(),
        _ => "other".into(),
    }
}

fn cns_emit(span: CnsSpan, replicant: &str, detail: &str) {
    info!(
        target: "hkask.acp",
        cns_span = %span,
        replicant = %replicant,
        detail = %detail,
        "CNS"
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let agent = Arc::new(HkaskAcpAgent::build().await?);
    info!(target: "hkask.acp", replicant = %agent.replicant, "ACP replicant starting");

    let mut transport = StdioTransport::new();
    transport.serve(Arc::clone(&agent)).await?;

    cns_emit(
        CnsSpan::AcpIdeConnectionState,
        &agent.replicant,
        "disconnected",
    );
    Ok(())
}
