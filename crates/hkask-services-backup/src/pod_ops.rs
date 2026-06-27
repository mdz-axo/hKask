//! PodBackupOps — pod-level revert and spawn_agent on top of the CAS port.
//!
//! Extracted from `BackupService` to keep the backup service focused on
//! artifact-level operations. PodBackupOps handles pod.db I/O, blob
//! disambiguation (envelope vs raw), and atomic writes.
//!
//! Shares the mutual-exclusion gate with `BackupService` — a revert cannot
//! run concurrently with a snapshot or another revert.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;

use chrono::Utc;
use hkask_ports::git_cas::{CommitHash, GitCASPort};
use tracing::{info, instrument};

use crate::metadata::{RevertReport, SnapshotMetadata, SnapshotTrigger, SpawnAgentReport};
use crate::scope::ArtifactType;
use crate::service::{BackupError, decrypt_blob, encrypt_blob};

/// Pod-level backup operations — revert and spawn_agent.
///
/// Wraps a [`GitCASPort`] directly (not through `BackupService`)
/// because pod operations need CAS primitives but not artifact-level
/// scoping, retention, or verification.
pub struct PodBackupOps {
    pub(crate) cas: Arc<dyn GitCASPort>,
    pub(crate) encryption_key: Option<[u8; 32]>,
    gate: Arc<AtomicBool>,
}

impl PodBackupOps {
    /// Create from a CAS port, encryption key, and shared gate.
    ///
    /// The gate should be cloned from `BackupService::gate()` to ensure
    /// mutual exclusion between artifact backups and pod operations.
    pub fn new(
        cas: Arc<dyn GitCASPort>,
        encryption_key: Option<[u8; 32]>,
        gate: Arc<AtomicBool>,
    ) -> Self {
        Self {
            cas,
            encryption_key,
            gate,
        }
    }

    fn acquire_gate(&self) -> Result<(), BackupError> {
        if self
            .gate
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(BackupError::BackupInProgress);
        }
        Ok(())
    }

    pub(crate) fn release_gate(&self) {
        self.gate.store(false, Ordering::Release);
    }

    /// Snapshot a pod's current state (used by PodBackupCap).
    /// Gated — cannot run concurrently with other backup operations.
    pub(crate) async fn snapshot_pod(
        &self,
        pod_id: &str,
        pod_db_path: &Path,
    ) -> Result<SnapshotMetadata, BackupError> {
        self.acquire_gate()?;
        let _guard = PodGateGuard { ops: self };
        let start = Instant::now();

        if !pod_db_path.exists() {
            return Err(BackupError::PodNotFound(pod_id.to_string()));
        }
        let pod_data = std::fs::read(pod_db_path)
            .map_err(|e| BackupError::Config(format!("Failed to read pod.db: {e}")))?;

        let artifact = crate::serialization::serialize_artifact(
            &ArtifactType::PodState,
            pod_id,
            &serde_json::json!({"pod_id": pod_id}),
        )
        .map_err(|e| BackupError::Serialization(format!("Pod snapshot: {e}")))?;

        let repo_id = ArtifactType::PodState.repo_id();
        let envelope_blob = if self.encryption_key.is_some() {
            let aad = format!("pod_state/{}", pod_id).into_bytes();
            encrypt_blob(&self.encryption_key, &artifact, &aad)?
        } else {
            artifact
        };
        let raw_blob = if self.encryption_key.is_some() {
            let aad = format!("pod_state/{}-raw", pod_id).into_bytes();
            encrypt_blob(&self.encryption_key, &pod_data, &aad)?
        } else {
            pod_data
        };

        self.cas.put_blob(&repo_id, &envelope_blob).await?;
        self.cas.put_blob(&repo_id, &raw_blob).await?;
        let commit = self
            .cas
            .snapshot(
                &repo_id,
                &format!(
                    "pod snapshot: {pod_id} — {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S")
                ),
            )
            .await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "snapshot",
            commit = %commit,
            duration_ms = duration_ms,
            "CNS"
        );

        Ok(SnapshotMetadata {
            commits: vec![(repo_id, commit)],
            artifact_count: Some(1),
            trigger: Some(SnapshotTrigger::Manual),
            timestamp: Utc::now(),
        })
    }

    /// Revert a pod to a prior snapshot.
    ///
    /// Takes a safety snapshot of the current pod state before restoring
    /// the target commit. The safety snapshot is the bail-out point.
    ///
    /// **Restart protocol:** After revert writes the restored pod.db, the
    /// caller MUST:
    /// 1. Signal the pod to shut down gracefully
    /// 2. Wait for the pod to exit (the pod has the old state in memory)
    /// 3. Restart the pod (it will read the restored pod.db)
    ///
    /// The revert only replaces the database file — it does NOT restart
    /// the running pod process. The pod will continue running with its
    /// pre-revert in-memory state until restarted.
    ///
    /// CNS span: `cns.agent_pod.revert`
    #[instrument(skip(self), fields(pod_id, safety_commit, target_commit))]
    pub async fn revert(
        &self,
        pod_id: &str,
        target_commit: &CommitHash,
        pod_db_path: &Path,
        reason: &str,
    ) -> Result<RevertReport, BackupError> {
        self.acquire_gate()?;
        let _guard = PodGateGuard { ops: self };
        let start = Instant::now();

        // 1. Safety snapshot: capture current pod state BEFORE revert
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "revert.safety_snapshot",
            reason = reason,
            "CNS"
        );

        if !pod_db_path.exists() {
            return Err(BackupError::PodNotFound(pod_id.to_string()));
        }
        let current_state = std::fs::read(pod_db_path)
            .map_err(|e| BackupError::Config(format!("Failed to read pod.db: {e}")))?;

        let safety_artifact = crate::serialization::serialize_artifact(
            &ArtifactType::PodState,
            &format!("{pod_id}-safety"),
            &serde_json::json!({"pod_id": pod_id, "reason": reason}),
        )
        .map_err(|e| BackupError::Serialization(format!("Safety snapshot: {e}")))?;

        let safety_blob = if self.encryption_key.is_some() {
            let aad = format!("pod_state/{}-safety", pod_id).into_bytes();
            encrypt_blob(&self.encryption_key, &safety_artifact, &aad)?
        } else {
            safety_artifact
        };

        let repo_id = ArtifactType::PodState.repo_id();
        self.cas.put_blob(&repo_id, &safety_blob).await?;
        let pod_blob = if self.encryption_key.is_some() {
            let aad = format!("pod_state/{}-safety-raw", pod_id).into_bytes();
            encrypt_blob(&self.encryption_key, &current_state, &aad)?
        } else {
            current_state
        };
        self.cas.put_blob(&repo_id, &pod_blob).await?;
        let safety_commit = self
            .cas
            .snapshot(
                &repo_id,
                &format!(
                    "revert: safety snapshot for {pod_id} — {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S")
                ),
            )
            .await?;

        tracing::Span::current().record("safety_commit", safety_commit.to_string());
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "revert.safety_complete",
            safety_commit = %safety_commit,
            "CNS"
        );

        // 2. Restore target state to pod.db
        let target_str = target_commit.to_string();
        let prefix = format!("{}/", ArtifactType::PodState.label());
        let entries = self.cas.list_tree(&repo_id, &target_str, &prefix).await?;
        let blob = find_raw_pod_blob(&self.cas, &self.encryption_key, &repo_id, &entries).await?;
        let tmp = pod_db_path.with_extension("pod.db.tmp");
        std::fs::write(&tmp, &blob)
            .map_err(|e| BackupError::Config(format!("Failed to write pod.db: {e}")))?;
        std::fs::rename(&tmp, pod_db_path)
            .map_err(|e| BackupError::Config(format!("Failed to commit pod.db: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::Span::current().record("target_commit", target_str);
        tracing::Span::current().record("pod_id", pod_id);
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "revert",
            safety_commit = %safety_commit,
            target_commit = %target_commit,
            reason = reason,
            duration_ms = duration_ms,
            "CNS"
        );

        Ok(RevertReport {
            pod_id: pod_id.to_string(),
            safety_commit,
            target_commit: target_commit.clone(),
            artifact_count: 1,
            timestamp: Utc::now(),
        })
    }

    /// Spawn a new agent pod from a prior snapshot.
    ///
    /// Restores pod state from a source commit to a new pod location.
    /// DISTINCT from kanban's `spawn_subagent`.
    ///
    /// CNS span: `cns.agent_pod.spawn`
    #[instrument(skip(self), fields(source_pod_id, new_pod_id, source_commit))]
    pub async fn spawn_agent(
        &self,
        source_pod_id: &str,
        source_commit: &CommitHash,
        new_pod_id: &str,
        output_db_path: &Path,
    ) -> Result<SpawnAgentReport, BackupError> {
        self.acquire_gate()?;
        let _guard = PodGateGuard { ops: self };
        let start = Instant::now();
        let repo_id = ArtifactType::PodState.repo_id();
        let target_str = source_commit.to_string();
        let prefix = format!("{}/", ArtifactType::PodState.label());
        let entries = self.cas.list_tree(&repo_id, &target_str, &prefix).await?;
        let blob = find_raw_pod_blob(&self.cas, &self.encryption_key, &repo_id, &entries).await?;
        let tmp = output_db_path.with_extension("pod.db.tmp");
        std::fs::write(&tmp, &blob)
            .map_err(|e| BackupError::Config(format!("Failed to write new pod.db: {e}")))?;
        std::fs::rename(&tmp, output_db_path)
            .map_err(|e| BackupError::Config(format!("Failed to commit new pod.db: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::Span::current().record("source_pod_id", source_pod_id);
        tracing::Span::current().record("new_pod_id", new_pod_id);
        tracing::Span::current().record("source_commit", target_str);
        info!(
            target: "cns.agent_pod",
            operation = "spawn",
            source_pod_id = source_pod_id,
            new_pod_id = new_pod_id,
            source_commit = %source_commit,
            duration_ms = duration_ms,
            "CNS"
        );

        Ok(SpawnAgentReport {
            source_pod_id: source_pod_id.to_string(),
            new_pod_id: new_pod_id.to_string(),
            source_commit: source_commit.clone(),
            new_db_path: output_db_path.to_string_lossy().to_string(),
            timestamp: Utc::now(),
        })
    }
}

/// Find the first raw pod.db blob among tree entries (skip JSON envelopes).
async fn find_raw_pod_blob(
    cas: &Arc<dyn GitCASPort>,
    encryption_key: &Option<[u8; 32]>,
    repo_id: &hkask_ports::git_cas::RepoId,
    entries: &[hkask_ports::git_cas::TreeEntry],
) -> Result<Vec<u8>, BackupError> {
    for entry in entries {
        let raw = cas.get_blob(repo_id, &entry.content_hash).await?;
        let blob = if encryption_key.is_some() {
            decrypt_blob(encryption_key, &raw, &[])?
        } else {
            raw
        };
        if serde_json::from_slice::<crate::serialization::ArtifactEnvelopeValue>(&blob).is_err() {
            return Ok(blob);
        }
    }
    Err(BackupError::NoSnapshots)
}

/// RAII guard that releases the pod backup gate on drop.
struct PodGateGuard<'a> {
    ops: &'a PodBackupOps,
}
impl Drop for PodGateGuard<'_> {
    fn drop(&mut self) {
        self.ops.release_gate();
    }
}

/// Per-pod backup capability — unforgeable reference to backup a specific pod.
///
/// The capability IS the authorization (Miller OCAP pattern). You cannot
/// backup another pod because you don't have its `PodBackupCap`.
///
/// Constructed at the call site where both `PodDeployment` and `BackupService`
/// are available:
///
/// ```ignore
/// let cap = PodBackupCap::new(
///     deployment.pod_id.to_string(),
///     deployment.storage.db_path.clone(),
///     backup_service.pod_ops(),
/// );
/// cap.snapshot().await?;
/// ```
pub struct PodBackupCap {
    pod_id: String,
    pod_db_path: PathBuf,
    ops: PodBackupOps,
}

impl PodBackupCap {
    /// Create a scoped backup capability for a specific pod.
    ///
    /// The `ops` should share the same gate as `BackupService` to ensure
    /// mutual exclusion between artifact and pod backups.
    pub fn new(pod_id: impl Into<String>, pod_db_path: PathBuf, ops: PodBackupOps) -> Self {
        Self {
            pod_id: pod_id.into(),
            pod_db_path,
            ops,
        }
    }

    /// Snapshot this pod's current state to the Pods repo.
    ///
    /// Delegates to `PodBackupOps::snapshot_pod` with this pod's identity
    /// and database path. The commit hash can be used later for revert or
    /// spawn_agent.
    ///
    /// CNS span: `cns.agent_pod.snapshot`
    pub async fn snapshot(&self) -> Result<SnapshotMetadata, BackupError> {
        self.ops.snapshot_pod(&self.pod_id, &self.pod_db_path).await
    }

    /// Revert this pod to a prior snapshot.
    ///
    /// Delegates to [`PodBackupOps::revert`] with this pod's identity and
    /// database path. Takes a safety snapshot before restoring.
    pub async fn revert(
        &self,
        target_commit: &CommitHash,
        reason: &str,
    ) -> Result<RevertReport, BackupError> {
        self.ops
            .revert(&self.pod_id, target_commit, &self.pod_db_path, reason)
            .await
    }

    /// Spawn a new agent from this pod's state snapshot.
    ///
    /// Delegates to [`PodBackupOps::spawn_agent`] with this pod as the source.
    pub async fn spawn_agent(
        &self,
        source_commit: &CommitHash,
        new_pod_id: &str,
        output_db_path: &Path,
    ) -> Result<SpawnAgentReport, BackupError> {
        self.ops
            .spawn_agent(&self.pod_id, source_commit, new_pod_id, output_db_path)
            .await
    }
}
