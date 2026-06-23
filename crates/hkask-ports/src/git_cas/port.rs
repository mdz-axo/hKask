//! Git CAS hexagonal port — trait, mock implementation, and verification/report types.
use async_trait::async_trait;

use super::error::GitCasError;
use super::types::{CommitHash, ContentHash, RepoId, TreeEntry, TreeEntryKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// A stored snapshot entry: (repo, message, commit_hash, timestamp_secs, content_hashes).
type SnapshotEntry = (RepoId, String, CommitHash, u64, Vec<ContentHash>);

// ── Verification / Log Types ─────────────────────────────────────────────────

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

    /// Delete a blob by its BLAKE3 hash.
    async fn delete_blob(&self, repo: &RepoId, hash: &ContentHash) -> Result<(), GitCasError>;

    /// Create a snapshot commit of all staged changes.
    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError>;

    /// Create an orphan snapshot commit (no parent) for history rewriting.
    async fn snapshot_orphan(
        &self,
        repo: &RepoId,
        message: &str,
    ) -> Result<CommitHash, GitCasError>;

    /// List file paths at a given ref with their content hashes.
    async fn list_tree(
        &self,
        repo: &RepoId,
        reference: &str,
        prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError>;

    /// Verify content integrity: re-hash all blobs, compare to stored hashes.
    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError>;

    /// List snapshot history for a repo.
    ///
    /// Returns commit entries from newest to oldest, up to `max_count`.
    async fn log(&self, repo: &RepoId, max_count: usize) -> Result<Vec<LogEntry>, GitCasError>;
}

// ── MockGitCas (test helper) ─────────────────────────────────────────────────

/// Return the current Unix timestamp in seconds.
#[cfg(not(target_arch = "wasm32"))]
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(target_arch = "wasm32")]
fn now_secs() -> u64 {
    0
}

/// In-memory mock implementation of [`GitCASPort`] for testing.
///
/// Stores blobs in a `HashMap` and snapshots in a `Vec`. Does not
/// perform any real git operations. Useful for unit tests where a
/// real git repository is unnecessary.
pub struct MockGitCas {
    blobs: RwLock<HashMap<ContentHash, Vec<u8>>>,
    snapshots: RwLock<Vec<SnapshotEntry>>,
}

impl MockGitCas {
    /// Create a new empty mock.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  (no inputs)
    /// post: returns a [`MockGitCas`] with empty blob storage and empty snapshot history
    pub fn new() -> Self {
        Self {
            blobs: RwLock::new(HashMap::new()),
            snapshots: RwLock::new(Vec::new()),
        }
    }

    /// Return the history of snapshot calls as `(repo, message, commit_hash)`.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`MockGitCas`]
    /// post: returns a [`Vec`] of all snapshots recorded via [`GitCASPort::snapshot`] calls,
    ///       in insertion order (oldest first); never panics
    pub fn snapshot_history(&self) -> Vec<(RepoId, String, CommitHash)> {
        self.snapshots
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(r, m, c, _, _)| (r.clone(), m.clone(), c.clone()))
            .collect()
    }

    /// Return the number of blobs stored via `put_blob`.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is any [`MockGitCas`]
    /// post: returns the count of unique blobs currently stored; never panics
    pub fn blob_count(&self) -> usize {
        self.blobs.read().unwrap_or_else(|e| e.into_inner()).len()
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
            .expect("MockGitCas RwLock write")
            .insert(hash.clone(), content.to_vec());
        Ok(hash)
    }

    async fn get_blob(&self, _repo: &RepoId, hash: &ContentHash) -> Result<Vec<u8>, GitCasError> {
        self.blobs
            .read()
            .expect("MockGitCas RwLock read")
            .get(hash)
            .cloned()
            .ok_or_else(|| GitCasError::NotFound(hash.to_string()))
    }

    async fn delete_blob(&self, _repo: &RepoId, hash: &ContentHash) -> Result<(), GitCasError> {
        self.blobs
            .write()
            .expect("MockGitCas RwLock write")
            .remove(hash);
        Ok(())
    }

    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError> {
        // Generate a deterministic commit hash from the message
        let hash_bytes = *blake3::hash(message.as_bytes()).as_bytes();
        let mut commit_bytes = [0u8; 20];
        commit_bytes.copy_from_slice(&hash_bytes[..20]);
        let commit = CommitHash::from_bytes(commit_bytes);

        // Capture all current blob hashes for this commit's tree
        let blob_hashes: Vec<ContentHash> = {
            let blobs = self.blobs.read().unwrap_or_else(|e| e.into_inner());
            blobs.keys().cloned().collect()
        };

        self.snapshots
            .write()
            .expect("MockGitCas RwLock write")
            .push((
                repo.clone(),
                message.to_string(),
                commit.clone(),
                now_secs(),
                blob_hashes,
            ));
        Ok(commit)
    }

    async fn snapshot_orphan(
        &self,
        repo: &RepoId,
        message: &str,
    ) -> Result<CommitHash, GitCasError> {
        // Mock: same as snapshot but with no parent tracking.
        self.snapshot(repo, message).await
    }

    async fn list_tree(
        &self,
        _repo: &RepoId,
        reference: &str,
        _prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError> {
        // Look up the commit and return only its blobs.
        let snapshots = self.snapshots.read().unwrap_or_else(|e| e.into_inner());
        let commit_blobs = snapshots
            .iter()
            .find(|(_, _, c, _, _)| c.to_string() == reference)
            .map(|(_, _, _, _, blobs)| blobs.clone());

        let blob_hashes = match commit_blobs {
            Some(hashes) => hashes,
            // Commit not found — fall back to all blobs for backward compat
            None => {
                let blobs = self.blobs.read().unwrap_or_else(|e| e.into_inner());
                blobs.keys().cloned().collect()
            }
        };

        let mut entries: Vec<TreeEntry> = blob_hashes
            .iter()
            .enumerate()
            .map(|(i, hash)| TreeEntry {
                path: format!("blob_{}", i),
                content_hash: hash.clone(),
                kind: TreeEntryKind::Blob,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError> {
        let blobs = self.blobs.read().unwrap_or_else(|e| e.into_inner());
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
        let snapshots = self.snapshots.read().unwrap_or_else(|e| e.into_inner());
        let entries: Vec<LogEntry> = snapshots
            .iter()
            .rev() // newest first
            .filter(|(r, _, _, _, _)| r == repo)
            .map(|(_, message, commit, ts, _)| LogEntry {
                commit: commit.clone(),
                message: message.clone(),
                timestamp_secs: *ts,
            })
            .take(max_count)
            .collect();
        Ok(entries)
    }
}
