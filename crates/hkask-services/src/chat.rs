//! Chat service — unified inference, memory integration, and prompt composition.
//!
//! This is the deepest service module: it encapsulates the full chat turn
//! pipeline — agent lookup, system prompt assembly, semantic recall,
//! inference, episodic storage, and tool-call handling — so that both
//! CLI and API surfaces delegate to a single implementation rather than
//! duplicating ~400 lines of business logic.

use std::sync::Arc;

use hkask_agents::curator::persona_filter;
use hkask_agents::ports::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
use hkask_types::ports::{InferencePort, StructuredToolCall};
use hkask_types::{
    AuthContext, Confidence, DelegationAction, DelegationToken, LLMParameters, PersonaConstraints,
    WebID,
};

use crate::error::ServiceError;
use crate::{AgentService, InferenceContext, InferenceService};

/// Token usage breakdown for gas accounting.
#[derive(Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Total tokens as energy cost. Uses a 1:1 mapping — one gas unit per token.
    pub fn gas_cost(&self) -> u64 {
        self.total_tokens as u64
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use hkask_agents::ports::memory_storage::RecalledEpisode;
    use hkask_types::loops::episodic::ExperienceClassification;

    // REQ: MDS-chat-gas-001 — Token usage maps to gas cost at 1:1 ratio
    #[test]
    fn token_usage_gas_cost_one_to_one() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        assert_eq!(usage.gas_cost(), 150, "Gas cost must equal total_tokens");
    }

    // REQ: MDS-chat-gas-002 — Gas cost of zero tokens is zero
    #[test]
    fn token_usage_zero_tokens_zero_gas() {
        let usage = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        assert_eq!(usage.gas_cost(), 0);
    }

    // REQ: MDS-chat-gas-003 — Gas cost derived from total_tokens
    #[test]
    fn token_usage_gas_uses_total_not_sum_of_parts() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 250,
        };
        assert_eq!(usage.gas_cost(), 250);
        assert_ne!(usage.gas_cost(), 300);
    }

    fn test_token(from: WebID, to: WebID) -> DelegationToken {
        use hkask_types::capability::{DelegationAction, DelegationResource};
        DelegationToken::new(
            DelegationResource::Registry,
            "test".into(),
            DelegationAction::Execute,
            from,
            to,
            b"test-hmac-secret-32-bytes-long!!",
        )
    }

    struct MockSemanticPort {
        triples: Vec<RecalledSemantic>,
    }
    impl SemanticStoragePort for MockSemanticPort {
        fn store_semantic(
            &self,
            _: StorageRequest,
            _: &DelegationToken,
        ) -> Result<String, hkask_agents::error::MemoryError> {
            Ok("id".into())
        }
        fn recall_semantic(
            &self,
            _: &RecallRequest,
        ) -> Result<Vec<RecalledSemantic>, hkask_agents::error::MemoryError> {
            Ok(self.triples.clone())
        }
        fn semantic_storage_usage(
            &self,
            _: &str,
        ) -> Result<usize, hkask_agents::error::MemoryError> {
            Ok(self.triples.len())
        }
    }

    struct MockEpisodicPort {
        last_request: std::sync::Mutex<Option<StorageRequest>>,
    }
    impl EpisodicStoragePort for MockEpisodicPort {
        fn store_episodic(
            &self,
            r: StorageRequest,
            _: &DelegationToken,
        ) -> Result<String, hkask_agents::error::MemoryError> {
            *self.last_request.lock().unwrap() = Some(r);
            Ok("id".into())
        }
        fn recall_episodic(
            &self,
            _: &RecallRequest,
        ) -> Result<Vec<RecalledEpisode>, hkask_agents::error::MemoryError> {
            Ok(vec![])
        }
        fn episodic_storage_usage(
            &self,
            _: &WebID,
        ) -> Result<usize, hkask_agents::error::MemoryError> {
            Ok(0)
        }
        fn episodic_storage_budget(&self) -> usize {
            10_000
        }
        fn store_episodic_classified(
            &self,
            r: StorageRequest,
            _: ExperienceClassification,
            _: Option<Confidence>,
            t: &DelegationToken,
        ) -> Result<String, hkask_agents::error::MemoryError> {
            self.store_episodic(r, t)
        }
    }

    // REQ: MDS-chat-memory-001 — recall_semantic returns None when no triples match
    #[test]
    fn recall_semantic_empty_returns_none() {
        let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort { triples: vec![] });
        let port: Arc<dyn SemanticStoragePort> = mock;
        let w = WebID::new();
        let result = ChatService::recall_semantic(&port, "q", &test_token(w, w));
        assert!(result.is_none());
    }

    // REQ: MDS-chat-memory-002 — recall_semantic joins string values with newlines
    #[test]
    fn recall_semantic_joins_values_with_newlines() {
        let t = |s: &str| RecalledSemantic {
            id: "x".into(),
            entity: "doc".into(),
            attribute: "c".into(),
            value: serde_json::json!(s),
            confidence: Confidence::new(0.9),
            visibility: hkask_types::Visibility::Public,
            valid_from: "2026-01-01T00:00:00Z".into(),
        };
        let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort {
            triples: vec![t("A"), t("B")],
        });
        let port: Arc<dyn SemanticStoragePort> = mock;
        let w = WebID::new();
        let result = ChatService::recall_semantic(&port, "q", &test_token(w, w));
        assert_eq!(result, Some("A\nB".into()));
    }

    // REQ: MDS-chat-memory-003 — recall_semantic filters non-string values
    #[test]
    fn recall_semantic_filters_non_string_values() {
        let t1 = RecalledSemantic {
            id: "x".into(),
            entity: "doc".into(),
            attribute: "c".into(),
            value: serde_json::json!("Text"),
            confidence: Confidence::new(0.9),
            visibility: hkask_types::Visibility::Public,
            valid_from: "2026-01-01T00:00:00Z".into(),
        };
        let t2 = RecalledSemantic {
            id: "y".into(),
            entity: "doc".into(),
            attribute: "c".into(),
            value: serde_json::json!(42),
            confidence: Confidence::new(0.9),
            visibility: hkask_types::Visibility::Public,
            valid_from: "2026-01-01T00:00:00Z".into(),
        };
        let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort {
            triples: vec![t1, t2],
        });
        let port: Arc<dyn SemanticStoragePort> = mock;
        let w = WebID::new();
        let result = ChatService::recall_semantic(&port, "q", &test_token(w, w));
        assert_eq!(result, Some("Text".into()));
    }

    // REQ: MDS-chat-episodic-001 — store_episodic stores input+response as JSON
    #[test]
    fn store_episodic_records_chat_exchange() {
        let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
            last_request: std::sync::Mutex::new(None),
        });
        let port: Arc<dyn EpisodicStoragePort> = mock.clone();
        let w = WebID::from_persona(b"a");
        ChatService::store_episodic(&port, "Hello", "Hi!", w, &test_token(w, w), "Agent");
        let req = mock.last_request.lock().unwrap();
        let r = req.as_ref().unwrap();
        assert_eq!(r.entity, "chatted");
        assert_eq!(r.attribute, "chat_turn");
        assert_eq!(r.value["user_input"], "Hello");
        assert_eq!(r.value["agent_response"], "Hi!");
    }

    // REQ: MDS-chat-episodic-002 — store_episodic confidence 0.7
    #[test]
    fn store_episodic_uses_fixed_confidence() {
        let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
            last_request: std::sync::Mutex::new(None),
        });
        let port: Arc<dyn EpisodicStoragePort> = mock.clone();
        let w = WebID::from_persona(b"a");
        ChatService::store_episodic(&port, "in", "out", w, &test_token(w, w), "Agent");
        let req = mock.last_request.lock().unwrap();
        assert!((req.as_ref().unwrap().confidence.value() - 0.7).abs() < 0.001);
    }

    // REQ: MDS-chat-episodic-003 — store_episodic never panics
    #[test]
    fn store_episodic_never_panics() {
        let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
            last_request: std::sync::Mutex::new(None),
        });
        let port: Arc<dyn EpisodicStoragePort> = mock;
        let w = WebID::from_persona(b"t");
        ChatService::store_episodic(&port, "", "", w, &test_token(w, w), "");
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
    /// Pre-formatted tool-call section of the system prompt from MCP discovery
    pub tool_section: Option<String>,
    /// Override inference port — when provided, takes precedence over AgentService's shared port.
    /// The REPL uses this to pass its long-lived inference port.
    pub inference_port_override: Option<Arc<dyn InferencePort>>,
    /// Override episodic storage — when provided, takes precedence over AgentService's default.
    /// The REPL uses this to pass its per-agent persistent storage.
    pub episodic_storage_override: Option<Arc<dyn EpisodicStoragePort>>,
    /// Override semantic storage — when provided, takes precedence over AgentService's default.
    /// The REPL uses this to pass its per-agent persistent storage.
    pub semantic_storage_override: Option<Arc<dyn SemanticStoragePort>>,
    /// Verified authentication context from the caller. When provided, the service
    /// uses the caller's identity to derive operation-specific capability tokens
    /// instead of minting ad-hoc system-level tokens. API routes extract this from
    /// middleware-verified request extensions; CLI paths construct it from keystore secrets.
    pub auth_context: Option<AuthContext>,
    pub params_override: Option<LLMParameters>,
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
        ctx: &AgentService,
        req: &ChatRequest,
    ) -> Result<PreparedChat, ServiceError> {
        let name = req.agent_name.as_deref().unwrap_or("Curator");

        // Load agent registry to find the agent definition
        let loader = hkask_agents::AgentRegistryLoader::new(
            ctx.config().registry_yaml_path.clone(),
            ctx.acp_runtime().clone(),
            ctx.agent_registry_store().clone(),
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

        // Resolve inference port — prefer override, then shared port from AgentService
        let inference: Arc<dyn InferencePort> =
            match (&req.inference_port_override, ctx.inference_port()) {
                (Some(port), _) => Arc::clone(port),
                (None, Some(port)) => port,
                (None, None) => {
                    let inf_ctx = InferenceContext::from_parts(
                        None,
                        &model,
                        ctx.config().inference_config.clone(),
                    );
                    InferenceService::resolve_port(&inf_ctx, &model)?
                }
            };

        // Derive WebID for the agent
        let agent_webid = WebID::from_persona_with_namespace(name.as_bytes(), "replicant");

        // Create capability token for memory operations.
        let capability_token = ctx.capability_checker().grant_registry(
            DelegationAction::Execute,
            req.auth_context
                .as_ref()
                .map_or(*ctx.identity().0, |a| a.webid),
            agent_webid,
        );

        // Recall relevant knowledge from semantic memory
        let semantic_port: Arc<dyn SemanticStoragePort> = req
            .semantic_storage_override
            .clone()
            .unwrap_or_else(|| ctx.memory().1.clone());
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
            .unwrap_or_else(|| ctx.memory().0.clone());

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
    /// Uses `AgentService` for shared infrastructure (inference port,
    /// memory ports, ACP runtime, agent registry). When the context's
    /// inference_port is `None`, creates a fresh port via InferenceService.
    ///
    /// For streaming, use `prepare_chat()` + `generate_stream_with_model()`
    /// directly on the inference port.
    pub async fn chat(ctx: &AgentService, req: ChatRequest) -> Result<ChatResponse, ServiceError> {
        let prepared = Self::prepare_chat(ctx, &req).await?;
        // Access params_override after prepare_chat returns (prepare_chat only borrows req)
        let params_override = req.params_override;

        // Resolve LLM parameters: caller override > agent-kind defaults
        // REQ: P3 (Generative Space) — all parameters are user-exposed, none hidden.
        let params = params_override.unwrap_or(LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
        });

        // REQ: P9 (Homeostatic) — CNS span before inference
        tracing::debug!(target: "cns.chat.request", agent = %prepared.agent_name, model = %prepared.model, prompt_len = prepared.prompt.len());

        let result = prepared
            .inference_port
            .generate_with_model(&prepared.prompt, &params, Some(&prepared.model))
            .await
            .map_err(ServiceError::InferencePort)?;

        // REQ: P9 (Homeostatic) — CNS span after inference
        tracing::debug!(target: "cns.chat.response", agent = %prepared.agent_name, model = %prepared.model, tokens = result.usage.total_tokens, finish_reason = %result.finish_reason);

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

    /// Recall recent chat turns from episodic memory as pre-formatted context.
    ///
    /// Returns `None` if episodic storage is empty or recall fails.
    /// Each episode stores `user_input` + `agent_response` from `store_episodic()`.
    /// Formatted as "[Previous conversation]\nUser: ...\nAgent: ...\n[/Previous conversation]"
    ///
    /// # REQ: P2-session-history — every history access routes through episodic storage
    /// # REQ: P4-ocap-history — recall requires DelegationToken with Read on Manifest
    pub fn recall_recent_turns(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        agent_webid: &WebID,
        token: &DelegationToken,
        limit: usize,
    ) -> Option<String> {
        let request = RecallRequest::episodic("chatted", *agent_webid, token.clone());
        let episodes: Vec<RecalledEpisode> = match episodic_port.recall_episodic(&request) {
            Ok(v) if !v.is_empty() => v,
            _ => return None,
        };
        let recent: Vec<String> = episodes
            .iter()
            .rev()
            .take(limit)
            .filter_map(|e| {
                let v = e.value.as_object()?;
                let input = v.get("user_input")?.as_str()?;
                let response = v.get("agent_response")?.as_str()?;
                Some(format!("User: {}\nAgent: {}", input, response))
            })
            .collect();
        if recent.is_empty() {
            None
        } else {
            let formatted = recent.into_iter().rev().collect::<Vec<_>>().join("\n\n");
            Some(format!(
                "[Previous conversation]\n{}\n[/Previous conversation]\n\n",
                formatted
            ))
        }
    }

    /// Recall raw episodes (not formatted text) for condensation.
    ///
    /// Returns episodes as `Vec<(role, content)>` tuples suitable for
    /// passing to the condenser library's `thread_summary()`.
    /// Each episode yields one user message and one assistant message.
    pub fn recall_raw_episodes(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        agent_webid: &WebID,
        token: &DelegationToken,
        limit: usize,
    ) -> Vec<serde_json::Value> {
        let request = RecallRequest::episodic("chatted", *agent_webid, token.clone());
        let episodes: Vec<RecalledEpisode> = match episodic_port.recall_episodic(&request) {
            Ok(v) if !v.is_empty() => v,
            _ => return vec![],
        };
        let mut messages: Vec<serde_json::Value> = Vec::new();
        for e in episodes.iter().rev().take(limit) {
            if let Some(v) = e.value.as_object() {
                if let Some(input) = v.get("user_input").and_then(|s| s.as_str()) {
                    messages.push(serde_json::json!({"role": "user", "content": input}));
                }
                if let Some(response) = v.get("agent_response").and_then(|s| s.as_str()) {
                    messages.push(serde_json::json!({"role": "assistant", "content": response}));
                }
            }
        }
        messages.reverse();
        messages
    }

    /// Run a process manifest cascade for the agent, returning manifest-derived context.
    ///
    /// The manifest is a declarative pipeline (from `process_manifest` in the agent
    /// definition) that enriches the user input with context before inference.
    /// Returns `None` if the agent has no manifest or execution fails.
    pub async fn execute_manifest_cascade(
        executor: &hkask_templates::ManifestExecutor,
        manifest: &hkask_templates::BundleManifest,
        input: &str,
        agent_name: &str,
    ) -> Option<String> {
        let mut initial_ctx = std::collections::HashMap::new();
        initial_ctx.insert(
            "user_input".to_string(),
            serde_json::Value::String(input.to_string()),
        );
        initial_ctx.insert(
            "agent".to_string(),
            serde_json::Value::String(agent_name.to_string()),
        );

        let ctx = match executor.execute_manifest(manifest, initial_ctx).await {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::warn!(
                    target: "cns.spec.executor",
                    error = %e,
                    "Manifest cascade failed — continuing without manifest enrichment"
                );
                return None;
            }
        };

        let context_parts: Vec<String> = ctx
            .iter()
            .filter_map(|(key, value)| {
                if key.starts_with("step_") {
                    Some(format!("{}: {}", key, value))
                } else {
                    None
                }
            })
            .collect();

        if context_parts.is_empty() {
            None
        } else {
            tracing::info!(
                target: "cns.spec.executor",
                steps_completed = context_parts.len(),
                "Manifest cascade completed"
            );
            Some(context_parts.join("\n"))
        }
    }

    /// Wrap input with manifest context when a cascade completed successfully.
    pub fn wrap_manifest_input(input: &str, manifest_context: &str) -> String {
        format!(
            "[Manifest Context]\n{}\n[/Manifest Context]\n\n{}",
            manifest_context, input
        )
    }

    /// Apply persona constraints to filter forbidden patterns from a response.
    pub fn apply_persona_filter(
        response: &str,
        constraints: Option<&PersonaConstraints>,
    ) -> String {
        let Some(constraints) = constraints else {
            return response.to_string();
        };
        let (cleaned, violations) = persona_filter::strip_forbidden_patterns(response, constraints);
        if !violations.is_empty() {
            tracing::warn!(
                target: "cns.persona",
                violation_count = violations.len(),
                violations = ?violations.iter().map(|(p, _)| p).collect::<Vec<_>>(),
                "Persona constraint violations stripped from output"
            );
        }
        cleaned
    }

    /// Execute a full single-agent turn — manifest cascade, history suffix,
    /// inference via `ChatService::chat()`, and persona filter.
    ///
    /// Returns the final response text, token usage, and iteration count.
    /// The caller is responsible for gas governance (reserving/settling energy),
    /// streaming display, tool-call execution, and CNS update display.
    ///
    /// Tool-call handling: when the model returns structured tool calls,
    /// the response includes them in `structured_tool_calls`. The caller
    /// executes tools, formats results, and passes them as `tool_results`
    /// on the next iteration via a new `TurnRequest` (only `input`,
    /// `tool_results`, and iteration counter fields matter for continuations).
    pub async fn execute_turn(
        ctx: &AgentService,
        req: &TurnRequest,
        manifest_executor: Option<&hkask_templates::ManifestExecutor>,
        process_manifest: Option<&hkask_templates::BundleManifest>,
    ) -> Result<TurnResult, ServiceError> {
        // 1. Execute manifest cascade if the agent has a process manifest.
        let base_input =
            if let (Some(executor), Some(manifest)) = (manifest_executor, process_manifest) {
                let manifest_context =
                    Self::execute_manifest_cascade(executor, manifest, &req.input, &req.agent_name)
                        .await;
                match manifest_context {
                    Some(ctx) => Self::wrap_manifest_input(&req.input, &ctx),
                    None => req.input.clone(),
                }
            } else {
                req.input.clone()
            };

        // 2. Append recent conversation history from episodic memory.
        let token = req.capability_checker.grant_registry(
            DelegationAction::Read,
            req.system_webid,
            req.agent_webid,
        );
        let history_suffix = Self::recall_recent_turns(
            &req.episodic_storage,
            &req.agent_webid,
            &token,
            req.context_turns,
        );
        let input_with_context = match history_suffix {
            Some(s) => format!("{}\n\n{}", base_input, s),
            None => base_input,
        };

        // 3. Apply tool results from previous iterations (if any).
        let effective_input = if let Some(ref tool_results) = req.tool_results {
            format!(
                "{}\n\nThe following tool calls were executed:\n\n{}\n\nBased on these results, provide your response.",
                input_with_context.trim(),
                tool_results
            )
        } else {
            input_with_context
        };

        // 4. Execute inference via ChatService::chat().
        let chat_req = ChatRequest {
            input: effective_input,
            agent_name: Some(req.agent_name.clone()),
            model_override: Some(req.model.clone()),
            tool_section: if req.tool_section.is_empty() {
                None
            } else {
                Some(req.tool_section.clone())
            },
            inference_port_override: Some(req.inference_port.clone()),
            episodic_storage_override: Some(req.episodic_storage.clone()),
            semantic_storage_override: Some(req.semantic_storage.clone()),
            auth_context: None,
            params_override: Some(req.llm_params.clone()),
        };
        let chat_response = Self::chat(ctx, chat_req).await?;

        // 5. Persona filter.
        let filtered =
            Self::apply_persona_filter(&chat_response.text, req.persona_constraints.as_ref());

        Ok(TurnResult {
            text: filtered,
            usage: chat_response.usage.unwrap_or(TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }),
            iterations: req.iteration,
            finish_reason: chat_response.finish_reason,
            structured_tool_calls: chat_response.tool_calls,
        })
    }
}

/// Request for a single-agent turn through `ChatService::execute_turn()`.
pub struct TurnRequest {
    /// User input message
    pub input: String,
    /// Agent name for registry lookup and memory operations
    pub agent_name: String,
    /// Model name (e.g., "deepseek-v4-flash")
    pub model: String,
    /// Inference port override (REPL passes its long-lived port)
    pub inference_port: Arc<dyn InferencePort>,
    /// Episodic storage port (per-agent, for history and storage)
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    /// Semantic storage port (per-agent, for recall)
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    /// Agent WebID for memory operations
    pub agent_webid: WebID,
    /// Persona constraints for output filtering
    pub persona_constraints: Option<PersonaConstraints>,
    /// Pre-formatted tool section of the system prompt
    pub tool_section: String,
    /// LLM parameters from user settings
    pub llm_params: LLMParameters,
    /// Number of past turns to include in history suffix
    pub context_turns: usize,
    /// Capability checker for minting memory access tokens
    pub capability_checker: Arc<hkask_types::CapabilityChecker>,
    /// System WebID for token minting
    pub system_webid: WebID,
    /// Iteration counter (0 = first iteration, incremented by caller for continuations)
    pub iteration: usize,
    /// Tool execution results from the previous iteration (None on first iteration)
    pub tool_results: Option<String>,
    /// Whether to auto-condense conversation history when approaching context limits.
    /// When true and context exceeds 87.5% of `context_window`, the oldest half
    /// of messages are condensed via the condenser library.
    pub auto_condense: bool,
    /// Model context window size in tokens, used for condensation threshold.
    /// None disables condensation (e.g., model metadata not yet fetched).
    pub context_window: Option<u32>,
    /// Base URL for the inference engine used by the condenser (e.g., Ollama URL).
    pub condenser_base_url: Option<String>,
    /// Model to use for condenser summarization (defaults to chat model if None).
    pub condenser_model: Option<String>,
}

/// Result of a single-agent turn from `ChatService::execute_turn()`.
pub struct TurnResult {
    /// The final response text (after persona filtering)
    pub text: String,
    /// Token usage for this iteration
    pub usage: TokenUsage,
    /// Iteration count (as passed in TurnRequest.iteration)
    pub iterations: usize,
    /// Why the model stopped ("stop", "tool_calls", etc.)
    pub finish_reason: String,
    /// Structured tool calls when the model requests tools.
    /// Empty if finish_reason != "tool_calls".
    pub structured_tool_calls: Vec<StructuredToolCall>,
}
