//! Snapshot and retention policy types for Git CAS.
//!
//! Includes retention tiers, per-repo snapshot policies, snapshot metadata,
//! and the TripleEntry DTO for CAS write-through.

use super::types::{CommitHash, RepoId};
use serde::{Deserialize, Serialize};

// ── Retention Policy ───────────────────────────────────────────────────────────

/// A single tier in a graduated retention cascade.
///
/// Snapshots within this tier's age range are kept at the specified interval.
/// Older snapshots that don't satisfy any tier's interval are pruned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionTier {
    /// Maximum age of snapshots in this tier.
    /// Snapshots older than `max_age` fall into the next tier.
    pub max_age_secs: u64,

    /// Minimum seconds between consecutive snapshots in this tier.
    /// Snapshots taken closer together than this interval are candidates
    /// for pruning (the oldest is kept).
    pub interval_secs: u64,
}

/// Graduated retention policy — cascade of tiers.
///
/// Default cascade (matches hKask's archival requirements):
/// - 30min intervals for snapshots up to 3 hours old
/// - daily intervals for snapshots up to 3 days old
/// - weekly intervals for snapshots up to 3 weeks old
/// - monthly intervals for everything older
///
/// This produces approximately 6 + 3 + 3 + N ≈ 12+N snapshots per repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Ordered list of tiers, from youngest to oldest.
    pub tiers: Vec<RetentionTier>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            tiers: vec![
                // 0–3 hours: every 30 minutes → ~6 snapshots
                RetentionTier {
                    max_age_secs: 3 * 3600,
                    interval_secs: 30 * 60,
                },
                // 3h–3 days: daily → ~3 snapshots
                RetentionTier {
                    max_age_secs: 3 * 86400,
                    interval_secs: 86400,
                },
                // 3d–3 weeks: weekly → ~3 snapshots
                RetentionTier {
                    max_age_secs: 3 * 7 * 86400,
                    interval_secs: 7 * 86400,
                },
                // 3w+: monthly (end-of-month) → indefinite retention
                RetentionTier {
                    max_age_secs: u64::MAX,
                    interval_secs: 30 * 86400,
                },
            ],
        }
    }
}

/// Per-repo snapshot policy — customizes frequency and retention per repo.
///
/// Each repo can override the global policy. Repos that change frequently
/// (CnsAudit, Registry) snapshot more often; stable repos (Vault) less often.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoSnapshotPolicy {
    /// Which repo this policy applies to.
    pub repo: RepoId,
    /// Whether snapshotting is enabled for this repo.
    pub enabled: bool,
    /// Retention policy for this repo. Uses the global default if None.
    pub policy: Option<RetentionPolicy>,
}

impl RepoSnapshotPolicy {
    /// Create a policy for a repo with default retention.
    ///
    ///       (falls back to global default retention)
    pub fn default_for(repo: RepoId) -> Self {
        Self {
            repo,
            enabled: true,
            policy: None,
        }
    }

    /// Create a policy for a repo with custom retention.
    ///
    pub fn with_policy(repo: RepoId, policy: RetentionPolicy) -> Self {
        Self {
            repo,
            enabled: true,
            policy: Some(policy),
        }
    }

    /// Create a disabled policy for a repo that shouldn't be snapshotted.
    ///
    pub fn disabled(repo: RepoId) -> Self {
        Self {
            repo,
            enabled: false,
            policy: None,
        }
    }

    /// Get the effective retention policy, falling back to default.
    ///
    ///       otherwise returns the global default [`RetentionPolicy`]; never panics
    pub fn effective_policy(&self) -> RetentionPolicy {
        self.policy.clone().unwrap_or_default()
    }
}

/// Snapshot metadata — recorded for each snapshot taken.
///
/// Stored alongside the git commit to enable pruning and rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// The commit hash of this snapshot.
    pub commit: CommitHash,
    /// Which repo was snapshotted.
    pub repo: RepoId,
    /// Commit message.
    pub message: String,
    /// Timestamp (Unix epoch seconds) when the snapshot was taken.
    pub timestamp_secs: u64,
    /// What triggered this snapshot (manual, scheduled, cns-triggered).
    pub trigger: SnapshotTrigger,
}

/// What triggered a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotTrigger {
    /// Manually triggered via `kask git snapshot`.
    Manual,
    /// Scheduled by the SnapshotLoop based on RetentionPolicy interval.
    Scheduled,
    /// Triggered by CNS variety deficit or algedonic alert.
    CnsTriggered,
}

// ── Triple Entry DTO ────────────────────────────────────────────────────────

/// Serializable representation of `Triple` from `hkask-storage`.
///
/// `Triple` in `hkask-storage` does not derive `Serialize`, so this DTO
/// captures the same fields in a serializable form for CAS write-through.
/// The `value` field is already a `serde_json::Value`, which serializes natively.
///
/// Construct via `TripleEntry::from(&triple)` in `hkask-storage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripleEntry {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: serde_json::Value,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub confidence: f64,
    pub perspective: String,
    pub visibility: String,
}
