//! hkask-mcp-replica — library target for integration tests.
//!
//! Re-exports the public types from services and storage so that
//! integration tests can use the replica's dependency chain.

pub use hkask_services::cosine_distance;
pub use hkask_storage::embeddings::{EmbeddingStore, StoredEmbedding};
