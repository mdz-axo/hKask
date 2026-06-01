//! hKask Storage — SQLite + SQLCipher storage backend

pub mod agent_registry;
pub mod audit_log;
pub mod database;
pub mod embeddings;
pub mod goals;
pub mod lock_priority;

pub mod nu_event_store;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod standing_session;
pub mod triples;
pub mod user_store;

pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use audit_log::{AuditEntry, AuditLogError, AuditLogStore};
pub use database::Database;
pub use embeddings::{Embedding, EmbeddingError, EmbeddingStore};
pub use goals::{GoalRepositoryError, Result as GoalResult, SqliteGoalRepository};
pub use hkask_types::TripleID;
pub use lock_priority::{LockPriority, PriorityLockGuard};

pub use nu_event_store::{NuEventError, NuEventStore};
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
