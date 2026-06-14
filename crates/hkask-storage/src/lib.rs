//! hKask Storage — SQLite + SQLCipher storage backend

#[macro_use]
mod store_macros;
pub mod lock_helpers;

pub use lock_helpers::{lock_mutex, read_rwlock, write_rwlock};
pub use store_macros::Store;
pub use store_macros::now_rfc3339;

pub mod agent_registry;
pub mod consent_store;
pub mod database;
pub mod embeddings;
pub mod escalation;
pub mod goals;
pub mod nu_event_store;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod spec_types;
pub(crate) mod standing_session;
pub use standing_session::{
    StandingSessionError, StandingSessionStore, StoredMessage, StoredSession,
};
pub mod gallery;
pub mod triples;
pub mod user_store;
pub mod wallet_store;

pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use consent_store::{ConsentStore, ConsentStoreError, StoredConsentRecord};
pub use database::{Database, DatabaseError, in_memory_db, open_database};
pub use embeddings::{EmbeddingError, EmbeddingStore, SimilarityResult, StoredEmbedding};
pub use escalation::{
    EscalationBatch, EscalationEntry, EscalationError, EscalationQueue, EscalationStats,
    EscalationStatus,
};
pub use gallery::{
    GalleryMode, GalleryRecord, GalleryStore, GalleryStoreError, ImageRecord, TagRecord,
};
pub use goals::{GoalRepositoryError, QuarantinedGoal, SqliteGoalRepository};
pub use hkask_types::TripleID;
pub use nu_event_store::{DecayConfig, NuEventError, NuEventStore, WeightedEvent};
pub use security::sanitize_path;
pub use sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError};
pub use spec_store::SpecStore;
pub use spec_store::SqliteCurationRecordStore;
pub use spec_store::SqliteSpecStore;
pub use spec_types::{
    Criterion, DomainAnchor, DriftReport, GoalSpec, Spec, SpecCategory, SpecCurationRecord,
    SpecCurator, SpecError, SpecId,
};

pub use triples::{Triple, TripleError, TripleStore};
pub use user_store::UserStoreError;
pub use wallet_store::WalletStore;
