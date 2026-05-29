//! hKask Storage — SQLite + SQLCipher storage backend

pub mod agent_registry;
pub mod audit_log;
pub mod blobs;
pub mod database;
pub mod embeddings;
pub mod git_cas;
pub mod goal_judge;
pub mod goals;
pub mod metacognition;
pub mod nu_event_store;
pub mod revocation_store;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod standing_session;
pub mod triples;
pub mod user_store;

pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use audit_log::{AuditEntry, AuditLogError, AuditLogStore};
pub use blobs::{Blob, BlobError, BlobStore};
pub use database::Database;
pub use embeddings::{Embedding, EmbeddingError, EmbeddingStore};
pub use git_cas::GitCas;
pub use goal_judge::{GoalJudgeAdapter, GoalJudgeError, GoalVerifier};
pub use goals::{GoalRepositoryError, Result as GoalResult, SqliteGoalRepository};
pub use metacognition::{MetacognitionError, MetacognitionStore, StoredSnapshot};
pub use nu_event_store::{NuEventError, NuEventStore};
pub use revocation_store::{RevocationError, RevocationRecord, RevocationStore};
pub use security::sanitize_path;
pub use sovereignty::{
    SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError,
    SovereigntyStoreStats,
};
pub use spec_store::{CnsSpecObserver, DefaultSpecCurator, SqliteSpecStore};
pub use standing_session::{
    StandingSessionError, StandingSessionStore, StoredMessage, StoredSession,
};
pub use triples::{Triple, TripleError, TripleStore};
pub use user_store::{UserStore, UserStoreError};
