//! Manifest executor — core execution loop
//
//! Implements the fixed logic that executes ANY manifest without modification.
//! Per architecture v0.21.0: ~50 lines of Rust that never changes when templates are added/edited.

use serde::{Deserialize, Serialize};

/// Model requirements for template execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModelRequirements {
    /// Required model ID (e.g., "ollama/llama-3.1-8b-instruct")
    pub required: String,
    /// Minimum context length required
    pub min_context: u32,
    /// Whether reasoning capability is required
    #[serde(default)]
    pub reasoning_required: bool,
    /// Required capabilities (e.g., "code", "math", "analysis")
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Embedding dimension (for embedding models only)
    pub dimension: Option<u32>,
    /// Pooling strategy (for embedding models only)
    pub pooling: Option<String>,
}
