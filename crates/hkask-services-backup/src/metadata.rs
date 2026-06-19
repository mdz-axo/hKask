//! Backup metadata types — snapshot results, prune reports.
//! # REQ: P8 (Semantic Grounding) — every type encodes a distinct domain concept.

use chrono::{DateTime, Utc};
use hkask_types::ports::git_cas::{CommitHash, RepoId};
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
    pub artifact_count: usize,
    /// What triggered this snapshot.
    pub trigger: SnapshotTrigger,
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
