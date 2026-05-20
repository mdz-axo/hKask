//! hKask Storage — SQLite + SQLCipher storage backend

pub mod blobs;
pub mod capability_cache;
pub mod database;
pub mod embeddings;
pub mod git_cas;
pub mod triples;
pub mod webid_store;

pub use blobs::{Blob, BlobError, BlobStore};
pub use capability_cache::{CacheStats, CapabilityCache, CapabilityCacheEntry};
pub use database::Database;
pub use embeddings::{Embedding, EmbeddingError, EmbeddingStore};
pub use git_cas::GitCas;
pub use triples::{Triple, TripleError, TripleStore};
pub use webid_store::{StoredCapability, StoredWebIDEntry, WebIDStore, WebIDStoreError, WebIDStoreStats};
