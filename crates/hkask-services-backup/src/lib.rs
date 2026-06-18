//! hKask Backup Service — policy layer on top of GitCASPort.
//!
//! Extracted from `hkask-services` to enable parallel compilation.
//! Public API: `BackupService`, `BackupError`, and supporting types.
//!
//! # REQ: P1 (User Sovereignty) — user controls what is tracked.
//! # expect: "My service operations flow through sovereignty-verifying boundaries" [P1]
//! # REQ: P4 (Clear Boundaries) — delegates to hexagonal GitCASPort, never raw git.
//! # expect: "Service boundaries enforce OCAP membranes" [P4]

pub mod config;
pub mod r#loop;
pub mod metadata;
pub mod scope;
pub mod serialization;

mod service;

pub use config::{
    BackupConfig, EncryptionConfig, RetentionPolicy, backup_config_path, load_backup_config,
};
pub use r#loop::BackupLoop;
pub use metadata::{PruneReport, SnapshotMetadata, SnapshotTrigger};
pub use scope::ArtifactType;
pub use scope::{BackupScope, ListFilter, RestoreScope};
pub use serialization::{
    ArtifactEnvelopeValue, artifact_git_path, deserialize_artifact, serialize_artifact,
};
pub use service::{BackupError, BackupService};
