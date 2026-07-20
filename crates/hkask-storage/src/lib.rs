//! hKask Storage — SQLite + SQLCipher storage backend
//!
//! Foundation types (`Database`, `Store`, lock helpers) are re-exported
//! from `hkask-storage-core`. Domain-specific storage modules live here
//! or in sub-crates behind this facade.

// ── Re-export foundation from hkask-storage-core ─────────────────────

pub use hkask_storage_core::database::{Database, DatabaseError};

pub use hkask_storage_core::{
    check_passphrase, define_driver_store, impl_from_db_error, open_database, open_or_repair,
    sanitize_path,
};
pub use hkask_types::time::now_rfc3339;

// ── Domain storage modules ───────────────────────────────────────────

pub mod agent_registry;
pub mod consent_store;
pub mod embeddings;
pub mod escalation;
pub mod gallery;
pub mod goals;
pub mod hmem;
pub mod kata;
pub mod nu_event_store;
pub mod sovereignty;
pub mod token_registry;
pub mod user_store;
pub mod wallet;

// ── Domain re-exports ────────────────────────────────────────────────

pub use agent_registry::{AgentRegistryError, AgentRegistryStore};
pub use consent_store::{ConsentStore, ConsentStoreError, StoredConsentRecord};
pub use embeddings::{EmbeddingError, EmbeddingStore, SimilarityResult, StoredEmbedding};
pub use escalation::{
    EscalationBatch, EscalationEntry, EscalationError, EscalationQueue, EscalationStats,
    EscalationStatus,
};
pub use gallery::{
    FaceRegistryRecord, GalleryMode, GalleryRecord, GalleryStore, GalleryStoreError, ImageRecord,
    TagRecord,
};
pub use goals::{GoalRepositoryError, QuarantinedGoal, SqliteGoalRepository};
pub use hkask_types::HMemId;
pub use hmem::archive::{ArchiveError, BackupArchive, BackupMeta, MigrationReceipt};
pub use hmem::{HMem, HMemError, HMemStore};
pub use kata::{KataHistoryEntry, KataHistoryError, KataHistoryStore};
pub use nu_event_store::{DecayConfig, NuEventStore, WeightedEvent};
pub use sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError};
pub use token_registry::TokenRegistryStore;

pub use user_store::UserStoreError;
pub use wallet::WalletStore;
