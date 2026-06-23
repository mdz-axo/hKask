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
pub struct TurnResult {
    /// The agent's response text
    pub text: String,
    /// Token usage for this turn
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    /// Gas cost of this turn
    pub gas_cost: u64,
    /// Number of tool-call iterations
    pub iterations: usize,
    /// Whether energy budget was exhausted
    pub budget_exhausted: bool,
}

/// Bridge between the TUI chat window and the inference engine.
///
/// This is a trait so the TUI crate can remain independent of `hkask-cli`.
/// The CLI crate provides the concrete implementation that wraps ReplState.
pub trait ReplBridge: Send + Sync {
    /// Send a user message and get the agent's response.
    /// This may block during inference (same as the rustyline REPL).
    fn send_message(&self, input: &str) -> TurnResult;

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
}
