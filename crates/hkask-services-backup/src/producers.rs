//! Artifact producers — push current subsystem state into CAS for backup.
//!
//! Each producer knows how to collect state for specific [`ArtifactType`]s
//! and push blobs to a [`GitCASPort`]. Called by `BackupLoop` before
//! the daily snapshot to ensure the CAS repos contain current data.

use async_trait::async_trait;
use hkask_ports::git_cas::GitCASPort;

use crate::scope::ArtifactType;
use crate::service::BackupError;

/// A subsystem that can produce artifact blobs for backup.
///
/// Implementations collect current state from their owning subsystem
/// (registry, memory store, pod manager, etc.) and push serialized blobs
/// to the CAS port. Called by [`super::BackupLoop`] before each daily snapshot.
///
/// Canonical implementations live in `hkask-services-context::context_impl`
/// where they have access to the subsystem state they need to produce.
#[async_trait]
pub trait ArtifactProducer: Send + Sync {
    /// Which artifact types this producer handles.
    fn artifact_types(&self) -> &[ArtifactType];

    /// Collect current state and push blobs to CAS.
    /// Returns the number of artifacts produced.
    async fn produce(&self, cas: &dyn GitCASPort) -> Result<usize, BackupError>;
}
