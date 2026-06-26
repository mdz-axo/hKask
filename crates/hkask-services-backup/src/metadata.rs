//! Backup metadata types — snapshot results, prune reports, revert/spawn_agent reports.
//! # REQ: P8 (Semantic Grounding) — every type encodes a distinct domain concept.
//! expect: "Backup metadata types encode distinct domain concepts"

use chrono::{DateTime, Utc};
use hkask_ports::git_cas::{CommitHash, RepoId};
use serde::{Deserialize, Serialize};

/// What triggered a backup snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotTrigger {
    /// User-initiated via CLI or API.
    Manual,
    /// Auto-snapshot triggered by artifact mutation.
    Auto,
    /// CNS-triggered (e.g., pre-consolidation safety snapshot).
    CnsTriggered,
    /// Safety snapshot — taken automatically before a revert operation.
    SafetySnapshot,
}

/// Metadata about a completed backup snapshot.
///
/// Returned by [`super::BackupService::snapshot`]. Enriches the raw
/// commit hash with operational context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Commit hashes produced, keyed by repository.
    pub commits: Vec<(RepoId, CommitHash)>,
    /// Total number of artifacts included in this snapshot.
    /// `None` when reconstructed from log (git doesn't carry this).
    pub artifact_count: Option<usize>,
    /// What triggered this snapshot.
    /// `None` when reconstructed from log (git doesn't carry this).
    pub trigger: Option<SnapshotTrigger>,
    /// When the snapshot was created.
    pub timestamp: DateTime<Utc>,
}

/// Report from a prune operation.
///
/// Returned by [`super::BackupService::prune`]. Lists what was
/// evaluated and what would be (or was) removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    /// Whether this was a dry run (no actual deletion).
    pub dry_run: bool,
    /// Number of snapshots evaluated against the retention policy.
    pub evaluated: usize,
    /// Commit hashes that were (or would be) removed.
    pub removed: Vec<(RepoId, CommitHash)>,
    /// Commit hashes retained.
    pub retained: usize,
}

/// Report from a pod revert operation.
///
/// Returned by `super::BackupService::revert`. Records the safety
/// snapshot taken before the revert and the restored target commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertReport {
    /// The pod identifier that was reverted.
    pub pod_id: String,
    /// Commit hash of the safety snapshot taken before the revert.
    /// This IS the bail-out point — restore this commit to undo the revert.
    pub safety_commit: CommitHash,
    /// Commit hash that the pod was restored to.
    pub target_commit: CommitHash,
    /// Number of artifacts restored.
    pub artifact_count: usize,
    /// When the revert was executed.
    pub timestamp: DateTime<Utc>,
}

/// Report from a spawn_agent operation.
///
/// Returned by `super::BackupService::spawn_agent`. Records the new
/// pod created from a prior state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentReport {
    /// The source pod identifier (whose state was cloned).
    pub source_pod_id: String,
    /// The new pod identifier.
    pub new_pod_id: String,
    /// The source commit hash that the new pod was spawned from.
    pub source_commit: CommitHash,
    /// Path to the new pod's database file.
    pub new_db_path: String,
    /// When the spawn was executed.
    pub timestamp: DateTime<Utc>,
}
