//! hKask Storage — SQLite + SQLCipher storage backend

pub mod audit_log;
pub mod blobs;
pub mod database;
pub mod embeddings;
pub mod git_cas;
pub mod goal_judge;
pub mod goals;
pub mod nu_event_store;
pub mod sovereignty;
pub mod spec_store;
pub mod triples;

pub use audit_log::{AuditEntry, AuditLogError, AuditLogStore};
pub use blobs::{Blob, BlobError, BlobStore};
pub use database::Database;
pub use embeddings::{Embedding, EmbeddingError, EmbeddingStore};
pub use git_cas::GitCas;
pub use goal_judge::{GoalJudgeAdapter, GoalJudgeError, GoalVerifier};
pub use goals::{
    GoalRepositoryError, GoalRepositoryPort, Result as GoalResult, SqliteGoalRepository,
};
pub use nu_event_store::{NuEventError, NuEventStore};
pub use sovereignty::{
    SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError,
    SovereigntyStoreStats,
};
pub use spec_store::{CnsSpecObserver, DefaultSpecCurator, SqliteSpecStore};
pub use triples::{Triple, TripleError, TripleStore};
