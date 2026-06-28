#![allow(unused_imports)]
//! hKask Storage — SQLite + SQLCipher storage backend
#[macro_use]
mod store_macros;
pub mod lock_helpers;
pub use hkask_types::time::now_rfc3339;
pub use lock_helpers::{lock_mutex, read_rwlock, write_rwlock};
pub use store_macros::Store;
pub mod agent_registry;
pub mod archive;
pub mod consent_store;
pub mod database;
pub mod embeddings;
pub mod escalation;
pub mod gallery;
pub mod goals;
pub mod kata_history;
pub mod nu_event_store;
pub mod registry_types;
pub mod security;
pub mod sovereignty;
pub mod spec_store;
pub mod spec_types;
pub mod triples;
pub mod user_store;
pub mod wallet_store;
pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use archive::{ArchiveError, BackupArchive, BackupMeta, MigrationReceipt};
pub use consent_store::{ConsentStore, ConsentStoreError, StoredConsentRecord};
pub use database::{
    Database, DatabaseError, check_passphrase, in_memory_db, open_database, open_or_repair,
};
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
pub use registry_types::{
    AgentDefinition, Charter, Contact, RegisteredAgent, Responsibility, Right, ScheduledTask,
    UserProfile,
};
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
