//! RepoManager — Multi-repo management over [`GitCASPort`]
//!
//! Owns an `Arc<dyn GitCASPort>` and delegates operations to the correct
//! repository via [`RepoId`]. Also provides batch operations (`snapshot_all`,
//! `verify_all`) and environment-based construction.

use hkask_types::ports::git_cas::{
    CommitHash, ContentHash, FileDiff, GitCASPort, GitCasError, LogEntry, RepoId, TreeEntry,
    VerificationReport,
};
use std::sync::Arc;

/// Multi-repo manager delegating to a [`GitCASPort`] implementation.
///
/// Does not inherit from the adapter — it *delegates* (P2). The adapter
/// is injected via `Arc<dyn GitCASPort>`, so the RepoManager is testable
/// with [`MockGitCas`].
#[allow(dead_code)]
pub(crate) struct RepoManager {
    adapter: Arc<dyn GitCASPort>,
}

#[allow(dead_code)]
impl RepoManager {
    /// Create a new RepoManager wrapping the given adapter.
    pub(crate) fn new(adapter: Arc<dyn GitCASPort>) -> Self {
        Self { adapter }
    }

    /// Store a blob in the specified repo, returning its content hash.
    pub(crate) async fn put_blob(
        &self,
        repo: &RepoId,
        content: &[u8],
    ) -> Result<ContentHash, GitCasError> {
        self.adapter.put_blob(repo, content).await
    }

    /// Retrieve a blob by its content hash from the specified repo.
    pub(crate) async fn get_blob(
        &self,
        repo: &RepoId,
        hash: &ContentHash,
    ) -> Result<Vec<u8>, GitCasError> {
        self.adapter.get_blob(repo, hash).await
    }

    /// Create a snapshot commit of all staged changes in the specified repo.
    pub(crate) async fn snapshot(
        &self,
        repo: &RepoId,
        message: &str,
    ) -> Result<CommitHash, GitCasError> {
        self.adapter.snapshot(repo, message).await
    }

    /// Resolve a symbolic ref to a commit SHA in the specified repo.
    pub(crate) async fn resolve_ref(
        &self,
        repo: &RepoId,
        reference: &str,
    ) -> Result<CommitHash, GitCasError> {
        self.adapter.resolve_ref(repo, reference).await
    }

    /// List tree entries at a given ref in the specified repo.
    pub(crate) async fn list_tree(
        &self,
        repo: &RepoId,
        reference: &str,
        prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError> {
        self.adapter.list_tree(repo, reference, prefix).await
    }

    /// Diff two commits in the specified repo.
    pub(crate) async fn diff(
        &self,
        repo: &RepoId,
        from: &str,
        to: &str,
    ) -> Result<Vec<FileDiff>, GitCasError> {
        self.adapter.diff(repo, from, to).await
    }

    /// Verify content integrity in the specified repo.
    pub(crate) async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError> {
        self.adapter.verify(repo).await
    }

    /// List snapshot history for a repo, up to `max_count` entries.
    pub(crate) async fn log(
        &self,
        repo: &RepoId,
        max_count: usize,
    ) -> Result<Vec<LogEntry>, GitCasError> {
        self.adapter.log(repo, max_count).await
    }

    /// Snapshot all 7 repos, returning a vec of (repo, result) pairs.
    ///
    /// Each snapshot is taken sequentially. A failure in one repo does not
    /// prevent others from being snapshotted.
    pub(crate) async fn snapshot_all(
        &self,
        message: &str,
    ) -> Vec<(RepoId, Result<CommitHash, GitCasError>)> {
        let mut results = Vec::with_capacity(RepoId::all().len());
        for repo in RepoId::all() {
            let result = self.adapter.snapshot(repo, message).await;
            results.push((repo.clone(), result));
        }
        results
    }

    /// Verify all 7 repos, returning a vec of (repo, result) pairs.
    ///
    /// Each verification is independent. A failure in one repo does not
    /// prevent others from being verified.
    pub(crate) async fn verify_all(
        &self,
    ) -> Vec<(RepoId, Result<VerificationReport, GitCasError>)> {
        let mut results = Vec::with_capacity(RepoId::all().len());
        for repo in RepoId::all() {
            let result = self.adapter.verify(repo).await;
            results.push((repo.clone(), result));
        }
        results
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::ports::git_cas::MockGitCas;

    /// Tracer bullet 12: RepoManager delegates put_blob/get_blob to adapter.
    #[tokio::test]
    async fn repo_manager_delegates_put_and_get() {
        let mock = Arc::new(MockGitCas::new());
        let manager = RepoManager::new(mock);

        let content = b"hello repo manager";
        let hash = manager
            .put_blob(&RepoId::Registry, content)
            .await
            .expect("put_blob should succeed");

        let retrieved = manager
            .get_blob(&RepoId::Registry, &hash)
            .await
            .expect("get_blob should succeed");
        assert_eq!(retrieved, content);
    }

    /// Tracer bullet 13: RepoManager.snapshot_all snapshots all 7 repos.
    #[tokio::test]
    async fn repo_manager_snapshot_all_snapshots_every_repo() {
        let mock = Arc::new(MockGitCas::new());
        let manager = RepoManager::new(mock);

        let results = manager.snapshot_all("test snapshot").await;
        assert_eq!(results.len(), 7, "snapshot_all should cover all 7 repos");

        for (repo, result) in &results {
            assert!(
                result.is_ok(),
                "snapshot of {:?} should succeed, got: {:?}",
                repo,
                result
            );
        }
    }

    /// Tracer bullet 14: RepoManager.verify_all verifies all 7 repos.
    #[tokio::test]
    async fn repo_manager_verify_all_checks_every_repo() {
        let mock = Arc::new(MockGitCas::new());
        let manager = RepoManager::new(mock);

        // Put some content so verify has something to check
        manager
            .put_blob(&RepoId::Memory, b"verify me")
            .await
            .expect("put_blob should succeed");

        let results = manager.verify_all().await;
        assert_eq!(results.len(), 7, "verify_all should cover all 7 repos");

        for (repo, result) in &results {
            let report = result.as_ref().expect("verify should succeed");
            assert_eq!(
                report.repo, *repo,
                "report repo should match requested repo"
            );
        }
    }
}
