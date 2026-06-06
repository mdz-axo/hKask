//! Git CAS Port — Hexagonal boundary for content-addressable git storage
//!
//! Defines the trait and value types for a content-addressed git storage system.
//! Content is addressed by BLAKE3 hash. Snapshots are git commits.
//!
//! Each method operates on a named repository (`RepoId`), providing isolation
//! between the 7 snapshot repos.

use crate::blake3_hash;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;

// ── Value Types ──────────────────────────────────────────────────────────────

/// BLAKE3 content hash — 32 bytes, displayed as hex.
///
/// Addresses blob content within a CAS repository. Produced by
/// [`ContentHash::from_blake3`] which wraps [`crate::blake3_hash`].
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Compute a BLAKE3 content hash from arbitrary data.
    pub fn from_blake3(data: &[u8]) -> Self {
        Self(blake3_hash(data))
    }

    /// Return the raw 32-byte hash.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", hex::encode(self.0))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl std::str::FromStr for ContentHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("expected 32 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Git commit SHA — 20 bytes, displayed as hex.
///
/// Addresses a snapshot commit within a CAS repository.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitHash(pub [u8; 20]);

impl CommitHash {
    /// Create from a raw 20-byte SHA.
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Return the raw 20-byte SHA.
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// The null commit hash (all zeros), used as a sentinel for "no parent".
    pub fn null() -> Self {
        Self([0u8; 20])
    }
}

impl fmt::Debug for CommitHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommitHash({})", hex::encode(self.0))
    }
}

impl fmt::Display for CommitHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl std::str::FromStr for CommitHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 20 {
            return Err(format!("expected 20 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Repository identifier — one of the 7 snapshot repos.
///
/// Each variant names a distinct git repository that stores a specific
/// category of hKask state. Repos are isolated from each other.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepoId {
    /// Agent registry (templates, personas, dispatch manifests)
    Registry,
    /// Semantic memory (triples, knowledge graph)
    Memory,
    /// CNS audit trail (ν-events, variety counters, algedonic alerts)
    CnsAudit,
    /// User sovereignty (consent records, OCAP tokens)
    Sovereignty,
    /// Goals and specifications
    GoalsSpecs,
    /// Standing sessions (conversation history)
    Sessions,
    /// Vault (encrypted master key material)
    Vault,
}

impl RepoId {
    /// Return the directory name used for this repo on disk.
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::Memory => "memory",
            Self::CnsAudit => "cns-audit",
            Self::Sovereignty => "sovereignty",
            Self::GoalsSpecs => "goals-specs",
            Self::Sessions => "sessions",
            Self::Vault => "vault",
        }
    }

    /// Iterate all 7 repo variants.
    pub fn all() -> &'static [RepoId] {
        &[
            RepoId::Registry,
            RepoId::Memory,
            RepoId::CnsAudit,
            RepoId::Sovereignty,
            RepoId::GoalsSpecs,
            RepoId::Sessions,
            RepoId::Vault,
        ]
    }
}

// ── CAS Domain Types ─────────────────────────────────────────────────────────

/// A file entry in a git tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// File path relative to repo root.
    pub path: String,
    /// BLAKE3 content hash of the file content.
    pub content_hash: ContentHash,
    /// Whether this is a blob (file) or tree (directory).
    pub kind: TreeEntryKind,
}

/// Kind of tree entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreeEntryKind {
    Blob,
    Tree,
}

/// A file diff between two commits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// File path relative to repo root.
    pub path: String,
    /// Kind of change.
    pub kind: DiffKind,
    /// Unified diff content.
    pub content: String,
}

/// Kind of file change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
}

/// Verification report — integrity check results.
///
/// After calling [`GitCASPort::verify`], this report lists the total number
/// of blobs checked, how many passed, and which content hashes failed integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Which repo was verified.
    pub repo: RepoId,
    /// Total blobs in the repo.
    pub total_blobs: usize,
    /// Blobs whose content matched their stored hash.
    pub verified_blobs: usize,
    /// Content hashes where re-hashing produced a different digest.
    pub corrupt_hashes: Vec<ContentHash>,
}

/// A single entry in the snapshot log.
///
/// Returned by [`GitCASPort::log`], each entry represents a past snapshot
/// commit in the repository's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// The commit hash of this snapshot.
    pub commit: CommitHash,
    /// The commit message.
    pub message: String,
    /// Unix timestamp (seconds) when the snapshot was taken.
    pub timestamp_secs: u64,
}

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
    /// The last tier's `max_age_secs` should be `u64::MAX` (forever).
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
    pub fn default_for(repo: RepoId) -> Self {
        Self {
            repo,
            enabled: true,
            policy: None,
        }
    }

    /// Create a policy for a repo with custom retention.
    pub fn with_policy(repo: RepoId, policy: RetentionPolicy) -> Self {
        Self {
            repo,
            enabled: true,
            policy: Some(policy),
        }
    }

    /// Create a disabled policy for a repo that shouldn't be snapshotted.
    pub fn disabled(repo: RepoId) -> Self {
        Self {
            repo,
            enabled: false,
            policy: None,
        }
    }

    /// Get the effective retention policy, falling back to default.
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

/// Serializable representation of a [`Triple`](hkask_storage::Triple).
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

// ── Error Type ───────────────────────────────────────────────────────────────

/// Errors from Git CAS port operations.
///
/// Each variant has a distinct recovery path (C5: every error variant = unique recovery):
/// - `CrateNotFound` → create the repo
/// - `Io` → retry or check filesystem permissions
/// - `Git` → inspect git state, possibly reinitialize
/// - `PathValidation` → reject the request, possible attack
/// - `ContentHashMismatch` → re-download or restore from backup
/// - `NotFound` → create the blob first, or check the hash
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitCasError {
    #[error("Crate not found: {0}")]
    CrateNotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Path validation error: {0}")]
    PathValidation(String),

    #[error("Content hash mismatch: expected {expected}, got {actual}")]
    ContentHashMismatch { expected: String, actual: String },

    #[error("Not found: {0}")]
    NotFound(String),
}

// ── Hexagonal Port ───────────────────────────────────────────────────────────

/// Hexagonal port for content-addressed git storage.
///
/// Each method operates on a named repository ([`RepoId`]).
/// Content is addressed by BLAKE3 hash. Snapshots are git commits.
///
/// Implementations:
/// - `GixCasAdapter` (production, in `hkask-mcp`)
/// - [`MockGitCas`] (testing, in this module)
#[async_trait]
pub trait GitCASPort: Send + Sync {
    /// Store content, returning its BLAKE3 content hash.
    async fn put_blob(&self, repo: &RepoId, content: &[u8]) -> Result<ContentHash, GitCasError>;

    /// Retrieve content by its BLAKE3 hash.
    async fn get_blob(&self, repo: &RepoId, hash: &ContentHash) -> Result<Vec<u8>, GitCasError>;

    /// Create a snapshot commit of all staged changes.
    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError>;

    /// Resolve a symbolic ref (branch, tag) to a commit SHA.
    async fn resolve_ref(&self, repo: &RepoId, reference: &str) -> Result<CommitHash, GitCasError>;

    /// List file paths at a given ref with their content hashes.
    async fn list_tree(
        &self,
        repo: &RepoId,
        reference: &str,
        prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError>;

    /// Diff two commits.
    async fn diff(&self, repo: &RepoId, from: &str, to: &str)
    -> Result<Vec<FileDiff>, GitCasError>;

    /// Verify content integrity: re-hash all blobs, compare to stored hashes.
    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError>;

    /// List snapshot history for a repo.
    ///
    /// Returns commit entries from newest to oldest, up to `max_count`.
    async fn log(&self, repo: &RepoId, max_count: usize) -> Result<Vec<LogEntry>, GitCasError>;
}

// ── MockGitCas (test helper) ─────────────────────────────────────────────────

/// In-memory mock implementation of [`GitCASPort`] for testing.
///
/// Stores blobs in a `HashMap` and snapshots in a `Vec`. Does not
/// perform any real git operations. Useful for unit tests where a
/// real git repository is unnecessary.
pub struct MockGitCas {
    blobs: RwLock<HashMap<ContentHash, Vec<u8>>>,
    snapshots: RwLock<Vec<(RepoId, String, CommitHash)>>,
}

impl MockGitCas {
    /// Create a new empty mock.
    pub fn new() -> Self {
        Self {
            blobs: RwLock::new(HashMap::new()),
            snapshots: RwLock::new(Vec::new()),
        }
    }

    /// Return the history of snapshot calls as `(repo, message, commit_hash)`.
    pub fn snapshot_history(&self) -> Vec<(RepoId, String, CommitHash)> {
        self.snapshots.read().unwrap().clone()
    }
}

impl Default for MockGitCas {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitCASPort for MockGitCas {
    async fn put_blob(&self, _repo: &RepoId, content: &[u8]) -> Result<ContentHash, GitCasError> {
        let hash = ContentHash::from_blake3(content);
        self.blobs
            .write()
            .unwrap()
            .insert(hash.clone(), content.to_vec());
        Ok(hash)
    }

    async fn get_blob(&self, _repo: &RepoId, hash: &ContentHash) -> Result<Vec<u8>, GitCasError> {
        self.blobs
            .read()
            .unwrap()
            .get(hash)
            .cloned()
            .ok_or_else(|| GitCasError::NotFound(hash.to_string()))
    }

    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError> {
        // Generate a deterministic commit hash from the message
        let hash_bytes = blake3_hash(message.as_bytes());
        let mut commit_bytes = [0u8; 20];
        commit_bytes.copy_from_slice(&hash_bytes[..20]);
        let commit = CommitHash::from_bytes(commit_bytes);

        self.snapshots
            .write()
            .unwrap()
            .push((repo.clone(), message.to_string(), commit.clone()));
        Ok(commit)
    }

    async fn resolve_ref(
        &self,
        _repo: &RepoId,
        _reference: &str,
    ) -> Result<CommitHash, GitCasError> {
        let snapshots = self.snapshots.read().unwrap();
        snapshots
            .last()
            .map(|(_, _, hash)| hash.clone())
            .ok_or_else(|| GitCasError::NotFound("no snapshots".to_string()))
    }

    async fn list_tree(
        &self,
        _repo: &RepoId,
        _reference: &str,
        _prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError> {
        let blobs = self.blobs.read().unwrap();
        let mut entries: Vec<TreeEntry> = blobs
            .iter()
            .enumerate()
            .map(|(i, (hash, _))| TreeEntry {
                path: format!("blob_{}", i),
                content_hash: hash.clone(),
                kind: TreeEntryKind::Blob,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    async fn diff(
        &self,
        _repo: &RepoId,
        _from: &str,
        _to: &str,
    ) -> Result<Vec<FileDiff>, GitCasError> {
        // Mock returns empty diff — no real git state to compare
        Ok(vec![])
    }

    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError> {
        let blobs = self.blobs.read().unwrap();
        let total = blobs.len();
        let mut corrupt = Vec::new();
        for (hash, content) in blobs.iter() {
            let actual = ContentHash::from_blake3(content);
            if &actual != hash {
                corrupt.push(hash.clone());
            }
        }
        Ok(VerificationReport {
            repo: repo.clone(),
            total_blobs: total,
            verified_blobs: total - corrupt.len(),
            corrupt_hashes: corrupt,
        })
    }

    async fn log(&self, repo: &RepoId, max_count: usize) -> Result<Vec<LogEntry>, GitCasError> {
        let snapshots = self.snapshots.read().unwrap();
        let entries: Vec<LogEntry> = snapshots
            .iter()
            .rev() // newest first
            .filter(|(r, _, _)| r == repo)
            .map(|(_, message, commit)| LogEntry {
                commit: commit.clone(),
                message: message.clone(),
                timestamp_secs: 0, // mock has no timestamp
            })
            .take(max_count)
            .collect();
        Ok(entries)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Tracer bullet 1: GitCASPort trait is object-safe and mock-constructible.
    #[test]
    fn git_cas_port_is_object_safe_and_mock_constructible() {
        // This test verifies that GitCASPort can be used as a trait object
        // (object safety) and that MockGitCas satisfies the trait.
        let mock: Box<dyn GitCASPort> = Box::new(MockGitCas::new());
        // If this compiles, the trait is object-safe and the mock implements it.
        drop(mock);
    }

    // Tracer bullet 2: ContentHash round-trips through serialization
    // and from_blake3 produces deterministic hashes.
    #[test]
    fn content_hash_round_trips_through_serialization() {
        let data = b"hello, hKask!";
        let hash = ContentHash::from_blake3(data);

        // Deterministic: same input → same hash
        let hash2 = ContentHash::from_blake3(data);
        assert_eq!(hash, hash2, "BLAKE3 must be deterministic");

        // Different input → different hash
        let hash3 = ContentHash::from_blake3(b"different data");
        assert_ne!(hash, hash3, "different data must produce different hashes");
    }

    #[test]
    fn content_hash_serializes_and_deserializes() {
        let hash = ContentHash::from_blake3(b"test content");

        // JSON round-trip
        let json = serde_json::to_string(&hash).expect("serialize ContentHash");
        let back: ContentHash = serde_json::from_str(&json).expect("deserialize ContentHash");
        assert_eq!(hash, back, "ContentHash must round-trip through JSON");
    }

    #[test]
    fn content_hash_display_and_from_str_round_trip() {
        let hash = ContentHash::from_blake3(b"display test");
        let displayed = hash.to_string();

        // Parse back
        let parsed: ContentHash = displayed.parse().expect("parse ContentHash from hex");
        assert_eq!(hash, parsed, "Display → FromStr must round-trip");
    }

    #[test]
    fn commit_hash_display_and_from_str_round_trip() {
        let commit = CommitHash::from_bytes([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
        ]);
        let displayed = commit.to_string();
        let parsed: CommitHash = displayed.parse().expect("parse CommitHash from hex");
        assert_eq!(
            commit, parsed,
            "Display → FromStr must round-trip for CommitHash"
        );
    }

    #[test]
    fn repo_id_dir_name_is_lowercase_with_hyphens() {
        assert_eq!(RepoId::Registry.dir_name(), "registry");
        assert_eq!(RepoId::CnsAudit.dir_name(), "cns-audit");
        assert_eq!(RepoId::GoalsSpecs.dir_name(), "goals-specs");
    }

    #[test]
    fn repo_id_all_returns_seven_variants() {
        assert_eq!(RepoId::all().len(), 7);
    }

    #[tokio::test]
    async fn mock_git_cas_put_and_get_round_trips() {
        let mock = MockGitCas::new();
        let repo = RepoId::Registry;

        // Put a blob
        let hash = mock
            .put_blob(&repo, b"test content")
            .await
            .expect("put_blob should succeed");

        // Get it back
        let content = mock
            .get_blob(&repo, &hash)
            .await
            .expect("get_blob should succeed");
        assert_eq!(content, b"test content");

        // Hash is deterministic
        let hash2 = mock
            .put_blob(&repo, b"test content")
            .await
            .expect("put_blob");
        assert_eq!(hash, hash2, "same content must produce same hash");
    }

    #[tokio::test]
    async fn mock_git_cas_get_blob_not_found() {
        let mock = MockGitCas::new();
        let hash = ContentHash::from_blake3(b"nonexistent");
        let result = mock.get_blob(&RepoId::Registry, &hash).await;
        assert!(result.is_err(), "get_blob should fail for nonexistent hash");
    }

    #[tokio::test]
    async fn mock_git_cas_snapshot_records_history() {
        let mock = MockGitCas::new();
        let repo = RepoId::Registry;

        let commit = mock
            .snapshot(&repo, "initial commit")
            .await
            .expect("snapshot");
        assert!(
            !commit.as_bytes().iter().all(|&b| b == 0u8),
            "commit hash should be non-null"
        );

        let history = mock.snapshot_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].1, "initial commit");
    }

    #[tokio::test]
    async fn mock_git_cas_verify_reports_integrity() {
        let mock = MockGitCas::new();
        let repo = RepoId::Memory;

        mock.put_blob(&repo, b"some data").await.expect("put_blob");
        let report = mock.verify(&repo).await.expect("verify");

        assert_eq!(report.total_blobs, 1);
        assert_eq!(report.verified_blobs, 1);
        assert!(report.corrupt_hashes.is_empty(), "no corruption expected");
    }

    #[test]
    fn git_cas_error_variants_have_distinct_recovery_paths() {
        // C5: every error variant = unique recovery path.
        let errors = vec![
            GitCasError::CrateNotFound("test".into()),
            GitCasError::Io("test".into()),
            GitCasError::Git("test".into()),
            GitCasError::PathValidation("test".into()),
            GitCasError::ContentHashMismatch {
                expected: "abc".into(),
                actual: "def".into(),
            },
            GitCasError::NotFound("test".into()),
        ];
        // All display differently
        let displays: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        // No two display strings start with the same prefix
        let mut prefixes: Vec<&str> = displays
            .iter()
            .map(|s| s.split(':').next().unwrap_or(s))
            .collect();
        prefixes.sort();
        prefixes.dedup();
        assert_eq!(
            prefixes.len(),
            6,
            "each error variant should have a unique prefix"
        );
    }

    // ── Retention Policy Tests ────────────────────────────────────────────────────

    #[test]
    fn retention_policy_default_has_four_tiers() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.tiers.len(), 4, "default cascade should have 4 tiers");
    }

    #[test]
    fn retention_policy_first_tier_is_30min_interval() {
        let policy = RetentionPolicy::default();
        let first = &policy.tiers[0];
        assert_eq!(
            first.interval_secs,
            30 * 60,
            "first tier: 30-minute interval"
        );
        assert_eq!(first.max_age_secs, 3 * 3600, "first tier: 3-hour max age");
    }

    #[test]
    fn retention_policy_last_tier_is_monthly_forever() {
        let policy = RetentionPolicy::default();
        let last = policy.tiers.last().expect("at least one tier");
        assert_eq!(
            last.interval_secs,
            30 * 86400,
            "last tier: monthly interval"
        );
        assert_eq!(last.max_age_secs, u64::MAX, "last tier: infinite max age");
    }

    #[test]
    fn retention_policy_tiers_are_ordered_by_age() {
        // Each tier's max_age should be strictly greater than the previous
        let policy = RetentionPolicy::default();
        for window in policy.tiers.windows(2) {
            assert!(
                window[0].max_age_secs < window[1].max_age_secs,
                "tiers must be ordered by ascending max_age"
            );
        }
    }

    #[test]
    fn retention_policy_serializes_and_deserializes() {
        let policy = RetentionPolicy::default();
        let json = serde_json::to_string(&policy).expect("serialize");
        let back: RetentionPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(policy.tiers.len(), back.tiers.len());
        for (orig, deser) in policy.tiers.iter().zip(back.tiers.iter()) {
            assert_eq!(orig.max_age_secs, deser.max_age_secs);
            assert_eq!(orig.interval_secs, deser.interval_secs);
        }
    }

    #[test]
    fn repo_snapshot_policy_default_for_uses_global_default() {
        let policy = RepoSnapshotPolicy::default_for(RepoId::Registry);
        assert!(policy.enabled);
        assert!(policy.policy.is_none());
        // effective_policy falls back to global default
        let effective = policy.effective_policy();
        assert_eq!(effective.tiers.len(), 4);
    }

    #[test]
    fn repo_snapshot_policy_disabled_disables_snapshots() {
        let policy = RepoSnapshotPolicy::disabled(RepoId::Vault);
        assert!(!policy.enabled);
    }

    #[test]
    fn snapshot_trigger_serializes() {
        let triggers = vec![
            SnapshotTrigger::Manual,
            SnapshotTrigger::Scheduled,
            SnapshotTrigger::CnsTriggered,
        ];
        let json = serde_json::to_string(&triggers).expect("serialize");
        let back: Vec<SnapshotTrigger> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(triggers.len(), back.len());
    }
}
