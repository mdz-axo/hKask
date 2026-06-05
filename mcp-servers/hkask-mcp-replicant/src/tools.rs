//! MCP tools for replicant chat — exposes a hKask replicant as an MCP tool
//!
//! Three tools exposed via MCP protocol:
//! - `replicant:chat` — Send a message to a replicant and receive a response
//! - `replicant:status` — Check replicant registration and identity
//! - `replicant:history` — List recent conversation turns in the current session
//!
//! # Architecture
//!
//! This server bridges Zed's MCP context server model with hKask's pod-mediated
//! chat flow. When Zed's Agent Panel calls `replicant:chat`, this server:
//!
//! 1. Resolves the replicant persona name → WebID
//! 2. Loads the full agent definition from the YAML registry (if available),
//!    falling back to a minimal persona definition
//! 3. Creates a pod via `PodManagerBuilder` with ACP runtime and capability checker
//!    resolved from the same secret chain as the CLI (Follow-up #1: ACP integration)
//! 4. Constructs a rich system prompt from the agent definition's charter,
//!    responsibilities, rights, and voice/tone configuration (Follow-up #2:
//!    system prompt richness)
//! 5. Appends conversation history for context continuity (Follow-up #3:
//!    session persistence)
//! 6. Sends the user's message through pod-mediated inference via `InferencePort`
//! 7. Returns the LLM response and records it in the session history
//!
//! The replicant persona is configured via `HKASK_AGENT_PERSONA` env var.
//! The model is configured via `HKASK_DEFAULT_MODEL` env var or per-request override.
//!
//! # Session Persistence
//!
//! The server maintains an in-memory conversation history per session. Sessions
//! are identified by the caller's WebID. Each `replicant:chat` call appends the
//! user message and response to the history, and the next call includes the
//! recent history as context in the system prompt. This provides conversation
//! continuity across MCP tool invocations within the same server process.
//!
//! History is bounded to the last `MAX_HISTORY_TURNS` turns (default 20) to
//! manage token budget. The `replicant:history` tool exposes the current
//! session state.

use hkask_agents::acp::AcpRuntime;
use hkask_agents::pod::{AgentPersona, PodContext, PodManager, PodManagerBuilder};
use hkask_agents::ports::AcpPort;
use hkask_mcp::server::{McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_types::ports::InferencePort;
use hkask_types::{CapabilityChecker, LLMParameters, McpErrorKind, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent_loader::{load_agent_definition, resolve_acp_secret};
use crate::types::{ChatRequest, HistoryRequest, StatusRequest};

/// Maximum number of conversation turns to retain and include in context.
const MAX_HISTORY_TURNS: usize = 20;

/// A single conversation turn (user message + assistant response).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ConversationTurn {
    role: String,
    content: String,
}

// ── Session State ────────────────────────────────────────────────────────────

/// Per-session conversation history, keyed by the caller's WebID.
struct SessionState {
    turns: VecDeque<ConversationTurn>,
    acp_runtime: Arc<AcpRuntime>,
    agent_definition: Option<hkask_types::AgentDefinition>,
}

// ── Server ────────────────────────────────────────────────────────────────────

pub struct ReplicantServer {
    /// Calling agent's WebID (from run_stdio_server resolution)
    webid: WebID,
    /// Replicant persona name
    persona: String,
    /// Default model for inference
    default_model: String,
    /// Per-session conversation state
    session: Arc<RwLock<SessionState>>,
}

impl ReplicantServer {
    pub fn new(webid: WebID, persona: &str, default_model: &str) -> anyhow::Result<Self> {
        // Resolve ACP secret through the full derivation chain
        // (master key → env → keychain → deterministic default) so that the ACP runtime
        // is initialized with the same secret as the CLI and other MCP servers.
        // Per-replicant capability tokens are derived from the master secret via
        // AcpRuntime::derive_agent_secret(), so each replicant has its own signing context.
        let acp_secret = resolve_acp_secret();
        let acp_runtime = Arc::new(AcpRuntime::new(acp_secret.as_bytes()));

        // Follow-up #2: Try to load the full agent definition from the registry.
        // This provides charter, responsibilities, rights, and voice/tone for
        // rich system prompts. If the registry is not available, we fall back
        // to the minimal built-in persona.
        let agent_definition = load_agent_definition(persona);

        Ok(Self {
            webid,
            persona: persona.to_string(),
            default_model: default_model.to_string(),
            session: Arc::new(RwLock::new(SessionState {
                turns: VecDeque::new(),
                acp_runtime,
                agent_definition,
            })),
        })
    }

    /// Build an inference port for the given model using Okapi config from env.
    fn build_inference_port(&self, model: &str) -> Result<Arc<dyn InferencePort>, String> {
        let base_url = std::env::var("OKAPI_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
        let config = hkask_templates::OkapiConfig {
            base_url,
            ..hkask_templates::OkapiConfig::default()
        };
        hkask_templates::OkapiInference::new(model, config)
            .map(|i| Arc::new(i) as Arc<dyn InferencePort>)
            .map_err(|e| format!("Okapi init error: {}", e))
    }

    /// Compose the system prompt, using the full agent definition if available
    /// (Follow-up #2) or a minimal fallback otherwise.
    fn compose_system_prompt(&self) -> String {
        let state = self.session.blocking_read();
        match &state.agent_definition {
            Some(def) => def.compose_system_prompt(),
            None => format!(
                "You are {}, a Replicant in the hKask system.\n\n",
                self.persona
            ),
        }
    }

    /// Build conversation context from recent history (Follow-up #3).
    /// Returns a formatted string of recent turns to prepend to the prompt.
    fn format_history_prompt(&self) -> String {
        let state = self.session.blocking_read();
        if state.turns.is_empty() {
            return String::new();
        }
        let mut ctx = String::from("\n## Conversation History\n\n");
        for turn in &state.turns {
            ctx.push_str(&format!("{}: {}\n", turn.role, turn.content));
        }
        ctx.push('\n');
        ctx
    }

    /// Create a pod, activate it, and return the pod ID and context.
    async fn create_and_activate_pod(
        &self,
        pod_manager: &PodManager,
        persona: &AgentPersona,
    ) -> Result<(hkask_agents::pod::PodID, PodContext), String> {
        let pod_id = pod_manager
            .create_pod(
                "replicant-chat-template",
                persona,
                Some(self.persona.clone()),
            )
            .await
            .map_err(|e| format!("Pod creation error: {}", e))?;

        pod_manager
            .activate_pod(&pod_id)
            .await
            .map_err(|e| format!("Pod activation error: {}", e))?;

        let pod_context = PodContext::from_manager(pod_manager, &pod_id)
            .await
            .map_err(|e| format!("Pod context error: {}", e))?;

        Ok((pod_id, pod_context))
    }

    /// Record a single conversation turn and trim history to the maximum length.
    async fn record_turn(&self, role: &str, content: String) {
        let mut session = self.session.write().await;
        session.turns.push_back(ConversationTurn {
            role: role.to_string(),
            content,
        });
        while session.turns.len() > MAX_HISTORY_TURNS * 2 {
            session.turns.pop_front();
        }
    }

    /// Format an internal error response with the persona name.
    /// Consumes the span guard (every call site immediately returns this value).
    fn internal_error(&self, span: ToolSpanGuard, message: String) -> String {
        span.error(
            McpErrorKind::Internal,
            McpToolOutput::new(serde_json::json!({
                "error": message,
                "persona": self.persona,
            }))
            .to_json_string(),
        )
    }
}

// ── MCP Tool Handlers ─────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl ReplicantServer {
    #[tool(
        description = "Send a message to a hKask replicant agent and receive a response. The replicant persona is configured via HKASK_AGENT_PERSONA (default: 'Curator'). Optionally override the model per request. Conversation history is maintained across calls within the same session."
    )]
    async fn replicant_chat(
        &self,
        Parameters(ChatRequest { message, model }): Parameters<ChatRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("replicant:chat", &self.webid);

        if let Err(e) = validate_identifier("message", &message, 8192) {
            return span.error(e.kind, e.to_json_string());
        }

        let model = if model.is_empty() {
            self.default_model.clone()
        } else {
            if let Err(e) = validate_identifier("model", &model, 128) {
                return span.error(e.kind, e.to_json_string());
            }
            model
        };

        tracing::info!(
            target: "hkask.mcp.replicant",
            persona = %self.persona,
            model = %model,
            message_len = message.len(),
            "Replicant chat request"
        );

        // Build inference port
        let inference_port = match self.build_inference_port(&model) {
            Ok(port) => port,
            Err(e) => return self.internal_error(span, e),
        };

        // Build pod manager with ACP runtime and per-replicant capability checker.
        // The capability checker uses a key derived from the master secret specifically
        // for this replicant's WebID, so each replicant has its own signing context.
        let session = self.session.read().await;
        let acp_port: Arc<dyn AcpPort + Send + Sync> = session.acp_runtime.clone();
        let agent_secret = session.acp_runtime.derive_agent_secret(&self.webid).await;
        let capability_checker = CapabilityChecker::new(&agent_secret);

        let pod_manager = PodManagerBuilder::new()
            .acp_runtime(acp_port)
            .capability_checker(capability_checker)
            .inference_port(inference_port)
            .with_in_memory_storage()
            .build();
        drop(session); // Release read lock before write lock below

        // Follow-up #2: Use rich system prompt from agent definition when available.
        let persona_yaml = format!(
            r#"
agent:
  name: {}
  type: Replicant
  version: "0.1.0"
charter:
  description: "Chat session with {} via MCP"
  editor: mcp-server
capabilities:
  - "tool:inference:call"
rights: []
responsibilities: []
visibility:
  default: public
  episodic_override: private
"#,
            self.persona, self.persona
        );

        let persona = match AgentPersona::from_yaml(&persona_yaml) {
            Ok(p) => p,
            Err(e) => {
                return self.internal_error(span, format!("Persona parse error: {}", e));
            }
        };

        // Create and activate the pod
        let (_pod_id, pod_context) =
            match self.create_and_activate_pod(&pod_manager, &persona).await {
                Ok(result) => result,
                Err(e) => return self.internal_error(span, e),
            };

        // Follow-up #2: Compose the system prompt from the full agent definition
        let system_prompt = self.compose_system_prompt();

        // Follow-up #3: Include conversation history in the prompt for context continuity
        let history_prompt = self.format_history_prompt();

        let full_prompt = format!("{}{}User: {}", system_prompt, history_prompt, message);

        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
        };

        let pod_inference_port = match pod_context.inference_port() {
            Ok(port) => port,
            Err(e) => {
                return self.internal_error(span, format!("Inference port unavailable: {}", e));
            }
        };

        // Generate response using the pod's inference port with model override
        match pod_inference_port
            .generate_with_model(&full_prompt, &params, Some(&model))
            .await
        {
            Ok(result) => {
                // Follow-up #3: Record the turn in session history for context continuity
                self.record_turn("User", message).await;
                self.record_turn("Assistant", result.text.clone()).await;

                span.ok(McpToolOutput::new(serde_json::json!({
                    "text": result.text,
                    "model": result.model,
                    "persona": self.persona,
                    "usage": {
                        "prompt_tokens": result.usage.prompt_tokens,
                        "completion_tokens": result.usage.completion_tokens,
                        "total_tokens": result.usage.total_tokens,
                    },
                    "finish_reason": result.finish_reason,
                }))
                .to_json_string())
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolOutput::new(serde_json::json!({
                    "error": format!("Inference error: {}", e),
                    "persona": self.persona,
                    "model": model,
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Check the registration status and identity of the hKask replicant configured for this MCP server."
    )]
    async fn replicant_status(
        &self,
        Parameters(StatusRequest { persona }): Parameters<StatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("replicant:status", &self.webid);

        if !persona.is_empty()
            && let Err(e) = validate_identifier("persona", &persona, 128)
        {
            return span.error(e.kind, e.to_json_string());
        }

        let resolved_persona = if persona.is_empty() {
            &self.persona
        } else {
            &persona
        };

        let resolved_webid = WebID::from_persona(resolved_persona.as_bytes());

        let session = self.session.read().await;
        let has_definition = session.agent_definition.is_some();
        let history_turns = session.turns.len() / 2; // Each turn = user + assistant
        drop(session);

        span.ok(McpToolOutput::new(serde_json::json!({
            "persona": resolved_persona,
            "webid": resolved_webid.redacted_display().to_string(),
            "agent_type": "Replicant",
            "default_model": self.default_model,
            "server_webid": self.webid.redacted_display().to_string(),
            "has_registry_definition": has_definition,
            "session_history_turns": history_turns,
            "okapi_base_url": std::env::var("OKAPI_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string()),
        }))
        .to_json_string())
    }

    #[tool(
        description = "List recent conversation turns in the current session. Shows the last N turns of conversation history maintained across replicant:chat calls."
    )]
    async fn replicant_history(
        &self,
        Parameters(HistoryRequest { limit }): Parameters<HistoryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("replicant:history", &self.webid);

        let session = self.session.read().await;
        let total_turns = session.turns.len() / 2; // Each conversation turn = user + assistant
        let limit = limit.unwrap_or(total_turns);

        // Collect the most recent turns up to the limit
        let start = if session.turns.len() > limit * 2 {
            session.turns.len() - limit * 2
        } else {
            0
        };
        let history: Vec<&ConversationTurn> = session.turns.iter().skip(start).collect();

        let turns_json: Vec<serde_json::Value> = history
            .iter()
            .map(|turn| {
                serde_json::json!({
                    "role": turn.role,
                    "content": if turn.content.len() > 200 {
                        format!("{}…", &turn.content[..200])
                    } else {
                        turn.content.clone()
                    },
                })
            })
            .collect();

        span.ok(McpToolOutput::new(serde_json::json!({
            "persona": self.persona,
            "total_turns": total_turns,
            "showing": history.len() / 2,
            "history": turns_json,
        }))
        .to_json_string())
    }
}
