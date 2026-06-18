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
pub mod gallery;
pub mod goals;
pub mod kata_history;
pub mod nu_event_store;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod spec_types;
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
pub use kata_history::{KataHistoryEntry, KataHistoryError, KataHistoryStore};
pub use nu_event_store::{DecayConfig, NuEventStore, WeightedEvent};
pub use security::sanitize_path;
pub use sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError};
pub use spec_store::SpecStore;
pub use spec_store::SqliteCurationRecordStore;
pub use spec_store::SqliteSpecStore;
pub use spec_types::{
    Criterion, DomainAnchor, DriftReport, GoalSpec, Spec, SpecCategory, SpecCurationRecord,
    SpecCurator, SpecError, SpecId, infer_spec_category,
};

pub use triples::{Triple, TripleError, TripleStore};
pub use user_store::UserStoreError;
pub use wallet_store::WalletStore;

// ── From impls for ServiceError ──────────────────────────────────────
impl From<DatabaseError> for hkask_services_core::ServiceError {
    fn from(e: DatabaseError) -> Self {
        hkask_services_core::ServiceError::Storage { message: e.to_string() }
    }
}
impl From<TripleError> for hkask_services_core::ServiceError {
    fn from(e: TripleError) -> Self {
        hkask_services_core::ServiceError::Triple { message: e.to_string() }
    }
}

impl From<UserStoreError> for hkask_services_core::ServiceError {
    fn from(e: UserStoreError) -> Self {
        hkask_services_core::ServiceError::UserStore { message: e.to_string() }
    }
}

impl From<AgentRegistryError> for hkask_services_core::ServiceError {
    fn from(e: AgentRegistryError) -> Self {
        hkask_services_core::ServiceError::AgentRegistryStore { message: e.to_string() }
    }
}
impl From<ConsentStoreError> for hkask_services_core::ServiceError {
    fn from(e: ConsentStoreError) -> Self {
        hkask_services_core::ServiceError::ConsentStore { message: e.to_string() }
    }
}
impl From<SovereigntyStoreError> for hkask_services_core::ServiceError {
    fn from(e: SovereigntyStoreError) -> Self {
        hkask_services_core::ServiceError::SovereigntyStore { message: e.to_string() }
    }
}
impl From<SpecError> for hkask_services_core::ServiceError {
    fn from(e: SpecError) -> Self {
        hkask_services_core::ServiceError::Spec { message: e.to_string() }
    }
}
impl From<GoalRepositoryError> for hkask_services_core::ServiceError {
    fn from(e: GoalRepositoryError) -> Self {
        hkask_services_core::ServiceError::GoalRepo { message: e.to_string() }
    }
}
impl From<EscalationError> for hkask_services_core::ServiceError {
    fn from(e: EscalationError) -> Self {
        hkask_services_core::ServiceError::Escalation { message: e.to_string() }
    }
}
