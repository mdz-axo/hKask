//! hKask Storage — SQLite + SQLCipher storage backend

pub mod agent_registry;
pub mod audit_log;
pub mod database;
pub mod embeddings;
pub mod goals;
pub mod nu_event_store;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod standing_session;
pub mod triples;
pub mod user_store;

pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use audit_log::{AuditEntry, AuditLogStore};
pub use database::Database;
pub use embeddings::{Embedding, EmbeddingError, EmbeddingStore};
pub use goals::{GoalRepositoryError, SqliteGoalRepository};
pub use hkask_types::TripleID;

pub use nu_event_store::NuEventStore;
pub use security::sanitize_path;
pub use sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError};
pub use spec_store::{DefaultSpecCurator, SqliteSpecStore};
pub use standing_session::{StandingSessionStore, StoredMessage, StoredSession};
pub use triples::{Triple, TripleError, TripleStore};
