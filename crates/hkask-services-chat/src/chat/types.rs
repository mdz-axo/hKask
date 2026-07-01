//! Chat service types — request/response structs, token accounting, message sources.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_capability::{AuthContext, DelegationToken};
use hkask_ports::{ChatToolDefinition, InferencePort, StructuredToolCall};
use hkask_types::PersonaConstraints;
use hkask_types::WebID;
use hkask_types::template::LLMParameters;

/// Token usage breakdown for gas accounting.
#[derive(Clone, Serialize, Deserialize)]
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

/// Response from a chat inference call.
///
/// Carries the response text, token usage, and structured tool calls
/// (from native function calling) alongside the finish reason so the
/// calling surface can detect tool-call completions.
#[derive(Clone, Serialize, Deserialize)]
pub struct ChatTurnResponse {
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
pub struct ChatTurnRequest {
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
