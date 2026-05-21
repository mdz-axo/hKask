//! Semantic memory pipeline

use hkask_storage::{Embedding, EmbeddingError, EmbeddingStore, Triple, TripleError, TripleStore};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
}

/// Semantic memory — shared knowledge graph
pub struct SemanticMemory {
    triple_store: TripleStore,
    embedding_store: EmbeddingStore,
}

impl SemanticMemory {
    pub fn new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self {
        Self {
            triple_store,
            embedding_store,
        }
    }

    /// Store a semantic triple (public by default)
    pub fn store(&self, triple: Triple) -> Result<(), SemanticMemoryError> {
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    /// Query by entity
    pub fn query(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError> {
        Ok(self.triple_store.query_by_entity(entity)?)
    }

    /// Store embedding for semantic search
    pub fn store_embedding(&self, embedding: Embedding) -> Result<(), SemanticMemoryError> {
        self.embedding_store.insert(&embedding)?;
        Ok(())
    }
}
