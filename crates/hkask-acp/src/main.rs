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
use hkask_types::ports::InferencePort;
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

    async fn run_inference(
        &self,
        prompt: &str,
        session_id: &str,
    ) -> Result<hkask_types::ports::inference_types::InferenceResult, String> {
        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 4096,
            ..Default::default()
        };

        let port: Arc<dyn InferencePort> = Arc::clone(&self.inference) as Arc<dyn InferencePort>;
        let start = std::time::Instant::now();
        let result = port
            .generate_with_model(prompt, &params, Some(&self.default_model))
            .await
            .map_err(|e| format!("Inference error: {}", e))?;

        let latency_ms = start.elapsed().as_millis() as u64;
        info!(
            target: "hkask.acp",
            session_id = %session_id,
            latency_ms = %latency_ms,
            tokens = %result.usage.total_tokens,
            "Inference complete"
        );

        // Encode interaction as memory triple via daemon
        let entity = format!("session:{}:prompt", session_id);
        let _ = self
            .daemon
            .store_experience(
                &self.replicant,
                &entity,
                "response",
                &serde_json::json!({
                    "response_len": result.text.len(),
                    "tokens": result.usage.total_tokens,
                    "model": result.model,
                    "finish": result.finish_reason,
                }),
                Some(0.9),
            )
            .await;

        Ok(result)
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
