//! hKask Backup Service — policy layer on top of GitCASPort.
//!
//! Extracted from `hkask-services` to enable parallel compilation.
//! Public API: `BackupService`, `BackupError`, and supporting types.
//!
//! # REQ: P1 (User Sovereignty) — user controls what is tracked.
//! expect: "My backup data is tracked under my sovereignty control"
//! # REQ: P4 (Clear Boundaries) — delegates to hexagonal GitCASPort, never raw git.
//! expect: "Backup operations delegate through OCAP boundaries"

pub mod config;
pub mod r#loop;
pub mod metadata;
pub mod pod_ops;
pub mod scope;
pub mod serialization;

mod service;

pub use config::{
    BackupConfig, EncryptionConfig, RetentionPolicy, backup_config_path, load_backup_config,
};
pub use r#loop::BackupLoop;
pub use metadata::{
    PruneReport, RevertReport, SnapshotMetadata, SnapshotTrigger, SpawnAgentReport,
};
pub use pod_ops::{PodBackupCap, PodBackupOps};
pub use scope::ArtifactType;
pub use scope::{BackupScope, ListFilter, RestoreScope};
pub use serialization::{
    ArtifactEnvelopeValue, artifact_git_path, deserialize_artifact, serialize_artifact,
};
pub use service::{BackupError, BackupService};
