//! Chat service — unified inference, memory integration, and prompt composition.
//!
//! This is the deepest service module: it encapsulates the full chat turn
//! pipeline — agent lookup, system prompt assembly, semantic recall,
//! inference, episodic storage, and tool-call handling — so that both
//! CLI and API surfaces delegate to a single implementation rather than
//! duplicating ~400 lines of business logic.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use hkask_agents::curator::persona_filter;
use hkask_agents::ports::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
use hkask_capability::{AuthContext, DelegationAction, DelegationToken};
use hkask_ports::{ChatToolDefinition, InferencePort, StructuredToolCall};
use hkask_types::PersonaConstraints;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::template::LLMParameters;
use hkask_types::{Confidence, DataCategory, WebID};

use crate::ServiceError;
use crate::{InferenceContext, InferenceService, MemoryService};
use hkask_services_context::AgentService;

/// System prompt for the auto-condense summarization request.
const CONDENSER_SYSTEM_PROMPT: &str = "You are a context condensation assistant. Produce structured summaries that \
     preserve technical details (file paths, error messages, decisions) while \
     eliminating verbosity. Use bullet points. Be concise.";

/// Token usage breakdown for gas accounting.
#[derive(Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Total tokens as energy cost. Uses a 1:1 mapping — one gas unit per token.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self.total_tokens must be set
    /// post: returns total_tokens as u64 gas cost
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
    use hkask_cns::types::loops::episodic::ExperienceClassification;

    #[test]
    fn token_usage_gas_cost_one_to_one() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        assert_eq!(usage.gas_cost(), 150, "Gas cost must equal total_tokens");
    }

    #[test]
    fn token_usage_zero_tokens_zero_gas() {
        let usage = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        assert_eq!(usage.gas_cost(), 0);
    }

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
        use hkask_capability::{DelegationAction, DelegationResource, derive_signing_key};
        DelegationToken::new(
            DelegationResource::Registry,
            "test".into(),
            DelegationAction::Execute,
            from,
            to,
            &derive_signing_key(b"test-hmac-secret-32-bytes-long!!"),
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

    #[test]
    fn recall_semantic_empty_returns_none() {
        let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort { triples: vec![] });
        let port: Arc<dyn SemanticStoragePort> = mock;
        let w = WebID::new();
        let result = MemoryService::recall_semantic(&port, "q", &test_token(w, w));
        assert!(result.is_none());
    }

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
        let result = MemoryService::recall_semantic(&port, "q", &test_token(w, w));
        assert_eq!(result, Some("A\nB".into()));
    }

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
        let result = MemoryService::recall_semantic(&port, "q", &test_token(w, w));
        assert_eq!(result, Some("Text".into()));
    }

    #[test]
    fn store_episodic_records_chat_exchange() {
        let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
            last_request: std::sync::Mutex::new(None),
        });
        let port: Arc<dyn EpisodicStoragePort> = mock.clone();
        let w = WebID::from_persona(b"a");
        MemoryService::store_episodic(&port, "Hello", "Hi!", w, &test_token(w, w), "Agent");
        let req = mock.last_request.lock().unwrap();
        let r = req.as_ref().unwrap();
        assert_eq!(r.entity, "chatted");
        assert_eq!(r.attribute, "chat_turn");
        assert_eq!(r.value["user_input"], "Hello");
        assert_eq!(r.value["agent_response"], "Hi!");
    }

    #[test]
    fn store_episodic_uses_fixed_confidence() {
        let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
            last_request: std::sync::Mutex::new(None),
        });
        let port: Arc<dyn EpisodicStoragePort> = mock.clone();
        let w = WebID::from_persona(b"a");
        MemoryService::store_episodic(&port, "in", "out", w, &test_token(w, w), "Agent");
        let req = mock.last_request.lock().unwrap();
        assert!((req.as_ref().unwrap().confidence.value() - 0.7).abs() < 0.001);
    }

    #[test]
    fn store_episodic_never_panics() {
        let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
            last_request: std::sync::Mutex::new(None),
        });
        let port: Arc<dyn EpisodicStoragePort> = mock;
        let w = WebID::from_persona(b"t");
        MemoryService::store_episodic(&port, "", "", w, &test_token(w, w), "");
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
    /// OpenAI-compatible tool definitions for native function calling.
    /// When present, tools are included in the inference request so the model
    /// can return structured tool calls via `finish_reason == "tool_calls"`.
    /// The REPL passes these from `state.tool_definitions` during turn processing
    /// including the `/ask` handler which routes through `single_agent_turn`.
    pub tools: Option<Vec<ChatToolDefinition>>,
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx must be fully built; req.input must be non-empty; agent must be registered
    /// post: returns PreparedChat with prompt, model, agent_webid, capability_token, inference_port, episodic_port, and agent_name; Err(AgentNotFound) if agent not registered
    pub async fn prepare_chat(
        ctx: &AgentService,
        req: &ChatRequest,
    ) -> Result<PreparedChat, ServiceError> {
        let name = req.agent_name.as_deref().unwrap_or("Curator");

        // Load agent registry to find the agent definition
        let loader = hkask_agents::AgentRegistryLoader::new(
            ctx.config().registry_yaml_path.clone(),
            ctx.a2a_runtime().clone(),
            ctx.agent_registry_store().clone(),
            Arc::new(hkask_agents::adapters::FilesystemRegistrySource::new()),
        );
        let agents = loader
            .boot()
            .await
            .map_err(|e| ServiceError::AgentRegistry {
                message: e.to_string(),
            })?;
        let agent = agents.iter().find(|a| a.definition.name == name);

        // Compose system prompt from agent definition
        let mut system_prompt = match agent {
            Some(registered) => format!(
                "You are {}, a {} in the hKask system.\n\n",
                registered.definition.name, registered.definition.agent_kind
            ),
            None => format!("You are {}, an assistant in the hKask system.\n\n", name),
        };

        // Append tool-call format instructions
        if let Some(ref section) = req.tool_section
            && !section.is_empty()
        {
            system_prompt.push_str(section);
        }

        // Determine agent kind (used for capability routing, not model selection)
        let _agent_kind = match agent {
            Some(registered) => registered.definition.agent_kind,
            None => {
                return Err(ServiceError::AgentNotFound {
                    source: None,
                    message: "Agent not registered — run `kask agent register` first.".to_string(),
                });
            }
        };
        // Model flows from request override → config default.
        // Agent kind no longer hardcodes model selection — use session/replicant settings.
        let model = req
            .model_override
            .as_deref()
            .unwrap_or(&ctx.config().default_model)
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

        // Recall relevant knowledge from semantic memory (third-person facts)
        let semantic_port: Arc<dyn SemanticStoragePort> = req
            .semantic_storage_override
            .clone()
            .unwrap_or_else(|| ctx.memory().1.clone());
        let episodic_port: Arc<dyn EpisodicStoragePort> = req
            .episodic_storage_override
            .clone()
            .unwrap_or_else(|| ctx.memory().0.clone());

        // \[NORMATIVE\] Sovereignty gate (H3/P2): only recall memory when
        // the owner has granted consent for the category. No consent ⇒ no recall.
        let semantic_context = if MemoryService::has_memory_consent(
            ctx,
            &agent_webid,
            &DataCategory::SemanticMemory,
        ) {
            MemoryService::recall_semantic(&semantic_port, &req.input, &capability_token)
        } else {
            None
        };
        let episodic_context = if MemoryService::has_memory_consent(
            ctx,
            &agent_webid,
            &DataCategory::EpisodicMemory,
        ) {
            MemoryService::recall_episodic(
                &episodic_port,
                &req.input,
                &agent_webid,
                &capability_token,
            )
        } else {
            None
        };

        // Merge semantic (third-person) and episodic (first-person) memory.
        // Both are recall_* results that mirror each other in structure —
        // concatenated into a single "Relevant Memory" section sorted by salience.
        let memory_context = match (semantic_context, episodic_context) {
            (Some(s), Some(e)) => Some(format!("{}\n\n{}", s, e)),
            (Some(s), None) => Some(s),
            (None, Some(e)) => Some(e),
            (None, None) => None,
        };

        // Compose full prompt with merged memory context
        let full_prompt = match memory_context {
            Some(ref ctx_text) => {
                format!(
                    "{}\n\n## Relevant Memory\n{}\n\nUser: {}",
                    system_prompt, ctx_text, req.input
                )
            }
            None => format!("{}\n\nUser: {}", system_prompt, req.input),
        };

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
    /// memory ports, A2A runtime, agent registry). When the context's
    /// inference_port is `None`, creates a fresh port via InferenceService.
    ///
    /// For streaming, use `prepare_chat()` + `generate_stream_with_model()`
    /// directly on the inference port.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx must be fully built; req.input must be non-empty
    /// post: returns ChatResponse with text, usage, finish_reason, and tool_calls; CNS spans emitted; episodic trace stored; Err on agent lookup or inference failure
    pub async fn chat(ctx: &AgentService, req: ChatRequest) -> Result<ChatResponse, ServiceError> {
        let prepared = Self::prepare_chat(ctx, &req).await?;
        // Access params_override after prepare_chat returns (prepare_chat only borrows req)
        let params_override = req.params_override;

        // Resolve LLM parameters: caller override > agent-kind defaults.
        // When fusion is active, the primary chat model bypasses fusion so
        // the user's chosen model is used directly. Skills route through fusion.
        let chat_bypass = ctx.config().inference_config.fusion.is_some();
        let mut params = params_override.unwrap_or(LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
            disable_thinking: false,
            adapter: None,
            bypass_fusion: chat_bypass,
        });
        // Always enforce the service-layer bypass decision — a caller's
        // params_override must not silently defeat the cost-safety guard.
        params.bypass_fusion = chat_bypass;

        let request_span = Span::new(SpanNamespace::from(CnsSpan::Chat), "request");
        let request_event = NuEvent::new(
            prepared.agent_webid,
            request_span,
            Phase::Act,
            serde_json::json!({
                "agent": &prepared.agent_name,
                "model": &prepared.model,
                "prompt_len": prepared.prompt.len(),
            }),
            0,
        );
        let _ = ctx.event_sink().persist(&request_event);

        let result = prepared
            .inference_port
            .generate_with_model(
                &prepared.prompt,
                &params,
                Some(&prepared.model),
                req.tools.as_deref(),
            )
            .await
            .map_err(|e| ServiceError::InferencePort {
                message: e.to_string(),
                retryable: false,
            })?;

        let response_span = Span::new(SpanNamespace::from(CnsSpan::Chat), "response");
        let response_event = NuEvent::new(
            prepared.agent_webid,
            response_span,
            Phase::Act,
            serde_json::json!({
                "agent": &prepared.agent_name,
                "model": &prepared.model,
                "tokens": result.usage.total_tokens,
                "finish_reason": &result.finish_reason,
            }),
            0,
        )
        .with_parent(request_event.id);
        let _ = ctx.event_sink().persist(&response_event);

        // Store the exchange as episodic triple (with CNS observability)
        let memory_span = Span::new(
            SpanNamespace::from(CnsSpan::MemoryEncode),
            "episodic_stored",
        );
        let memory_event = NuEvent::new(
            prepared.agent_webid,
            memory_span,
            Phase::Act,
            serde_json::json!({
                "agent": &prepared.agent_name,
                "operation": "store_episodic",
                "input_len": req.input.len(),
                "response_len": result.text.len(),
            }),
            0,
        );
        let _ = ctx.event_sink().persist(&memory_event);

        // \[NORMATIVE\] Sovereignty gate (H3/P2): only persist the exchange to
        // episodic (sovereign) memory when the owner has granted consent.
        if MemoryService::has_memory_consent(
            ctx,
            &prepared.agent_webid,
            &DataCategory::EpisodicMemory,
        ) {
            MemoryService::store_episodic(
                &prepared.episodic_port,
                &req.input,
                &result.text,
                prepared.agent_webid,
                &prepared.capability_token,
                &prepared.agent_name,
            );
        } else {
            tracing::debug!(
                target: "hkask.chat.memory",
                agent = %prepared.agent_name,
                "Episodic store skipped — no episodic-memory consent (P2)"
            );
        }

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

    /// Sovereignty gate for chat-path memory access (H3).
    ///
    /// The chat orchestration path operates with raw storage ports rather than a
    /// `PodContext`, so it must apply the same consent gate that
    /// Recall semantic memory triples relevant to the input.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  semantic_port must be initialized; input must be non-empty; token must be valid
    /// post: returns Some(String) of concatenated triple values if matches found; None if no matches or recall fails
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

        // Only return the most recent context to bound prompt size
        let context: Vec<String> = context.into_iter().take(10).collect();

        if context.is_empty() {
            None
        } else {
            Some(context.join("\n"))
        }
    }

    /// Recall episodic memories relevant to the input, sorted by salience.
    ///
    /// Mirrors `recall_semantic`: both return `Option<String>` of concatenated
    /// memory values, both take top N results, both are called together in
    /// `prepare_chat` and merged before injection into context.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  episodic_port must be initialized; input must be non-empty; agent_webid must be valid; token must be valid
    /// post: returns Some(String) of formatted episodes sorted by relevance to input; None if no episodes or recall fails
    pub fn recall_episodic(
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        input: &str,
        agent_webid: &WebID,
        token: &DelegationToken,
    ) -> Option<String> {
        let request = RecallRequest::episodic("chatted", *agent_webid, token.clone());
        let episodes: Vec<RecalledEpisode> = match episodic_port.recall_episodic(&request) {
            Ok(v) if !v.is_empty() => v,
            _ => return None,
        };

        // Build scored entries: (salience_score, formatted_text)
        let input_lower = input.to_lowercase();
        let keywords: Vec<&str> = input_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        let mut scored: Vec<(usize, String)> = episodes
            .iter()
            .filter_map(|e| {
                let v = e.value.as_object()?;
                let ui = v.get("user_input")?.as_str()?;
                let ar = v.get("agent_response")?.as_str()?;
                let combined = format!("{} {}", ui.to_lowercase(), ar.to_lowercase());
                let score = keywords.iter().filter(|kw| combined.contains(*kw)).count();
                Some((
                    score,
                    format!(
                        "User: {}
Agent: {}",
                        ui, ar
                    ),
                ))
            })
            .collect();

        // Sort descending by salience score, take top 10
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        let top: Vec<String> = scored.into_iter().take(10).map(|(_, text)| text).collect();

        if top.is_empty() {
            None
        } else {
            Some(top.join(
                "

",
            ))
        }
    }

    /// Paired memory recall — returns both semantic (third-person) and
    /// episodic (first-person) memories merged into a single context string.
    ///
    /// This is the standalone entry point for the dual-recall circuit.
    /// Mirrors the merge pattern in `prepare_chat` but without prompt composition —
    /// callers get just the memory context for injection wherever needed.
    ///
    /// \[P5\] Motivating: Essentialism — single entry point for paired memory access.
    /// pre:  both ports must be initialized; input must be non-empty; agent_webid valid; token valid
    /// post: returns Some(String) with merged semantic+episodic context; None if both recalled empty; each recall independently gated
    pub fn recall_memory(
        semantic_port: &Arc<dyn SemanticStoragePort>,
        episodic_port: &Arc<dyn EpisodicStoragePort>,
        input: &str,
        agent_webid: &WebID,
        token: &DelegationToken,
    ) -> Option<String> {
        let semantic = MemoryService::recall_semantic(semantic_port, input, token);
        let episodic = MemoryService::recall_episodic(episodic_port, input, agent_webid, token);

        match (semantic, episodic) {
            (Some(s), Some(e)) => Some(format!("{}\n\n{}", s, e)),
            (Some(s), None) => Some(s),
            (None, Some(e)) => Some(e),
            (None, None) => None,
        }
    }

    /// Store the chat exchange as an episodic triple.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  episodic_port must be initialized; input and response must be non-empty; agent_webid must be valid; token must be valid
    /// post: chat exchange is stored as episodic triple with confidence 0.7; failures are logged but not returned (best-effort)
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
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  episodic_port must be initialized; agent_webid must be valid; token must be valid; limit must be > 0
    /// post: returns Some(String) of formatted recent turns; None if no episodes or recall fails
    /// # REQ: P2-svc-chat-session-history — every history access routes through episodic storage
    /// # expect: "Service operations require explicit, scoped consent"
    /// # REQ: P4-svc-chat-ocap-history — recall requires DelegationToken with Read on Manifest
    /// # expect: "Service boundaries enforce OCAP membranes"
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
    /// passing to the condenser's `condenser_thread_summary` MCP tool.
    /// Each episode yields one user message and one assistant message.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  episodic_port must be initialized; agent_webid must be valid; token must be valid; limit must be > 0
    /// post: returns `Vec<Value>` of {role, content} messages; empty Vec if no episodes or recall fails
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  executor must be initialized; manifest must be valid; input and agent_name must be non-empty
    /// post: returns Some(String) of concatenated step outputs if cascade completes; None if no manifest or execution fails
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  input and manifest_context must be non-empty
    /// post: returns formatted string with [Manifest Context] block prepended to input
    pub fn wrap_manifest_input(input: &str, manifest_context: &str) -> String {
        format!(
            "[Manifest Context]\n{}\n[/Manifest Context]\n\n{}",
            manifest_context, input
        )
    }

    /// Apply persona constraints to filter forbidden patterns from a response.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  response must be non-empty; constraints if Some must be valid PersonaConstraints
    /// post: returns cleaned response with forbidden patterns stripped; violations logged; returns original if constraints is None
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

    /// Condense the oldest half of conversation history when approaching context limits.
    ///
    /// Fetches raw episodes, splits at midpoint, summarizes the oldest half via the
    /// inference port, and returns a rebuilt input with `[Condensed history]` +
    /// `[Recent conversation]` blocks. Returns `None` on any failure (graceful
    /// degradation — caller falls back to uncondensed context).
    async fn condense_history(
        ctx: &AgentService,
        req: &TurnRequest,
        token: &DelegationToken,
        base_input: &str,
    ) -> Option<String> {
        // \[NORMATIVE\] Sovereignty gate (H3/P2): condensing reads episodic
        // (sovereign) history — only proceed when the owner has granted consent.
        if !MemoryService::has_memory_consent(ctx, &req.agent_webid, &DataCategory::EpisodicMemory)
        {
            return None;
        }
        let episodes = MemoryService::recall_raw_episodes(
            &req.episodic_storage,
            &req.agent_webid,
            token,
            req.context_turns * 2,
        );
        if episodes.len() < 4 {
            return None; // too few messages to meaningfully condense
        }

        // Saliency-based split: keep the most recent N exchanges verbatim
        // (where N = condense_saliency_window, each exchange = 2 episodes).
        // Older episodes are summarized. This preserves recent context as
        // anchors while condensing stale history.
        let keep_count = (req.condense_saliency_window * 2).min(episodes.len().saturating_sub(2));
        let old_half = &episodes[..episodes.len() - keep_count];
        let recent_half = &episodes[episodes.len() - keep_count..];

        let recent_text = hkask_condenser::inference::format_conversation_text(recent_half);
        let old_text = hkask_condenser::inference::format_conversation_text(old_half);
        let summary_prompt =
            hkask_condenser::inference::build_summarization_prompt(&old_text, &req.input);

        let full_prompt = format!("{CONDENSER_SYSTEM_PROMPT}\n\nUser: {summary_prompt}");

        let condenser_model = req.condenser_model.as_deref().unwrap_or(&req.model);
        let params = LLMParameters {
            temperature: 0.3,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 500,
            seed: None,
            disable_thinking: true,
            adapter: None,
            bypass_fusion: true,
        };

        let port = ctx.inference_port()?;
        let result = port
            .generate_with_model(&full_prompt, &params, Some(condenser_model), None)
            .await
            .ok()?;

        let summary = result.text;
        if summary.trim().is_empty() {
            return None;
        }

        tracing::debug!(
            target: "cns.chat.condense",
            agent = %req.agent_name,
            old_msgs = old_half.len(),
            recent_msgs = recent_half.len(),
            summary_len = summary.len(),
            "History condensed"
        );

        Some(format!(
            "{base_input}\n\n[Condensed history]\n{summary}\n\n[Recent conversation]\n{recent_text}"
        ))
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx must be fully built; req.input must be non-empty; req.agent_name must be registered
    /// post: returns TurnResult with response text, token usage, tool calls, and iteration count; manifest cascade and history suffix applied; persona filter applied; Err on inference failure
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

        // 2. Build context — invariant structure for all agents:
        //    [thread history] → [semantic facts] → [episodic history] → [user input]
        //
        // Thread history is short-term memory: the active thread's recent
        // conversation stream. Episodic is long-term memory: past sessions,
        // other threads, consolidated experience. Semantic injects relevant
        // facts derived from both. All three layers coexist structurally;
        // each may be empty (None) but the assembly order is invariant.
        let history_token = req.capability_checker.grant_registry(
            DelegationAction::Read,
            req.system_webid,
            req.agent_webid,
        );
        let history_suffix = if MemoryService::has_memory_consent(
            ctx,
            &req.agent_webid,
            &DataCategory::EpisodicMemory,
        ) {
            MemoryService::recall_recent_turns(
                &req.episodic_storage,
                &req.agent_webid,
                &history_token,
                req.context_turns,
            )
        } else {
            None
        };
        let semantic_suffix = if MemoryService::has_memory_consent(
            ctx,
            &req.agent_webid,
            &DataCategory::SemanticMemory,
        ) {
            let semantic_context =
                MemoryService::recall_semantic(&req.semantic_storage, &base_input, &history_token);
            semantic_context.map(|s| format!("## Relevant Facts\n{}", s))
        } else {
            None
        };
        let mut input_with_context = match (&req.thread_history, &history_suffix, &semantic_suffix)
        {
            (Some(t), Some(e), Some(s)) => format!("{}\n\n{}\n\n{}\n\n{}", base_input, t, s, e),
            (Some(t), Some(e), None) => format!("{}\n\n{}\n\n{}", base_input, t, e),
            (Some(t), None, Some(s)) => format!("{}\n\n{}\n\n{}", base_input, t, s),
            (Some(t), None, None) => format!("{}\n\n{}", base_input, t),
            (None, Some(e), Some(s)) => format!("{}\n\n{}\n\n{}", base_input, s, e),
            (None, Some(e), None) => format!("{}\n\n{}", base_input, e),
            (None, None, Some(s)) => format!("{}\n\n{}", base_input, s),
            (None, None, None) => base_input.clone(),
        };

        // 2b. Auto-condense: if enabled and context exceeds the configured
        // pressure threshold (default 87.5%), condense older messages to
        // free context space. The most recent `saliency_window` exchanges
        // are preserved verbatim; older messages are summarized.
        if req.auto_condense
            && let Some(window) = req.context_window
        {
            let threshold = (window as f64 * req.condense_pressure_threshold as f64) as usize;
            if hkask_condenser::inference::approx_token_count(&input_with_context) > threshold
                && let Some(condensed) =
                    Self::condense_history(ctx, req, &history_token, &base_input).await
            {
                input_with_context = condensed;
            }
            // Graceful degradation: on failure, use uncondensed context.
        }

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
        // 4a. Inject improv mode instructions into the system prompt.
        let effective_input = if let Some(ref mode) = req.improv_mode {
            let improv_instruction = improv_system_prompt(mode);
            format!(
                "{}

{}",
                improv_instruction, effective_input
            )
        } else {
            effective_input
        };

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
            tools: req.tools.clone(),
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
    pub capability_checker: Arc<hkask_capability::CapabilityChecker>,
    /// System WebID for token minting
    pub system_webid: WebID,
    /// Iteration counter (0 = first iteration, incremented by caller for continuations)
    pub iteration: usize,
    /// Tool execution results from the previous iteration (None on first iteration)
    pub tool_results: Option<String>,
    /// Whether to auto-condense conversation history when approaching context limits.
    /// When true and context exceeds 87.5% of `context_window`, the oldest half
    /// of messages are condensed via `InferencePort::generate_with_model()`.
    pub auto_condense: bool,
    /// Model context window size in tokens, used for condensation threshold.
    /// None disables condensation (e.g., model metadata not yet fetched).
    pub context_window: Option<u32>,
    /// Model to use for condenser summarization (defaults to chat model if None).
    pub condenser_model: Option<String>,
    /// Context pressure threshold (0.0–1.0). When context fill exceeds this
    /// fraction of context_window, auto-condensation triggers. Default 0.875.
    pub condense_pressure_threshold: f32,
    /// Number of most recent exchanges to preserve verbatim during condensation.
    /// Older messages are summarized; these N are kept as anchors. Default 5.
    pub condense_saliency_window: usize,
    /// Pre-formatted conversation history from the active short-term thread.
    /// When set, prepended to context before episodic memory recall. This is
    /// the thread's own stream — switching threads changes this context.
    /// None if no active thread or thread has no turns.
    pub thread_history: Option<String>,
    /// Active improv mode — when set, prepends mode-specific instructions
    /// to the system prompt so the model adopts the interaction posture.
    /// None means no improv posture (default agent behavior).
    pub improv_mode: Option<hkask_improv::ImprovMode>,
    /// Source of this turn — which communication channel the message arrived from.
    /// None means unknown/CLI. When set, enables the agent
    /// to maintain separate conversation contexts per source (P12: every action
    /// has an author).
    pub source: Option<MessageSource>,
    /// OpenAI-compatible tool definitions for native function calling.
    /// Built from MCP-discovered tools by the REPL at init time.
    /// When present, the model may return structured tool calls.
    pub tools: Option<Vec<ChatToolDefinition>>,
}

/// Which communication channel a turn's input arrived from.
///
/// Enables agents to distinguish between different humans and channels,
/// maintaining separate conversation contexts. Per P12 (Replicant Host Mandate),
/// every action must trace to an author — the source field provides that trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSource {
    /// Message from a Matrix room.
    Matrix {
        /// Matrix room ID (e.g., "!abc123:example.com")
        room_id: String,
        /// Sender's Matrix user ID (e.g., "@bob-jones:example.com")
        sender_mxid: String,
    },
    /// Message from the daemon socket (local agent-to-agent).
    Daemon {
        /// Sender's WebID
        sender_webid: String,
    },
    /// Message from the CLI REPL (stdin).
    Cli,
    /// Message from the HTTP API.
    Api,
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

/// Generate a system-prompt instruction for the active improv mode.
///
/// Prepended to the effective input before inference so the model
/// adopts the specified interaction posture. Each mode has a concise
/// instruction that encodes its core constraint.
fn improv_system_prompt(mode: &hkask_improv::ImprovMode) -> String {
    match mode {
        hkask_improv::ImprovMode::Plussing => {
            "[Improv mode: Plussing]\n\
             Find what you can agree with in the user's message. Build constructively on those points.\n\
             Silently omit anything you disagree with — never explicitly negate or reject.\n\
             If nothing is agreeable, redirect constructively: \"Let's explore this from a different angle.\"".to_string()
        }
        hkask_improv::ImprovMode::YesAnd => {
            "[Improv mode: Yes And]\n\
             Accept the user's entire message as valid. Extend it with a novel, additive layer.\n\
             Your extension must build on their contribution, not replace or contradict it.\n\
             Start with \"Yes, and also:\" or equivalent acceptance language.".to_string()
        }
        hkask_improv::ImprovMode::YesBut => {
            "[Improv mode: Yes But]\n\
             Accept the user's entire message as valid. Then append a constructive constraint\n\
             or boundary condition that narrows scope without contradicting.\n\
             Frame as additive guidance: \"Yes, and let's also consider...\" not \"No, because...\"\n\
             Never use rejecting language (no, wrong, can't, impossible).".to_string()
        }
        hkask_improv::ImprovMode::Freestyling { .. } => {
            "[Improv mode: Freestyling]\n\
             Engage in rapid, associative, creative response. Keep responses short (1-3 sentences).\n\
             Build on the energy of the conversation — this is creative exploration, not careful analysis.\n\
             Take creative leaps. Connect ideas associatively. Don't over-think.".to_string()
        }
        hkask_improv::ImprovMode::Riffing { .. } => {
            "[Improv mode: Riffing]\n\
             Take one idea from the user's message and explore it independently as a solo tangent.\n\
             Go deep, go wide, go creative — this is your independent exploration space.\n\
             When done, either return to the main topic with a synthesis of your findings,\n\
             or signal that this tangent deserves its own thread.".to_string()
        }
        hkask_improv::ImprovMode::Cascade(c) => {
            let step_labels: Vec<String> = c
                .modes
                .iter()
                .map(|m| m.label().to_string())
                .collect();
            format!(
                "[Improv mode: Cascade — {}]\n\
                 Apply these improv modes in sequence to your response:\n\
                 {}\n\
                 Each step's output feeds into the next. Stay within the matryoshka limit of 7 total applications.",
                step_labels.join(" → "),
                step_labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| format!("  {}. {}", i + 1, label))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}
