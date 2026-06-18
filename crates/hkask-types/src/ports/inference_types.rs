use serde::{Deserialize, Serialize};

/// Inference error types
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Model error: {0}")]
    Model(String),
    #[error("Generation error: {0}")]
    Generation(String),
    #[error("JSON error: {0}")]
    Json(String),
    #[error("Circuit open: {0}")]
    CircuitOpen(String),
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Token probability from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProbability {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TokenProb>,
}

/// Top-k token probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
}

/// Confidence = avg(prob) × (1 - sqrt(variance)). Higher avg + lower variance = higher confidence.
///
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
/// pre:  probs is a slice of [`TokenProbability`] values; may be empty
/// post: returns 0.0 if probs is empty; otherwise returns avg(prob) × (1 - √variance),
///       a value in [0.0, 1.0] where higher average probability and lower variance produce higher confidence
pub fn compute_confidence(probs: &[TokenProbability]) -> f64 {
    if probs.is_empty() {
        return 0.0;
    }

    let avg_prob: f64 = probs.iter().map(|p| p.prob).sum::<f64>() / probs.len() as f64;

    let variance: f64 = probs
        .iter()
        .map(|p| (p.prob - avg_prob).powi(2))
        .sum::<f64>()
        / probs.len() as f64;

    avg_prob * (1.0 - variance.sqrt())
}

/// Structured tool call from a model response (OpenAI/Anthropic/Gemini native function calling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredToolCall {
    pub server: String,
    pub tool: String,
    pub args: serde_json::Value,
    pub call_id: Option<String>,
}

/// Inference result from LLM backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub text: String,
    pub model: String,
    pub usage: InferenceUsage,
    pub finish_reason: String,
    pub token_probabilities: Option<Vec<TokenProbability>>,
    /// Populated when `finish_reason == "tool_calls"`. For models without native function calling,
    /// always empty — `parse_tool_calls()` in `tool_augmented` handles `<<tool:...>>` fallback.
    #[serde(default)]
    pub tool_calls: Vec<StructuredToolCall>,
}
