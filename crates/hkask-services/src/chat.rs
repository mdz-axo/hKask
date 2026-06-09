//! Chat service — unified inference, memory integration, and prompt composition.
//!
//! This is the deepest service module: it encapsulates the full chat turn
//! pipeline — agent lookup, system prompt assembly, semantic recall,
//! inference, episodic storage, and tool-call handling — so that both
//! CLI and API surfaces delegate to a single implementation rather than
//! duplicating ~400 lines of business logic.

use std::sync::Arc;

use hkask_agents::ports::{
    EpisodicStoragePort, RecallRequest, RecalledSemantic, SemanticStoragePort, StorageRequest,
};
use hkask_types::ports::{InferencePort, StructuredToolCall};
use hkask_types::{
    AuthContext, Confidence, DelegationAction, DelegationToken, LLMParameters, WebID,
};

use crate::error::ServiceError;
use crate::{InferenceContext, InferenceService, ServiceContext};

/// Token usage breakdown for gas accounting.
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Total tokens as gas cost. Uses a 1:1 mapping — one gas unit per token.
    pub fn gas_cost(&self) -> u64 {
        self.total_tokens as u64
    }
}

/// Response from a chat inference call.
///
/// Carries the response text, token usage, and structured tool calls
/// (from native function calling) alongside the finish reason so the
/// calling surface can detect tool-call completions.
pub struct ChatResponse {
    /// The agent's response text
    pub text: String,
    /// Token usage from the inference call (prompt + completion tokens)
    pub usage: Option<TokenUsage>,
    /// Why the model stopped generating ("stop", "tool_calls", etc.)
    pub finish_reason: String,
    /// Structured tool calls when the model supports native function calling.
    pub tool_calls: Vec<StructuredToolCall>,
}

/// Request for a single chat turn.
///
/// Both CLI and API construct this from their surface-specific inputs,
/// then delegate to `ChatService::chat()`.
pub struct ChatRequest {
    /// User input message
    pub input: String,
    /// Agent name (defaults to "Curator")
    pub agent_name: Option<String>,
    /// Model override (defaults to agent-kind-specific model)
    pub model_override: Option<String>,
    /// HHH alignment suffix appended to the system prompt (when HHH mode is active)
    pub system_prompt_suffix: Option<String>,
    /// Pre-formatted tool-call section of the system prompt from MCP discovery
    pub tool_section: Option<String>,
    /// Override inference port — when provided, takes precedence over ServiceContext's shared port.
    /// The REPL uses this to pass its long-lived inference port.
    pub inference_port_override: Option<Arc<dyn InferencePort>>,
    /// Override episodic storage — when provided, takes precedence over ServiceContext's default.
    /// The REPL uses this to pass its per-agent persistent storage.
    pub episodic_storage_override: Option<Arc<dyn EpisodicStoragePort>>,
    /// Override semantic storage — when provided, takes precedence over ServiceContext's default.
    /// The REPL uses this to pass its per-agent persistent storage.
    pub semantic_storage_override: Option<Arc<dyn SemanticStoragePort>>,
    /// Verified authentication context from the caller. When provided, the service
    /// uses the caller's identity to derive operation-specific capability tokens
    /// instead of minting ad-hoc system-level tokens. API routes extract this from
    /// middleware-verified request extensions; CLI paths construct it from keystore secrets.
    pub auth_context: Option<AuthContext>,
}

/// Prepared chat context — the result of prompt composition before inference.
///
/// Returned by `ChatService::prepare_chat()` so that CLI/API surfaces can
/// stream inference output incrementally while still using the service layer
/// for agent lookup, prompt composition, and semantic recall.
pub struct PreparedChat {
    /// The full prompt ready for inference (system + semantic context + user input).
    pub prompt: String,
    /// The resolved model name.
    pub model: String,
    /// The agent's WebID (for episodic storage).
    pub agent_webid: WebID,
    /// Capability token for memory operations.
    pub capability_token: DelegationToken,
    /// The resolved inference port.
    pub inference_port: Arc<dyn InferencePort>,
    /// The resolved episodic storage port.
    pub episodic_port: Arc<dyn EpisodicStoragePort>,
    /// The agent name (for episodic storage).
    pub agent_name: String,
}

/// Chat service — encapsulates the full chat turn pipeline.
pub struct ChatService;

impl ChatService {
    /// Prepare a chat turn without executing inference.
    ///
    /// Does agent lookup, prompt composition, semantic recall,
    /// and resolves the inference port. Returns a `PreparedChat`
    /// that the caller can use to stream inference output.
    pub async fn prepare_chat(
        ctx: &ServiceContext,
        req: &ChatRequest,
    ) -> Result<PreparedChat, ServiceError> {
        let name = req.agent_name.as_deref().unwrap_or("Curator");

        // Load agent registry to find the agent definition
        let loader = hkask_agents::AgentRegistryLoader::new(
            ctx.config.registry_yaml_path.clone(),
            ctx.acp_runtime.clone(),
            ctx.agent_registry_store.clone(),
            Arc::new(hkask_agents::adapters::FilesystemRegistrySource::new()),
        );
        let agents = loader.boot().await.map_err(ServiceError::AgentRegistry)?;
        let agent = agents.iter().find(|a| a.definition.name == name);

        // Compose system prompt from agent definition
        let mut system_prompt = match agent {
            Some(registered) => registered.definition.compose_system_prompt(),
            None => format!("You are {}, an assistant in the hKask system.\n\n", name),
        };

        // Append tool-call format instructions
        if let Some(ref section) = req.tool_section
            && !section.is_empty()
        {
            system_prompt.push_str(section);
        }

        // Append HHH alignment suffix when active
        if let Some(ref suffix) = req.system_prompt_suffix {
            system_prompt.push_str(suffix);
        }

        // Determine agent kind and default model
        let agent_kind = match agent {
            Some(registered) => registered.definition.agent_kind,
            None => {
                return Err(ServiceError::AgentNotFound(
                    "Agent not registered — run `kask agent register` first.".to_string(),
                ));
            }
        };
        let default_model = match agent_kind {
            hkask_types::AgentKind::Bot => "deepseek-v4-flash",
            hkask_types::AgentKind::Replicant => "deepseek-v4-pro",
        };
        let model = req
            .model_override
            .as_deref()
            .unwrap_or(default_model)
            .to_string();

        // Resolve inference port — prefer override, then shared port from ServiceContext
        let inference: Arc<dyn InferencePort> =
            match (&req.inference_port_override, &ctx.inference_port) {
                (Some(port), _) => Arc::clone(port),
                (None, Some(port)) => Arc::clone(port),
                (None, None) => {
                    let inf_ctx =
                        InferenceContext::from_parts(None, &model, &ctx.config.okapi_base_url);
                    InferenceService::resolve_port(&inf_ctx, &model)
                        .map_err(|e| ServiceError::Inference(e.to_string()))?
                }
            };

        // Derive WebID for the agent
        let agent_webid = WebID::from_persona_with_namespace(name.as_bytes(), "replicant");

        // Create capability token for memory operations.
        let capability_token = ctx.capability_checker.grant_registry(
            DelegationAction::Execute,
            req.auth_context
                .as_ref()
                .map_or(ctx.system_webid, |a| a.webid),
            agent_webid,
        );

        // Recall relevant knowledge from semantic memory
        let semantic_port: Arc<dyn SemanticStoragePort> = req
            .semantic_storage_override
            .clone()
            .unwrap_or_else(|| ctx.semantic_storage.clone());
        let semantic_context = Self::recall_semantic(&semantic_port, &req.input, &capability_token);

        // Compose full prompt with semantic context
        let full_prompt = match semantic_context {
            Some(ref ctx_text) => {
                format!(
                    "{}\n\n## Relevant Knowledge\n{}\n\nUser: {}",
                    system_prompt, ctx_text, req.input
                )
            }
            None => format!("{}\n\nUser: {}", system_prompt, req.input),
        };

        // Resolve episodic storage port
        let episodic_port: Arc<dyn EpisodicStoragePort> = req
            .episodic_storage_override
            .clone()
            .unwrap_or_else(|| ctx.episodic_storage.clone());

        Ok(PreparedChat {
            prompt: full_prompt,
            model,
            agent_webid,
            capability_token,
            inference_port: inference,
            episodic_port,
            agent_name: name.to_string(),
        })
    }

    /// Execute a single chat turn: agent lookup → prompt composition →
    /// semantic recall → inference → episodic storage.
    ///
    /// Uses `ServiceContext` for shared infrastructure (inference port,
    /// memory ports, ACP runtime, agent registry). When the context's
    /// inference_port is `None`, creates a fresh port via InferenceService.
    ///
    /// For streaming, use `prepare_chat()` + `generate_stream_with_model()`
    /// directly on the inference port.
    pub async fn chat(
        ctx: &ServiceContext,
        req: ChatRequest,
    ) -> Result<ChatResponse, ServiceError> {
        let prepared = Self::prepare_chat(ctx, &req).await?;

        // Execute inference
        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
        };

        let result = prepared
            .inference_port
            .generate_with_model(&prepared.prompt, &params, Some(&prepared.model))
            .await
            .map_err(|e| ServiceError::Inference(e.to_string()))?;

        // Store the exchange as episodic triple
        Self::store_episodic(
            &prepared.episodic_port,
            &req.input,
            &result.text,
            prepared.agent_webid,
            &prepared.capability_token,
            &prepared.agent_name,
        );

        Ok(ChatResponse {
            text: result.text,
            usage: Some(TokenUsage {
                prompt_tokens: result.usage.prompt_tokens,
                completion_tokens: result.usage.completion_tokens,
                total_tokens: result.usage.total_tokens,
            }),
            finish_reason: result.finish_reason,
            tool_calls: result.tool_calls,
        })
    }

    /// Recall semantic memory triples relevant to the input.
    pub fn recall_semantic(
        semantic_port: &Arc<dyn SemanticStoragePort>,
        input: &str,
        token: &DelegationToken,
    ) -> Option<String> {
        let request = RecallRequest::semantic(input, token.clone());
        let triples = match semantic_port.recall_semantic(&request) {
            Ok(t) if !t.is_empty() => t,
            _ => return None,
        };

        let context: Vec<String> = triples
            .iter()
            .filter_map(|t: &RecalledSemantic| t.value.as_str().map(|s| s.to_string()))
            .collect();

        if context.is_empty() {
            None
        } else {
            Some(context.join("\n"))
        }
    }

    /// Store the chat exchange as an episodic triple.
    pub fn store_episodic(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        input: &str,
        response: &str,
        agent_webid: WebID,
        token: &DelegationToken,
        agent_name: &str,
    ) {
        let request = StorageRequest::episodic(
            "chatted",
            "chat_turn",
            serde_json::json!({
                "user_input": input,
                "agent_response": response,
            }),
            Confidence::new(0.7),
            agent_webid,
        );
        match episodic_port.store_episodic(request, token) {
            Ok(_) => {
                tracing::debug!(
                    target: "hkask.chat.memory",
                    agent = %agent_name,
                    "Episodic trace stored"
                );
            }
            Err(e) => {
                tracing::debug!(
                    target: "hkask.chat.memory",
                    agent = %agent_name,
                    error = %e,
                    "Episodic storage failed — response still returned"
                );
            }
        }
    }
}

