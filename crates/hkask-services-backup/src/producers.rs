//! Artifact producers — push current subsystem state into CAS for backup.
//!
//! Each producer knows how to collect state for specific [`ArtifactType`]s
//! and push blobs to a [`GitCASPort`]. Called by [`BackupLoop`] before
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
#[async_trait]
pub trait ArtifactProducer: Send + Sync {
    /// Which artifact types this producer handles.
    fn artifact_types(&self) -> &[ArtifactType];

    /// Collect current state and push blobs to CAS.
    /// Returns the number of artifacts produced.
    async fn produce(&self, cas: &dyn GitCASPort) -> Result<usize, BackupError>;
}

/// A producer that snapshots active pod databases.
///
/// Reads each pod's `pod.db` file and pushes it as a `PodState` artifact.
/// This enables revert and spawn_agent operations.
pub struct PodStateProducer {
    /// Paths to active pod database files, keyed by pod name.
    pods: Vec<(String, std::path::PathBuf)>,
}

impl PodStateProducer {
    /// Create from a list of (pod_name, pod_db_path) pairs.
    pub fn new(pods: Vec<(String, std::path::PathBuf)>) -> Self {
        Self { pods }
    }
}

#[async_trait]
impl ArtifactProducer for PodStateProducer {
    fn artifact_types(&self) -> &[ArtifactType] {
        &[ArtifactType::PodState]
    }

    async fn produce(&self, cas: &dyn GitCASPort) -> Result<usize, BackupError> {
        use crate::serialization::serialize_artifact;
        use ArtifactType::PodState;

        let repo_id = PodState.repo_id();
        let mut count = 0usize;

        for (pod_id, db_path) in &self.pods {
            if !db_path.exists() {
                tracing::warn!(
                    target: "cns.backup",
                    pod_id = %pod_id,
                    "Pod database not found — skipping backup"
                );
                continue;
            }

            let pod_data = std::fs::read(db_path).map_err(|e| {
                BackupError::Config(format!("Failed to read pod.db for {pod_id}: {e}"))
            })?;

            // Store envelope + raw blob (same format as revert safety snapshot)
            let artifact =
                serialize_artifact(&PodState, pod_id, &serde_json::json!({"pod_id": pod_id}))
                    .map_err(|e| {
                        BackupError::Serialization(format!("PodState serialization: {e}"))
                    })?;

            cas.put_blob(&repo_id, &artifact).await?;
            cas.put_blob(&repo_id, &pod_data).await?;
            count += 1;
        }

        Ok(count)
    }
}
