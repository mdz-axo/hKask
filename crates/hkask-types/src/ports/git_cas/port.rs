//! Git CAS hexagonal port — trait, mock implementation, and verification/report types.
use async_trait::async_trait;

use super::error::GitCasError;
use super::types::{CommitHash, ContentHash, FileDiff, RepoId, TreeEntry, TreeEntryKind};
use crate::text::blake3_hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

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

    /// Create a snapshot commit of all staged changes.
    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError>;

    /// Create an orphan snapshot commit (no parent) for history rewriting.
    async fn snapshot_orphan(
        &self,
        repo: &RepoId,
        message: &str,
    ) -> Result<CommitHash, GitCasError>;

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
    ///
    /// REQ: TYP-262
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
    /// REQ: TYP-263
    /// pre:  self is any [`MockGitCas`]
    /// post: returns a [`Vec`] of all snapshots recorded via [`GitCASPort::snapshot`] calls,
    ///       in insertion order (oldest first); never panics
    pub fn snapshot_history(&self) -> Vec<(RepoId, String, CommitHash)> {
        self.snapshots
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Return the number of blobs stored via `put_blob`.
    ///
    /// REQ: TYP-264
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

    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError> {
        // Generate a deterministic commit hash from the message
        let hash_bytes = blake3_hash(message.as_bytes());
        let mut commit_bytes = [0u8; 20];
        commit_bytes.copy_from_slice(&hash_bytes[..20]);
        let commit = CommitHash::from_bytes(commit_bytes);

        self.snapshots
            .write()
            .expect("MockGitCas RwLock write")
            .push((repo.clone(), message.to_string(), commit.clone()));
        Ok(commit)
    }

    async fn resolve_ref(
        &self,
        _repo: &RepoId,
        _reference: &str,
    ) -> Result<CommitHash, GitCasError> {
        let snapshots = self.snapshots.read().unwrap_or_else(|e| e.into_inner());
        snapshots
            .last()
            .map(|(_, _, hash)| hash.clone())
            .ok_or_else(|| GitCasError::NotFound("no snapshots".to_string()))
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
        _reference: &str,
        _prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError> {
        let blobs = self.blobs.read().unwrap_or_else(|e| e.into_inner());
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
