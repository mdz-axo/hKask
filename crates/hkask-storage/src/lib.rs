//! hKask Storage — SQLite + SQLCipher storage backend
//!
//! Consolidated from hkask-storage + hkask-database + hkask-storage-core.
//! Database driver abstraction and storage core foundation are now modules
//! within this crate. Domain-specific storage modules follow.

// ── Core modules (must precede domain modules for macro_use) ─────────

pub mod core;
pub mod database;

// ── Re-export foundation from core ───────────────────────────────────

pub use core::database::{Database, DatabaseError};
pub use core::{
    check_passphrase, define_driver_store, impl_from_db_error, open_database, open_or_repair,
    sanitize_path,
};

// ── Re-export database driver types ──────────────────────────────────

pub use database::{
    DatabaseDriver, DbProvider, PostgresDriver, SqliteDriver, WAL_PRAGMA_BATCH, init_wal_pragmas,
};

pub use hkask_types::time::now_rfc3339;

// ── Domain storage modules ───────────────────────────────────────────

pub mod consent_store;
pub mod embeddings;
pub mod escalation;
pub mod gallery;
pub mod goals;
pub mod hmem;
pub mod kata;
pub mod regulation_store;
pub mod sovereignty;
pub mod token_registry;
pub mod user_store;
pub mod wallet;

// ── Domain re-exports ────────────────────────────────────────────────

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
pub use regulation_store::{DecayConfig, RegulationArchive, WeightedEvent};
pub use sovereignty::{SovereigntyBoundaryEntry, SovereigntyBoundaryStore, SovereigntyStoreError};
pub use token_registry::TokenRegistryStore;

pub use user_store::UserStoreError;
pub use wallet::WalletStore;
