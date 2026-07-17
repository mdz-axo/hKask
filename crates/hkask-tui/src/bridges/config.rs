//! ConfigDataBridge — trait for inference and system settings in the TUI.
//!
//! Provides the configuration window with live settings data. Implemented
//! by the CLI via ReplSettings.

use std::sync::Arc;

/// Snapshot of the current REPL settings for TUI display.
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    pub model: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub tool_loop_limit: usize,
    pub context_turns: usize,
    pub gas_heuristic: u64,
    pub gas_cap: u64,
    pub auto_condense: bool,
    pub embedding_model: String,
    pub classifier_model: String,
    pub mcp_loaded: usize,
    pub mcp_total: usize,
}

/// Trait for querying configuration state.
pub trait ConfigDataBridge: Send + Sync {
    /// Snapshot of all visible REPL settings.
    fn config_snapshot(&self) -> ConfigSnapshot;
}

/// Mock implementation for TUI development and testing.
pub struct MockConfigBridge {
    pub snapshot: ConfigSnapshot,
}

impl Default for MockConfigBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl MockConfigBridge {
    pub fn new() -> Self {
        Self {
            snapshot: ConfigSnapshot {
                model: "mock-model".into(),
                temperature: 0.7,
                top_p: 0.9,
                max_tokens: 512,
                tool_loop_limit: 21,
                context_turns: 3,
                gas_heuristic: 500,
                gas_cap: 10_000,
                auto_condense: true,
                embedding_model: "mock-embed".into(),
                classifier_model: "mock-classify".into(),
                mcp_loaded: 2,
                mcp_total: 4,
            },
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl ConfigDataBridge for MockConfigBridge {
    fn config_snapshot(&self) -> ConfigSnapshot {
        self.snapshot.clone()
    }
}
