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
use hkask_agents::adapters::FilesystemRegistrySource;
use hkask_agents::pod::{AgentPersona, PodContext, PodManagerBuilder};
use hkask_agents::ports::{AcpPort, RegistrySourcePort};
use hkask_keystore;
use hkask_mcp::server::{McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_storage::Database;
use hkask_templates::{OkapiConfig, OkapiInference};
use hkask_types::ports::InferencePort;
use hkask_types::{CapabilityChecker, LLMParameters, McpErrorKind, SecretRef, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of conversation turns to retain and include in context.
const MAX_HISTORY_TURNS: usize = 20;

/// A single conversation turn (user message + assistant response).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ConversationTurn {
    role: String,
    content: String,
}

// ── Request Types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatRequest {
    /// The message to send to the replicant
    pub message: String,
    /// Model override (optional — uses the server default if empty)
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusRequest {
    /// Replicant persona name (optional — uses the server default if empty)
    #[serde(default)]
    pub persona: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HistoryRequest {
    /// Maximum number of turns to return (default: all)
    #[serde(default)]
    pub limit: Option<usize>,
}

// ── Session State ────────────────────────────────────────────────────────────

/// Per-session conversation history, keyed by the caller's WebID.
struct SessionState {
    turns: VecDeque<ConversationTurn>,
    acp_runtime: Arc<AcpRuntime>,
    acp_secret: String,
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
        // Follow-up #1: Resolve ACP secret through the full derivation chain
        // (master key → env → keychain → insecure dev) so that the ACP runtime
        // is initialized with the same secret as the CLI and other MCP servers.
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
                acp_secret,
                agent_definition,
            })),
        })
    }

    /// Build an inference port for the given model using Okapi config from env.
    fn build_inference_port(&self, model: &str) -> Result<Arc<dyn InferencePort>, String> {
        let base_url = std::env::var("OKAPI_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
        let config = OkapiConfig {
            base_url,
            ..OkapiConfig::default()
        };
        OkapiInference::new(model, config)
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
}

// ── ACP Secret Resolution ────────────────────────────────────────────────────
// Follow-up #1: Full ACP secret resolution chain matching the CLI's
// `resolve_acp_secret()`. This ensures the MCP server's ACP runtime uses
// the same secret as `kask chat`, so capability tokens are compatible.

fn resolve_acp_secret() -> String {
    // 1. Master key derivation (HKDF-SHA256)
    hkask_keystore::resolve(&SecretRef::derived(
        hkask_types::derivation_contexts::MASTER_KEY_ENV,
        hkask_types::derivation_contexts::ACP_SECRET,
    ))
    .map(|s| String::from_utf8_lossy(&s).to_string())
    // 2. Direct environment variable
    .or_else(|_| std::env::var("HKASK_ACP_SECRET"))
    // 3. OS keychain
    .or_else(|_| {
        hkask_keystore::Keychain::default()
            .retrieve_by_key("acp-secret")
            .map_err(|e| e.to_string())
    })
    // 4. Insecure dev mode (random secret, tokens won't survive restarts)
    .or_else(|_| {
        if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1") {
            tracing::warn!(
                target: "hkask.mcp.replicant",
                "⚠ INSECURE DEV MODE: Using random ACP secret. Tokens will not survive restarts."
            );
            use std::fmt::Write;
            let mut bytes = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
            let mut s = String::with_capacity(64);
            for b in &bytes {
                write!(s, "{b:02x}").unwrap();
            }
            Ok(s)
        } else {
            // Fall back to a deterministic default so the server can still start.
            // The CLI resolves this through onboarding; MCP servers may be started
            // independently and need a working default.
            tracing::warn!(
                target: "hkask.mcp.replicant",
                "No ACP secret resolved — using deterministic default. \
                 Set HKASK_ACP_SECRET, HKASK_MASTER_KEY, or HKASK_INSECURE_DEV=1 for proper token verification."
            );
            Ok("hkask-default-acp-secret-for-mcp-server".to_string())
        }
    })
    .unwrap_or_else(|_: String| "hkask-default-acp-secret-for-mcp-server".to_string())
}

// ── Agent Definition Loading ─────────────────────────────────────────────────
// Follow-up #2: Load the full agent definition from the YAML registry.
// This provides charter, responsibilities, rights, and voice/tone for
// rich system prompts. Falls back to None if the registry is unavailable.

fn load_agent_definition(persona: &str) -> Option<hkask_types::AgentDefinition> {
    let registry_path =
        std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string());

    let db_path = std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string());

    // Try to open the registry database. If it doesn't exist or we can't
    // read it, we fall back to the minimal persona definition.
    let passphrase = std::env::var("HKASK_DB_PASSPHRASE")
        .or_else(|_| {
            hkask_keystore::Keychain::default()
                .retrieve_by_key("hkask-db-passphrase")
                .map_err(|e| e.to_string())
        })
        .or_else(|_: String| {
            // In insecure dev mode, use a placeholder passphrase
            if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1") {
                Ok::<String, String>("insecure-dev-passphrase".to_string())
            } else {
                // Try empty passphrase for unencrypted databases
                Ok::<String, String>(String::new())
            }
        })
        .unwrap_or_default();

    let db = match Database::open(&db_path, &passphrase) {
        Ok(db) => db,
        Err(e) => {
            tracing::debug!(
                target: "hkask.mcp.replicant",
                error = %e,
                "Registry database not available, using minimal persona for '{}'",
                persona
            );
            return None;
        }
    };

    let store = hkask_storage::AgentRegistryStore::new(db.conn_arc());
    if let Err(e) = store.initialize_schema() {
        tracing::debug!(
            target: "hkask.mcp.replicant",
            error = %e,
            "Schema init failed, using minimal persona for '{}'",
            persona
        );
        return None;
    }

    match store.get(persona) {
        Ok(agent) => {
            tracing::info!(
                target: "hkask.mcp.replicant",
                persona = %persona,
                "Loaded full agent definition from registry"
            );
            Some(agent.definition)
        }
        Err(_) => {
            // Not found in the database — try loading from YAML files
            // via the registry loader as a secondary path.
            tracing::debug!(
                target: "hkask.mcp.replicant",
                persona = %persona,
                "Agent '{}' not found in database, attempting YAML discovery",
                persona
            );
            load_definition_from_yaml(persona, &registry_path)
        }
    }
}

fn load_definition_from_yaml(
    persona: &str,
    registry_path: &str,
) -> Option<hkask_types::AgentDefinition> {
    // The agent name is used as filename: registry/bots/{name}.yaml
    let yaml_path = format!("{}/{}.yaml", registry_path, persona.to_lowercase());
    let yaml_path_alt = format!("{}/{}.yml", registry_path, persona.to_lowercase());

    let source = FilesystemRegistrySource::new();
    let content = source
        .load_yaml(&yaml_path)
        .or_else(|_| source.load_yaml(&yaml_path_alt))
        .ok()?;

    // Parse the raw YAML to extract the agent definition
    let raw: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let agent_section = raw.get("agent")?;

    let name = agent_section.get("name")?.as_str()?.to_string();
    let agent_type = agent_section
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("Replicant");
    let agent_kind = match agent_type {
        "Replicant" | "replicant" => hkask_types::AgentKind::Replicant,
        _ => hkask_types::AgentKind::Bot,
    };

    let mut def = hkask_types::AgentDefinition {
        name,
        agent_kind,
        charter: None,
        capabilities: vec![],
        rights: vec![],
        responsibilities: vec![],
        persona: None,
        depends_on: vec![],
        process_manifest: None,
    };

    // Charter
    if let Some(charter) = raw
        .get("charter")
        .and_then(|c| c.get("description"))
        .and_then(|d| d.as_str())
    {
        def.charter = Some(hkask_types::Charter {
            description: charter.to_string(),
            archetype: raw
                .get("charter")
                .and_then(|c| c.get("archetype"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            visibility: raw
                .get("charter")
                .and_then(|c| c.get("visibility"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    // Capabilities
    if let Some(caps) = raw.get("capabilities").and_then(|c| c.as_sequence()) {
        def.capabilities = caps
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    // Persona (tone, verbosity, forbidden, required)
    if let Some(persona_section) = raw.get("persona") {
        def.persona = Some(hkask_types::PersonaConstraints {
            tone: persona_section
                .get("tone")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            verbosity: persona_section
                .get("verbosity")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            formatting: persona_section
                .get("formatting")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            forbidden: persona_section
                .get("forbidden")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            required: persona_section
                .get("required")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    tracing::info!(
        target: "hkask.mcp.replicant",
        persona = %persona,
        "Loaded agent definition from YAML file"
    );
    Some(def)
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
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": e,
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Follow-up #1: Build pod manager with a properly-initialized ACP runtime
        // and capability checker using the same secret derivation chain as the CLI.
        // This ensures capability tokens are compatible across all hKask surfaces
        // (CLI, API, MCP).
        let session = self.session.read().await;
        let acp_port: Arc<dyn AcpPort + Send + Sync> = session.acp_runtime.clone();

        let pod_manager = PodManagerBuilder::new()
            .acp_runtime(acp_port)
            .capability_checker(CapabilityChecker::new(session.acp_secret.as_bytes()))
            .inference_port(inference_port)
            .with_in_memory_storage()
            .build();
        drop(session); // Release read lock before write lock below

        // Follow-up #2: Use rich system prompt from agent definition when available.
        // The persona YAML still defines the pod's structural identity (name, type,
        // capabilities), while the system prompt carries charter, voice, rights,
        // and responsibilities from the full agent definition.
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
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Persona parse error: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Create and activate the pod
        let pod_id = match pod_manager
            .create_pod(
                "replicant-chat-template",
                &persona,
                Some(self.persona.clone()),
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Pod creation error: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        if let Err(e) = pod_manager.activate_pod(&pod_id).await {
            return span.error(
                McpErrorKind::Internal,
                McpToolOutput::new(serde_json::json!({
                    "error": format!("Pod activation error: {}", e),
                    "persona": self.persona,
                }))
                .to_json_string(),
            );
        }

        let pod_context = match PodContext::from_manager(&pod_manager, &pod_id).await {
            Ok(ctx) => ctx,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Pod context error: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Follow-up #2: Compose the system prompt from the full agent definition
        // (charter, responsibilities, rights, voice/tone) or fall back to minimal.
        let system_prompt = self.compose_system_prompt();

        // Follow-up #3: Include conversation history in the prompt for context
        // continuity across MCP tool invocations.
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
                return span.error(
                    McpErrorKind::Internal,
                    McpToolOutput::new(serde_json::json!({
                        "error": format!("Inference port unavailable: {}", e),
                        "persona": self.persona,
                    }))
                    .to_json_string(),
                );
            }
        };

        // Generate response using the pod's inference port with model override
        match pod_inference_port
            .generate_with_model(&full_prompt, &params, Some(&model))
            .await
        {
            Ok(result) => {
                // Follow-up #3: Record the turn in session history for context
                // continuity across subsequent calls.
                {
                    let mut session = self.session.write().await;
                    session.turns.push_back(ConversationTurn {
                        role: "User".to_string(),
                        content: message.clone(),
                    });
                    session.turns.push_back(ConversationTurn {
                        role: "Assistant".to_string(),
                        content: result.text.clone(),
                    });
                    // Trim to MAX_HISTORY_TURNS, keeping the most recent
                    while session.turns.len() > MAX_HISTORY_TURNS * 2 {
                        session.turns.pop_front();
                    }
                }

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
