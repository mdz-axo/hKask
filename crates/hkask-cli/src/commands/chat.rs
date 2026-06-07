//! Chat command handlers — inference, memory, and Russell bridge
//!
//! The chat path uses direct inference via the shared `InferencePort`.
//! Pod creation is not needed for standard chat — pods are reserved for
//! multi-agent sessions (ensemble, A2A) where the pod lifecycle adds value.
//!
//! Memory integration (Task 5):
//! - Before inference: recall semantic triples relevant to the user's input
//! - After inference: store an episodic triple recording the exchange
//! - Both operations use the memory loop adapter with OCAP discipline

use hkask_agents::adapters::MemoryLoopAdapter;
use hkask_agents::ports::{
    EpisodicStoragePort, RecallRequest, SemanticStoragePort, StorageRequest,
};
use hkask_templates::{OkapiConfig, OkapiInference};
use hkask_types::ports::InferencePort;

use crate::repl::TOOL_CALL_FORMAT_INTRO;
use hkask_types::{
    Confidence, DelegationAction, DelegationResource, DelegationToken, LLMParameters, WebID,
};
use std::sync::Arc;

use crate::commands::config::{
    ResolvedSecrets, init_registry, init_registry_with_secrets, registry_yaml_path,
    resolve_acp_secret,
};

use hkask_types::ports::StructuredToolCall;

/// Response from a chat inference call.
///
/// Carries the response text, token usage, and structured tool calls
/// (from native function calling) alongside the finish reason so the
/// REPL can detect tool-call completions and route them appropriately.
pub struct ChatResponse {
    /// The agent's response text
    pub text: String,
    /// Token usage from the inference call (prompt + completion tokens)
    pub usage: Option<TokenUsage>,
    /// Why the model stopped generating ("stop", "tool_calls", etc.)
    pub finish_reason: String,
    /// Structured tool calls when the model supports native function calling.
    /// Empty when `finish_reason != "tool_calls"` — the REPL falls back to
    /// parsing `<<tool:...>>` text directives.
    pub tool_calls: Vec<StructuredToolCall>,
}

/// Token usage breakdown for gas accounting.
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Total tokens as gas cost. Uses a 1:1 mapping — one gas unit per token.
    /// This replaces the flat 500-unit heuristic with actual token counting.
    pub fn gas_cost(&self) -> u64 {
        self.total_tokens as u64
    }
}

/// Send a chat message to an agent and return the response.
///
/// Routes through Russell adapter for Russell requests, otherwise uses
/// direct inference via the shared `InferencePort`.
///
/// The chat path uses the `InferencePort` directly — pod creation is not
/// needed for standard chat. Pods are reserved for multi-agent sessions
/// (ensemble, A2A) where the pod lifecycle (registration, activation,
/// memory, capability tokens) adds value.
///
/// When `inference_port` is provided, the shared port is reused across calls
/// and `generate_with_model()` is used for per-request model override.
/// When `None`, a new `OkapiInference` is created per call (backward compat).
///
/// When `secrets` is provided (from onboarding), uses them directly instead
/// of re-resolving from environment/keychain — avoids the mock keyring
/// backend's EntryOnly persistence on Linux.
///
/// Memory integration:
/// - Before inference, recall semantic triples relevant to the user's input
/// - After inference, store an episodic triple recording the exchange
/// - Episodic memory is private (agent-scoped), semantic memory is public
///
/// When `episodic_storage` and `semantic_storage` are provided, they are used
/// for memory operations (enabling persistence across REPL sessions).
/// When `None`, a fresh in-memory adapter is created per call (ephemeral).
#[allow(clippy::too_many_arguments)]
pub async fn chat_with_agent(
    input: &str,
    agent_name: Option<&str>,
    model_override: Option<&str>,
    inference_port: Option<Arc<dyn InferencePort>>,
    secrets: Option<&ResolvedSecrets>,
    episodic_storage: Option<Arc<dyn EpisodicStoragePort>>,
    semantic_storage: Option<Arc<dyn SemanticStoragePort>>,
    agent_webid: Option<WebID>,
    system_prompt_suffix: Option<&str>,
    // Pre-formatted tool-call section of the system prompt. When `Some`, the
    // caller has already discovered the live tool list (typically via the
    // REPL's `governed_tool`) and passes the resulting string. When `None`,
    // the minimal hardcoded instruction is used.
    tool_section: Option<&str>,
) -> ChatResponse {
    let name = agent_name.unwrap_or("Curator");

    // Load agent registry — prefer pre-resolved secrets from onboarding
    let (acp, store) = match secrets {
        Some(s) => match init_registry_with_secrets(s).await {
            Ok(r) => r,
            Err(e) => {
                return ChatResponse {
                    text: format!("Registry init error: {}", e),
                    usage: None,
                    finish_reason: "error".to_string(),
                    tool_calls: vec![],
                };
            }
        },
        None => match init_registry().await {
            Ok(r) => r,
            Err(e) => {
                return ChatResponse {
                    text: format!("Registry init error: {}", e),
                    usage: None,
                    finish_reason: "error".to_string(),
                    tool_calls: vec![],
                };
            }
        },
    };

    let loader = hkask_agents::AgentRegistryLoader::new(
        registry_yaml_path(),
        acp.clone(),
        store,
        Arc::new(hkask_agents::adapters::FilesystemRegistrySource::new()),
    );

    let agents = match loader.boot().await {
        Ok(a) => a,
        Err(e) => {
            return ChatResponse {
                text: format!("Registry load error: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    let agent = agents.iter().find(|a| a.definition.name == name);

    // R11: Wire Russell Direct Chat
    if name == "russell" || name == "Russell" {
        return ChatResponse {
            text: chat_via_russell(input, agent).await,
            usage: None,
            finish_reason: "stop".to_string(),
            tool_calls: vec![],
        };
    }

    // Standard chat flow for non-Russell agents
    let mut system_prompt = match agent {
        Some(registered) => registered.definition.compose_system_prompt(),
        None => format!("You are {}, an assistant in the hKask system.\n\n", name),
    };

    // Append tool call format instructions so the model can invoke MCP tools.
    // When a tool_section is provided (discovered from running MCP servers),
    // use the dynamic section that lists available tools. Otherwise fall back
    // to the minimal hardcoded instruction for contexts without MCP runtime.
    if let Some(section) = tool_section {
        if !section.is_empty() {
            system_prompt.push_str(section);
        }
    } else {
        system_prompt.push_str(TOOL_CALL_FORMAT_INTRO);
        system_prompt.push_str(
            "For example, to recall semantic memory:\n\
             <<tool:hkask-mcp-semantic/semantic_recall\n\
             {\"entity\": \"rust\"}\n\
             >>\n\
             \n\
             You may include multiple tool calls in a single response. After the tool \
             executes, the system will feed the results back to you for a follow-up response.\n\
             Use tools when they would provide better or more current information than your training data."
        );
    }

    // Append HHH alignment suffix when active (Helpful, Harmless, Honest).
    // The suffix is passed from the REPL turn loop when HHH mode is active.
    if let Some(suffix) = system_prompt_suffix {
        system_prompt.push_str(suffix);
    }

    let agent_kind = match agent {
        Some(registered) => &registered.definition.agent_kind,
        None => {
            return ChatResponse {
                text: "Agent not registered \u{2014} run `kask agent register` first.".to_string(),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };
    let default_model = match agent_kind {
        hkask_types::AgentKind::Bot => "deepseek-v4-flash",
        hkask_types::AgentKind::Replicant => "deepseek-v4-pro",
    };
    let model = model_override.unwrap_or(default_model);

    // Use the shared inference port when available, otherwise create one
    let inference: Arc<dyn InferencePort> = match inference_port {
        Some(port) => port,
        None => {
            let config = OkapiConfig::local_dev();
            match OkapiInference::new(model, config) {
                Ok(i) => Arc::new(i) as Arc<dyn InferencePort>,
                Err(e) => {
                    return ChatResponse {
                        text: format!("Okapi init error: {}", e),
                        usage: None,
                        finish_reason: "error".to_string(),
                        tool_calls: vec![],
                    };
                }
            }
        }
    };

    // Derive WebID for the agent (deterministic — same name → same WebID)
    let agent_webid = agent_webid
        .unwrap_or_else(|| WebID::from_persona_with_namespace(name.as_bytes(), "replicant"));

    // Set up memory adapters for episodic storage and semantic recall.
    // Prefer persistent storage from the REPL (session-bound) when available.
    // Fall back to in-memory storage otherwise.
    let memory_adapter = match (&episodic_storage, &semantic_storage) {
        (Some(epi), Some(sem)) => {
            // Persistent storage from REPL session — use directly
            let _ = (epi, sem); // Used via the trait methods below
            None // Already have separate ports, no need for adapter
        }
        _ => {
            // No persistent storage — create ephemeral in-memory adapter
            match MemoryLoopAdapter::in_memory() {
                Ok(adapter) => Some(Arc::new(adapter)),
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.chat",
                        error = %e,
                        "Memory adapter init failed — chat will proceed without memory"
                    );
                    None
                }
            }
        }
    };

    // Create a capability token for memory operations.
    // The token uses the ACP secret for HMAC signing, ensuring that
    // memory operations are authorized through the same OCAP discipline
    // as pod-mediated access.
    let acp_secret = match resolve_acp_secret() {
        Ok(s) => s,
        Err(e) => {
            return ChatResponse {
                text: format!("ACP secret resolution failed: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    let capability_token = DelegationToken::new(
        DelegationResource::Registry,
        "memory".to_string(),
        DelegationAction::Execute,
        WebID::new(), // system
        agent_webid,
        acp_secret.as_bytes(),
    );

    // Recall relevant knowledge from semantic memory to enrich the prompt.
    let semantic_context = match (&semantic_storage, &memory_adapter) {
        (Some(sem_port), _) => {
            let request = RecallRequest::semantic(input, capability_token.clone());
            match sem_port.recall_semantic(&request) {
                Ok(triples) if !triples.is_empty() => {
                    let context: Vec<String> = triples
                        .iter()
                        .filter_map(|t| {
                            t.get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                    if context.is_empty() {
                        None
                    } else {
                        Some(context.join("\n"))
                    }
                }
                Ok(_) => None,
                Err(e) => {
                    tracing::debug!(
                        target: "hkask.chat.memory",
                        error = %e,
                        "Semantic recall failed — proceeding without context"
                    );
                    None
                }
            }
        }
        (None, Some(adapter)) => {
            let request = RecallRequest::semantic(input, capability_token.clone());
            match adapter.recall_semantic(&request) {
                Ok(triples) if !triples.is_empty() => {
                    let context: Vec<String> = triples
                        .iter()
                        .filter_map(|t| {
                            t.get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                    if context.is_empty() {
                        None
                    } else {
                        Some(context.join("\n"))
                    }
                }
                Ok(_) => None,
                Err(e) => {
                    tracing::debug!(
                        target: "hkask.chat.memory",
                        error = %e,
                        "Semantic recall failed — proceeding without context"
                    );
                    None
                }
            }
        }
        (None, None) => None,
    };

    // Compose the full prompt, incorporating any semantic context
    let full_prompt = match semantic_context {
        Some(ref ctx) => {
            format!(
                "{}\n\n## Relevant Knowledge\n{}\n\nUser: {}",
                system_prompt, ctx, input
            )
        }
        None => format!("{}\n\nUser: {}", system_prompt, input),
    };

    let params = LLMParameters {
        temperature: 0.7,
        top_p: 0.9,
        top_k: 40,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 512,
        seed: None,
    };

    // Direct inference — no pod creation needed for standard chat.
    // The InferenceResult carries structured tool_calls from native function
    // calling when the model returns finish_reason == "tool_calls".
    let (response_text, usage, finish_reason, tool_calls) = match inference
        .generate_with_model(&full_prompt, &params, Some(model))
        .await
    {
        Ok(result) => {
            let text = result.text;
            let u = result.usage;
            let fr = result.finish_reason;
            let tc = result.tool_calls;
            (
                text,
                Some(TokenUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
                fr,
                tc,
            )
        }
        Err(e) => {
            return ChatResponse {
                text: format!("Inference error: {}", e),
                usage: None,
                finish_reason: "error".to_string(),
                tool_calls: vec![],
            };
        }
    };

    // Store the exchange as an episodic triple: (agent, "chatted", response_text)
    // This is private, agent-scoped memory — episodic_override: private.
    let store_result = match (&episodic_storage, &memory_adapter) {
        (Some(epi_port), _) => {
            let request = StorageRequest::episodic(
                "chatted",
                "chat_turn",
                serde_json::json!({
                    "user_input": input,
                    "agent_response": response_text,
                }),
                Confidence::new(0.7),
                agent_webid,
            );
            epi_port.store_episodic(request, &capability_token)
        }
        (None, Some(adapter)) => {
            let request = StorageRequest::episodic(
                "chatted",
                "chat_turn",
                serde_json::json!({
                    "user_input": input,
                    "agent_response": response_text,
                }),
                Confidence::new(0.7),
                agent_webid,
            );
            adapter.store_episodic(request, &capability_token)
        }
        (None, None) => Err(hkask_agents::error::MemoryError::Infra(
            hkask_types::InfrastructureError::Io("No memory adapter available".into()),
        )),
    };

    match store_result {
        Ok(_) => {
            tracing::debug!(
                target: "hkask.chat.memory",
                agent = %name,
                "Episodic trace stored"
            );
        }
        Err(e) => {
            tracing::debug!(
                target: "hkask.chat.memory",
                agent = %name,
                error = %e,
                "Episodic storage failed — response still returned"
            );
        }
    }

    ChatResponse {
        text: response_text,
        usage,
        finish_reason,
        tool_calls,
    }
}

/// Chat via Russell ACP bridge (R11: Russell Direct Chat)
async fn chat_via_russell(input: &str, agent: Option<&hkask_types::RegisteredAgent>) -> String {
    use hkask_agents::acp::A2AMessage;
    use hkask_agents::adapters::RussellAcpAdapter;
    use hkask_agents::ports::AcpPort;

    if agent.is_none() {
        return "Russell is not registered. Use `kask agent register` to register Russell first."
            .to_string();
    }

    let russell_binary =
        std::env::var("HKASK_RUSSELL_BINARY").unwrap_or_else(|_| "russell-acp-server".to_string());

    let russell_adapter = match RussellAcpAdapter::new(russell_binary) {
        Ok(adapter) => adapter,
        Err(e) => return format!("Failed to initialize Russell bridge: {}", e),
    };

    let webid = WebID::from_persona_with_namespace(b"russell-chat-session", "russell");

    if let Err(e) = russell_adapter
        .register_agent(webid, hkask_types::AgentKind::Replicant, vec![])
        .await
    {
        return format!("Failed to create Russell session: {}", e);
    }

    let message = A2AMessage::TemplateDispatch {
        from: webid,
        to: Some(webid),
        template_id: "russell:direct-chat".to_string(),
        input: serde_json::json!({
            "message": input,
        }),
        correlation_id: uuid::Uuid::new_v4().to_string(),
    };

    match russell_adapter.send_message(message).await {
        Ok(response) => response,
        Err(e) => format!("Russell error: {}", e),
    }
}

/// CLI handler for `kask chat` subcommand
#[allow(clippy::too_many_arguments)]
pub fn run_chat(
    rt: &tokio::runtime::Runtime,
    registry: &hkask_templates::SqliteRegistry,
    runtime: &hkask_mcp::runtime::McpRuntime,
    handle: &tokio::runtime::Handle,
    template: Option<String>,
    input: Option<std::path::PathBuf>,
    agent: String,
    model: Option<String>,
) {
    if let Some(input_path) = input {
        // Non-interactive mode: run onboarding to ensure keys are configured.
        let onboarding_outcome = match rt.block_on(crate::onboarding::run_onboarding()) {
            Ok(outcome) => outcome,
            Err(e) => {
                eprintln!("Cannot chat: {}", e);
                eprintln!("Run `kask chat` first to complete onboarding interactively.");
                std::process::exit(1);
            }
        };
        let content = super::helpers::or_exit(
            std::fs::read_to_string(&input_path),
            "Failed to read input file",
        );
        let chat_response = rt.block_on(chat_with_agent(
            content.trim(),
            Some(&agent),
            model.as_deref(),
            None,
            onboarding_outcome.resolved_secrets.as_ref(),
            None, // No persistent storage in non-interactive mode
            None, // No persistent storage in non-interactive mode
            None, // WebID derived from agent name
            None, // No HHH suffix in non-interactive mode
            None, // No tool section — non-interactive mode has no MCP runtime
        ));
        println!("{}: {}", agent, chat_response.text);
        if let Some(ref usage) = chat_response.usage {
            eprintln!(
                "  {} tokens ({} prompt + {} completion)",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            );
        }
    } else {
        crate::repl::run(
            registry,
            runtime,
            template.as_deref(),
            &agent,
            model.as_deref(),
            handle.clone(),
        );
    }
}
