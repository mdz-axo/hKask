//! hKask Storage — SQLite + SQLCipher storage backend

pub mod blobs;
pub mod database;
pub mod embeddings;
pub mod git_cas;
pub mod sovereignty;
pub mod triples;

pub use blobs::{Blob, BlobError, BlobStore};
pub use database::Database;
pub use embeddings::{Embedding, EmbeddingError, EmbeddingStore};
pub use git_cas::GitCas;
pub use sovereignty::{
    SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError,
    SovereigntyStoreStats,
};
pub use triples::{Triple, TripleError, TripleStore};
