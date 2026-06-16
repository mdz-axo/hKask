// G2 Justification: 1 public item — embedding generation error enum. Standalone because embedding is a distinct infrastructure concern (vector DB / semantic search) that doesn't compose with inference or tool ports. ≤7 cap met.

/// Errors from embedding generation backends (OpenAI, local models, etc.).
#[derive(Debug, Clone, thiserror::Error)]
pub enum EmbeddingGenerationError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("API error: status {0}: {1}")]
    Api(u16, String),
    #[error("JSON parse error: {0}")]
    Json(String),
    #[error("Empty response from embedding model")]
    EmptyResponse,
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}
