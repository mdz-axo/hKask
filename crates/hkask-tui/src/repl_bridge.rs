//! ReplBridge — trait for connecting the TUI chat window to inference.
//!
//! The TUI crate cannot depend on `hkask-cli` (dependency direction violation).
//! The ReplBridge trait defines the minimal interface the ChatWindow needs
//! to send messages and receive responses. `hkask-cli` implements it using
//! the full ReplState + InferenceLoop + GovernedTool stack.
//!
//! # RDF Triple
//! ```text
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

/// Bridge between the TUI chat window and the inference engine.
pub trait ReplBridge: Send + Sync {
    /// Start inference on a background task. Returns immediately.
    /// Call `poll_inference()` each frame to check for completion.
    fn start_inference(&self, input: String);

    /// Poll the inference state. Returns `Idle` if no inference is running,
    /// `Thinking` while inference is in progress, or `Done(result)` when complete.
    fn poll_inference(&self) -> InferenceState;

    /// Get current streaming text (partial response during inference).
    fn streaming_text(&self) -> String;

    /// Blocking send (used for quick commands, not for normal chat).
    fn send_message_blocking(&self, input: &str) -> TuiTurnResult;

    /// Get the current agent name.
    fn agent_name(&self) -> &str;

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

    /// Get pod counts (curator, replicant, team).
    fn pod_counts(&self) -> (usize, usize, usize);

    /// Get CNS domain health summary.
    fn cns_domains(&self) -> Vec<(String, bool)>;

    /// Send a message to the Curator daemon and get a response.
    fn send_curator_message(&self, input: &str) -> String;

    /// Start inference scoped to a single MCP server's tools.
    /// Only tools belonging to `mcp_server` are available to the model.
    /// Default implementation falls back to unscoped inference.
    fn start_scoped_inference(&self, input: String, _mcp_server: &str) {
        self.start_inference(input);
    }
}
