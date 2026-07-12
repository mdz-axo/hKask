//! Snapshot and retention policy types for Git CAS.
//!
//! Includes retention tiers, per-repo snapshot policies, snapshot metadata,
//! and the HMemEntry DTO for CAS write-through.

use super::types::RepoId;
use serde::{Deserialize, Serialize};

// ── Retention Policy ───────────────────────────────────────────────────────────

/// A single tier in a graduated retention cascade.
///
/// Snapshots within this tier's age range are kept at the specified interval.
/// Older snapshots that don't satisfy any tier's interval are pruned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasRetentionTier {
    /// Maximum age of snapshots in this tier.
    /// Snapshots older than `max_age` fall into the next tier.
    pub max_age_secs: u64,

    /// Minimum seconds between consecutive snapshots in this tier.
    /// Snapshots taken closer together than this interval are candidates
    /// for pruning (the oldest is kept).
    pub interval_secs: u64,
}

/// Graduated CAS retention policy — cascade of tiers for raw git-level snapshots.
///
/// DISTINCT from `hkask-services-backup::config::RetentionPolicy` (which is
/// calendar-based daily/weekly/monthly for artifact-level pruning). This type
/// controls the frequency of raw CAS git commits via `SnapshotLoop`.
///
/// Default cascade (matches hKask's archival requirements):
/// - 30min intervals for snapshots up to 3 hours old
/// - daily intervals for snapshots up to 3 days old
/// - weekly intervals for snapshots up to 3 weeks old
/// - monthly intervals for everything older
///
/// This produces approximately 6 + 3 + 3 + N ≈ 12+N snapshots per repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasRetentionPolicy {
    /// Ordered list of tiers, from youngest to oldest.
    /// \[NORMATIVE\] The last tier's `max_age_secs` should be `u64::MAX` (forever). (P5 — Essentialism).
    pub tiers: Vec<CasRetentionTier>,
}

impl Default for CasRetentionPolicy {
    fn default() -> Self {
        Self {
            tiers: vec![
                // 0–3 hours: every 30 minutes → ~6 snapshots
                CasRetentionTier {
                    max_age_secs: 3 * 3600,
                    interval_secs: 30 * 60,
                },
                // 3h–3 days: daily → ~3 snapshots
                CasRetentionTier {
                    max_age_secs: 3 * 86400,
                    interval_secs: 86400,
                },
                // 3d–3 weeks: weekly → ~3 snapshots
                CasRetentionTier {
                    max_age_secs: 3 * 7 * 86400,
                    interval_secs: 7 * 86400,
                },
                // 3w+: monthly (end-of-month) → indefinite retention
                CasRetentionTier {
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
    /// CAS retention policy for this repo. Uses the global default if None.
    pub policy: Option<CasRetentionPolicy>,
}

impl RepoSnapshotPolicy {
    /// Create a policy for a repo with default retention.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  repo is any [`RepoId`] variant
    /// post: returns a [`RepoSnapshotPolicy`] with `enabled: true` and `policy: None`
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  repo is any [`RepoId`] variant; policy is any [`CasRetentionPolicy`]
    /// post: returns a [`RepoSnapshotPolicy`] with `enabled: true` and the given custom policy
    pub fn with_policy(repo: RepoId, policy: CasRetentionPolicy) -> Self {
        Self {
            repo,
            enabled: true,
            policy: Some(policy),
        }
    }

    /// Create a disabled policy for a repo that shouldn't be snapshotted.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  repo is any [`RepoId`] variant
    /// post: returns a [`RepoSnapshotPolicy`] with `enabled: false` and `policy: None`
    pub fn disabled(repo: RepoId) -> Self {
        Self {
            repo,
            enabled: false,
            policy: None,
        }
    }

    /// Get the effective retention policy, falling back to default.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`RepoSnapshotPolicy`]
    /// post: returns the custom [`CasRetentionPolicy`] if `policy` is `Some`;
    ///       otherwise returns the global default [`CasRetentionPolicy`]; never panics
    pub fn effective_policy(&self) -> CasRetentionPolicy {
        self.policy.clone().unwrap_or_default()
    }
}

// ── HMem Entry DTO ────────────────────────────────────────────────────────

/// Serializable representation of `HMem` from `hkask-storage`.
///
/// `HMem` in `hkask-storage` does not derive `Serialize`, so this DTO
/// captures the same fields in a serializable form for CAS write-through.
/// The `value` field is already a `serde_json::Value`, which serializes natively.
///
/// Construct via `HMemEntry::from(&h_mem)` in `hkask-storage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HMemEntry {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: serde_json::Value,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub confidence: f64,
    pub perspective: String,
    pub visibility: String,
    pub dimension: Option<String>,
}
