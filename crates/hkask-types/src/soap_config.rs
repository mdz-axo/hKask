//! Inference configuration types — Loop 1 (Inference): pure data
//
//! Configuration data for inference backends. Environment variable
//! parsing and file I/O belong in hkask-cli, not hkask-types.

use serde::{Deserialize, Serialize};

/// Inference configuration — pure data, no I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Model identifier (e.g., "qwen3:8b")
    pub model: String,
    /// Inference temperature (0.0-1.0)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Per-request wall-clock timeout in seconds
    pub timeout_secs: u64,
    /// Maximum events per request
    pub max_events: usize,
    /// Maximum subjective text length
    pub max_subjective_len: usize,
    /// Maximum event message length
    pub max_message_len: usize,
}
