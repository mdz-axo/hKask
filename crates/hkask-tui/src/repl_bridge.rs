//! ReplBridge — traits for connecting the TUI to inference and system state.
//!
//! The TUI crate cannot depend on `hkask-cli` (dependency direction violation).
//! Two traits define the interfaces:
//! - `SystemBridge`: monitoring data (gas, CNS, context, pods) — used by Workspace tick
//! - `ReplBridge`: full bridge (monitoring + inference) — used by windows
//!
//! Both traits are implemented by the same concrete type in `hkask-repl`.
//! Windows receive `Arc<dyn ReplBridge>` (full access); the Workspace receives
//! `Arc<dyn SystemBridge>` (monitoring only).
//!
//! # RDF HMem
//! ```text
//! ⟨Workspace⟩ uses ⟨SystemBridge⟩ .
//! ⟨ChatWindow⟩ uses ⟨ReplBridge⟩ .
//! ⟨ReplBridge⟩ delegatesTo ⟨InferenceLoop, GovernedTool, ChatService⟩ .
//! ```

/// Result of a single inference turn.
#[derive(Debug, Clone)]
pub struct TuiTurnResult {
    pub text: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub gas_cost: u64,
    pub iterations: usize,
    pub budget_exhausted: bool,
}

/// Opaque identity for one asynchronous inference operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InferenceRequestId(uuid::Uuid);

impl InferenceRequestId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for InferenceRequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// State of the inference engine, polled by the TUI each frame.
#[derive(Debug, Clone)]
pub enum InferenceState {
    /// No inference in progress
    Idle,
    /// Inference is running (the TUI should show a spinner)
    Thinking,
    /// Inference completed successfully
    Done(TuiTurnResult),
}

/// System monitoring bridge — read-only access to agent state.
///
/// Used by the Workspace tick loop. Methods are infallible and run on the
/// event-loop thread, so implementations must keep synchronization brief and
/// must not perform blocking service or network I/O.
pub trait SystemBridge: Send + Sync {
    /// Get the current agent name.
    fn userpod_name(&self) -> &str;
    /// Get the current model name.
    fn model_name(&self) -> &str;
    /// Get gas remaining.
    fn gas_remaining(&self) -> u64;
    /// Get gas cap.
    fn gas_cap(&self) -> u64;
    /// Get CNS alert count (warning + critical).
    fn cns_alert_count(&self) -> u32;
    /// Get context window pressure (0.0–1.0).
    fn context_pressure(&self) -> f64;
    /// Get MCP server count (loaded / total).
    fn mcp_status(&self) -> (usize, usize);
    /// Get pod counts (curator, userpod, team), or `None` when scanning fails.
    fn pod_counts(&self) -> Option<(usize, usize)>;
    /// Get CNS domain health summary.
    fn cns_domains(&self) -> Vec<(String, bool)>;
}

/// Rendered result of a model switch — the resolved model name and a
/// human-readable detail string for display.
#[derive(Debug, Clone)]
pub struct ModelSwitchResult {
    /// The model name now in effect. May differ from the requested name when
    /// the catalog resolved a single match; equals the request when the
    /// provider was unreachable (stored verbatim for the next inference).
    pub resolved_name: String,
    /// Multi-line detail for display (family, parameters, quantization, or a
    /// soft-error note when the provider was unreachable / multiple matches).
    pub detail: String,
}

/// Model info for TUI display — a TUI-owned subset so `hkask-tui` does not
/// depend on `hkask-services-inference`. Populated from the inference
/// router's catalog by the bridge implementor.
#[derive(Debug, Clone)]
pub struct TuiModelInfo {
    pub name: String,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Settings + model mutation surface for the TUI.
///
/// Distinct from [`ReplBridge`] (inference primitives) to keep each trait's
/// public surface ≤7 items — the TUI command surface needs to reach
/// `ReplState` for model/settings mutation, but `hkask-tui` cannot depend on
/// `hkask-repl` (dependency direction). The same concrete bridge implements
/// both; the TUI receives `Arc<dyn SettingsBridge>` and ChatWindow renders
/// the results.
pub trait SettingsBridge: Send + Sync {
    /// Switch to `name`. Resolves via the model catalog: on a single match,
    /// updates the active model and populates model metadata; on zero matches
    /// or a provider error, stores the name verbatim; on multiple matches,
    /// leaves the model unchanged and returns the candidate list as detail.
    fn set_model(&self, name: &str) -> ModelSwitchResult;

    /// List available models from the catalog (TTL-cached by the implementor).
    /// Returns `Err` with a displayable message when no provider is reachable.
    fn list_models(&self) -> anyhow::Result<Vec<TuiModelInfo>>;

    /// Render the current REPL settings as a multi-line display string.
    fn settings_display(&self) -> String;

    /// Apply `key=value`. Returns a rendered confirmation on success or an
    /// error message on validation failure. Persistence is the implementor's
    /// responsibility.
    fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<String>;
}

/// Full bridge — chat/inference + all monitoring methods.
///
/// Used by ChatWindow, CuratorWindow, ScenariosWindow, and status windows.
/// Inference is async (start → poll) to avoid blocking the TUI event loop.
/// Monitoring methods mirror `SystemBridge` so a single `Arc<dyn ReplBridge>`
/// suffices for windows that need both.
pub trait ReplBridge: SystemBridge {
    // ── Inference ──────────────────────────────────────────────────

    /// Start inference on a background task and return its request identity.
    fn start_inference(&self, input: String) -> InferenceRequestId;
    /// Poll one inference request without consuming another request's result.
    fn poll_inference(&self, request: InferenceRequestId) -> InferenceState;
    /// Get current streaming text for one inference request.
    fn streaming_text(&self, request: InferenceRequestId) -> String;
    /// Blocking send (for quick commands, not normal chat).
    fn send_message_blocking(&self, input: &str) -> TuiTurnResult;
    /// Send a message to the Curator daemon and get a response.
    fn send_curator_message(&self, input: &str) -> String;
    /// Start inference scoped to a single MCP server's tools.
    fn start_scoped_inference(&self, input: String, _mcp_server: &str) -> InferenceRequestId {
        self.start_inference(input)
    }
}

/// Session-state surface for the TUI — agent switching, agent listing, and
/// session history. Distinct from [`SettingsBridge`] (model/settings) to
/// keep each trait's public surface ≤7 items. The same concrete bridge
/// implements it; the TUI receives `Arc<dyn SessionBridge>` optionally and
/// falls back to a stub when unset.
pub trait SessionBridge: Send + Sync {
    /// The live current userpod name.
    fn current_agent(&self) -> String;
    /// Render the registered-agent list as a display string.
    fn list_agents_display(&self) -> String;
    /// Render the session history (recent turns) as a display string.
    fn history_display(&self) -> String;
}
