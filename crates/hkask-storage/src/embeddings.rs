//! Embedding storage

pub struct EmbeddingStore;

impl EmbeddingStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmbeddingStore {
    fn default() -> Self {
        Self::new()
    }
}
