//! hKask Storage — SQLite + SQLCipher storage backend

pub mod agent_registry;
pub mod consent_store;
pub mod database;
pub mod embeddings;
pub mod goals;
pub mod nu_event_store;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod spec_types;
pub(crate) mod standing_session;
pub use standing_session::StandingSessionStore;
pub mod triples;
pub mod user_store;

pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use consent_store::{ConsentStore, ConsentStoreError, StoredConsentRecord};
pub use database::Database;
pub use embeddings::EmbeddingStore;
pub use goals::{GoalRepositoryError, SqliteGoalRepository};
pub use hkask_types::TripleID;
pub use hkask_types::ports::{EmbeddingError, EmbeddingPort, SimilarityResult, StoredEmbedding};
pub use nu_event_store::{NuEventError, NuEventStore};
pub use security::sanitize_path;
pub use sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError};
pub use spec_store::SqliteSpecStore;
pub use spec_types::{
    Criterion, DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurationRecord, SpecCurator,
    SpecError, SpecId, SpecStore,
};

pub use triples::{Triple, TripleError, TripleStore};
pub use user_store::UserStoreError;
