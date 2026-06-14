//! BackupService — policy layer on top of GitCASPort.
//! # REQ: P1 (User Sovereignty) — user controls what is tracked.
//! # REQ: P4 (Clear Boundaries) — delegates to hexagonal GitCASPort, never raw git.
//!
//! The backup service adds backup-specific semantics (scoped snapshot/restore,
//! retention pruning, CNS alerting) on top of the content-addressed git storage
//! provided by [`hkask_types::ports::git_cas::GitCASPort`].
//!
//! Public API (7 operations, per essentialist G2):
//! 1. `snapshot` — capture artifact state to git
//! 2. `restore` — restore artifacts from a prior snapshot
//! 3. `list` — list snapshot history
//! 4. `prune` — remove expired snapshots per retention policy
//! 5. `verify` — integrity check with CNS alerting
//! 6. `config` — get current backup configuration
//! 7. `update_config` — update backup configuration

pub mod config;
pub mod metadata;
pub mod scope;
pub mod serialization;

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use hkask_types::ports::git_cas::{CommitHash, GitCASPort, GitCasError, LogEntry, RepoId};

use config::BackupConfig;
use metadata::{PruneReport, SnapshotMetadata, SnapshotTrigger};
use scope::{ArtifactType, BackupScope, ListFilter, RestoreScope};

/// Errors specific to backup operations.
///
/// Composes with [`GitCasError`] for CAS-level failures and adds
/// backup-specific error states (config, serialization, CNS).
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    /// Underlying CAS operation failed.
    #[error("CAS error: {0}")]
    Cas(#[from] GitCasError),

    /// Artifact serialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Configuration is invalid or missing.
    #[error("Configuration error: {0}")]
    Config(String),

    /// CNS alerting failed (non-fatal — backup succeeded but alert didn't fire).
    #[error("CNS alert error: {0}")]
    Cns(String),

    /// Requested artifact type is not tracked in current config.
    #[error("Artifact type '{0}' is not tracked — add it to backup config first")]
    NotTracked(ArtifactType),

    /// No snapshots found matching the filter.
    #[error("No snapshots found")]
    NoSnapshots,
}

/// Policy layer for git-based artifact backup.
///
/// Wraps a [`GitCASPort`] implementation and adds:
/// - Scoped snapshot/restore (by artifact type or specific IDs)
/// - Retention policy enforcement (pruning)
/// - Integrity verification with CNS alerting
/// - Configuration management
///
/// The service does NOT own repository lifecycle — that belongs to the
/// CAS port implementation. It does NOT reimplement CAS primitives —
/// it delegates every git operation to the port.
pub struct BackupService {
    /// Hexagonal CAS port — all git operations delegate here.
    cas: Arc<dyn GitCASPort>,

    /// Current backup configuration.
    config: BackupConfig,
}

impl BackupService {
    /// Create a new backup service wrapping a CAS port.
    ///
    /// The config is loaded from disk via [`super::config::load_backup_config`]
    /// at construction time. Use [`Self::update_config`] to change it.
    pub fn new(cas: Arc<dyn GitCASPort>) -> Self {
        let config = config::load_backup_config();
        Self { cas, config }
    }

    /// Create a new backup service with an explicit config (for testing).
    pub fn with_config(cas: Arc<dyn GitCASPort>, config: BackupConfig) -> Self {
        Self { cas, config }
    }

    // ── Public API (7 operations) ────────────────────────────────────────

    /// 1. Snapshot artifacts to git.
    ///
    /// Resolves the scope to concrete artifacts, serializes each to a
    /// deterministic blob, stores via `put_blob`, then commits via `snapshot`.
    /// The commit DAG IS the changelog — each commit records what changed.
    ///
    /// Currently snapshots are **manual only** — the caller provides artifact
    /// data directly. Auto-snapshot on mutation (F4) will be wired when the
    /// daemon's MutationEvent emission is implemented.
    pub async fn snapshot(
        &self,
        scope: BackupScope,
        artifacts: &[(ArtifactType, String, Vec<u8>)],
    ) -> Result<SnapshotMetadata, BackupError> {
        // Validate: all artifact types in scope must be tracked
        self.validate_scope(&scope)?;

        // Group artifacts by repo
        let mut by_repo: std::collections::HashMap<RepoId, Vec<(&str, &[u8])>> =
            std::collections::HashMap::new();
        let mut artifact_count = 0usize;

        for (artifact_type, artifact_id, bytes) in artifacts {
            if !self.is_tracked(artifact_type) {
                continue;
            }
            let repo_id = artifact_type.repo_id();
            let _path = serialization::artifact_git_path(artifact_type, artifact_id);
            by_repo
                .entry(repo_id)
                .or_default()
                .push((artifact_id.as_str(), bytes.as_slice()));
            artifact_count += 1;
        }

        if artifact_count == 0 {
            return Err(BackupError::NoSnapshots);
        }

        // For each repo: put blobs, then snapshot
        let mut commits = Vec::new();
        for (repo_id, blobs) in &by_repo {
            for (_id, bytes) in blobs {
                self.cas.put_blob(repo_id, bytes).await?;
            }
            let message = format!(
                "backup: {} — {}",
                scope.description(),
                Utc::now().format("%Y-%m-%d %H:%M:%S")
            );
            let commit_hash = self.cas.snapshot(repo_id, &message).await?;
            commits.push((repo_id.clone(), commit_hash));
        }

        Ok(SnapshotMetadata {
            commits,
            artifact_count,
            trigger: SnapshotTrigger::Manual,
            timestamp: Utc::now(),
        })
    }

    /// 2. Restore artifacts from a prior snapshot.
    ///
    /// Resolves the target commit, lists the tree at that commit filtered
    /// by scope, reads each blob, and returns the deserialized artifact data.
    /// Callers are responsible for writing restored data back to the
    /// appropriate store (registry, memory, etc.).
    pub async fn restore(
        &self,
        target: &CommitHash,
        scope: RestoreScope,
    ) -> Result<Vec<(ArtifactType, String, Vec<u8>)>, BackupError> {
        // Determine which repos and prefixes to query
        let queries = self.resolve_restore_queries(&scope);

        let mut restored = Vec::new();
        for (repo_id, prefix) in &queries {
            let target_str = target.to_string();
            let entries = self.cas.list_tree(repo_id, &target_str, prefix).await?;

            for entry in entries {
                let blob = self.cas.get_blob(repo_id, &entry.content_hash).await?;

                // Parse the envelope to extract artifact type and ID
                let envelope: serialization::ArtifactEnvelopeValue = serde_json::from_slice(&blob)
                    .map_err(|e| {
                        BackupError::Serialization(format!(
                            "Failed to deserialize artifact at {}: {e}",
                            entry.path
                        ))
                    })?;

                // Resolve artifact type from the envelope label
                let artifact_type =
                    artifact_type_from_label(&envelope.artifact_type).ok_or_else(|| {
                        BackupError::Serialization(format!(
                            "Unknown artifact type in blob: {}",
                            envelope.artifact_type
                        ))
                    })?;

                // If scoped to specific IDs, filter
                if let RestoreScope::ByIds { ref ids, .. } = scope {
                    if !ids.contains(&envelope.artifact_id) {
                        continue;
                    }
                }

                restored.push((artifact_type, envelope.artifact_id, blob));
            }
        }

        Ok(restored)
    }

    /// 3. List snapshot history.
    ///
    /// Returns snapshots across all tracked repos, filtered by artifact type
    /// and limited by count. Newest first.
    pub async fn list(&self, filter: ListFilter) -> Result<Vec<SnapshotMetadata>, BackupError> {
        let repos: Vec<RepoId> = if let Some(ref at) = filter.artifact_type {
            vec![at.repo_id()]
        } else {
            self.tracked_repos()
        };

        let limit = filter.limit.unwrap_or(20);
        let mut snapshots = Vec::new();

        for repo_id in &repos {
            let entries: Vec<LogEntry> = self.cas.log(repo_id, limit).await?;
            for entry in entries {
                snapshots.push(SnapshotMetadata {
                    commits: vec![(repo_id.clone(), entry.commit)],
                    artifact_count: 0, // log doesn't give us artifact count
                    trigger: SnapshotTrigger::Manual, // log doesn't give us trigger
                    timestamp: chrono::DateTime::from_timestamp(entry.timestamp_secs as i64, 0)
                        .unwrap_or_default(),
                });
            }
        }

        if snapshots.is_empty() {
            return Err(BackupError::NoSnapshots);
        }

        // Sort by timestamp descending, then truncate to limit
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        snapshots.truncate(limit);

        Ok(snapshots)
    }

    /// 4. Prune expired snapshots.
    ///
    /// Evaluates each snapshot against the retention policy. In dry-run mode,
    /// reports what WOULD be removed without actually removing anything.
    /// Actual pruning (git history rewriting) is deferred to F5 — currently
    /// this is always a dry-run that reports eligibility.
    pub async fn prune(&self, dry_run: bool) -> Result<PruneReport, BackupError> {
        let policy = match &self.config.retention {
            Some(p) => p.clone(),
            None => {
                return Ok(PruneReport {
                    dry_run,
                    evaluated: 0,
                    removed: Vec::new(),
                    retained: 0,
                });
            }
        };

        let repos = self.tracked_repos();
        let mut evaluated = 0usize;
        let mut removed = Vec::new();
        let mut retained = 0usize;

        let now_secs = Utc::now().timestamp() as u64;
        let cutoff = now_secs.saturating_sub(policy.max_age_secs);

        for repo_id in &repos {
            // Get all log entries (up to a reasonable limit)
            let entries: Vec<LogEntry> = self.cas.log(repo_id, 1000).await?;
            evaluated += entries.len();

            // Sort by timestamp ascending for age evaluation
            let mut sorted = entries;
            sorted.sort_by_key(|e| e.timestamp_secs);

            // Keep the most recent `min_keep` regardless of age
            let min_keep = policy.min_keep;
            let keep_count = sorted.len().min(min_keep);

            for (i, entry) in sorted.iter().enumerate() {
                if i < keep_count {
                    retained += 1;
                } else if entry.timestamp_secs < cutoff {
                    removed.push((repo_id.clone(), entry.commit.clone()));
                } else {
                    retained += 1;
                }
            }
        }

        Ok(PruneReport {
            dry_run,
            evaluated,
            removed,
            retained,
        })
    }

    /// 5. Verify integrity of all tracked repositories.
    ///
    /// Delegates to [`GitCASPort::verify`] for each tracked repo.
    /// Returns the combined verification report. CNS alerting for
    /// integrity failures is handled by the caller (daemon).
    pub async fn verify(
        &self,
    ) -> Result<Vec<hkask_types::ports::git_cas::VerificationReport>, BackupError> {
        let repos = self.tracked_repos();
        let mut reports = Vec::new();

        for repo_id in &repos {
            let report = self.cas.verify(repo_id).await?;
            reports.push(report);
        }

        Ok(reports)
    }

    /// 6. Get current backup configuration.
    pub fn config(&self) -> &BackupConfig {
        &self.config
    }

    /// 7. Update backup configuration and persist to disk.
    pub fn update_config(&mut self, config: BackupConfig) -> Result<(), BackupError> {
        config::save_backup_config(&config)
            .map_err(|e| BackupError::Config(format!("Failed to save config: {e}")))?;
        self.config = config;
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    /// Check whether an artifact type is tracked in the current config.
    fn is_tracked(&self, artifact_type: &ArtifactType) -> bool {
        self.config.tracked_types.contains(artifact_type)
    }

    /// Validate that the scope's artifact types are all tracked.
    fn validate_scope(&self, scope: &BackupScope) -> Result<(), BackupError> {
        match scope {
            BackupScope::Full => {
                if self.config.tracked_types.is_empty() {
                    return Err(BackupError::Config(
                        "No artifact types are tracked. Configure backup first.".into(),
                    ));
                }
            }
            BackupScope::ByType(at) => {
                if !self.is_tracked(at) {
                    return Err(BackupError::NotTracked(at.clone()));
                }
            }
            BackupScope::ByIds { artifact_type, .. } => {
                if !self.is_tracked(artifact_type) {
                    return Err(BackupError::NotTracked(artifact_type.clone()));
                }
            }
        }
        Ok(())
    }

    /// Get the set of repos for all tracked artifact types.
    fn tracked_repos(&self) -> Vec<RepoId> {
        let mut seen = HashSet::new();
        let mut repos = Vec::new();
        for at in &self.config.tracked_types {
            let repo_id = at.repo_id();
            if seen.insert(repo_id.clone()) {
                repos.push(repo_id);
            }
        }
        repos
    }

    /// Resolve a restore scope into (repo, tree_prefix) queries.
    fn resolve_restore_queries(&self, scope: &RestoreScope) -> Vec<(RepoId, String)> {
        match scope {
            RestoreScope::Full => self
                .tracked_repos()
                .into_iter()
                .map(|r| (r, String::new()))
                .collect(),
            RestoreScope::ByType(at) => {
                vec![(at.repo_id(), format!("{}/", at.label()))]
            }
            RestoreScope::ByIds { artifact_type, .. } => {
                vec![(
                    artifact_type.repo_id(),
                    format!("{}/", artifact_type.label()),
                )]
            }
        }
    }
}

/// Resolve an artifact type from its label string.
fn artifact_type_from_label(label: &str) -> Option<ArtifactType> {
    match label {
        "template" => Some(ArtifactType::Template),
        "style" => Some(ArtifactType::Style),
        "goal" => Some(ArtifactType::Goal),
        "spec" => Some(ArtifactType::Spec),
        "memory_triple" => Some(ArtifactType::MemoryTriple),
        "embedding" => Some(ArtifactType::Embedding),
        "registry_entry" => Some(ArtifactType::RegistryEntry),
        "cns_audit" => Some(ArtifactType::CnsAudit),
        "sovereignty_manifest" => Some(ArtifactType::SovereigntyManifest),
        "session" => Some(ArtifactType::Session),
        "wallet_state" => Some(ArtifactType::WalletState),
        "settings" => Some(ArtifactType::Settings),
        _ => None,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::ports::git_cas::MockGitCas;

    fn test_config() -> BackupConfig {
        BackupConfig {
            tracked_types: vec![ArtifactType::Template, ArtifactType::Goal],
            retention: None,
            auto_snapshot: true,
            verify_after_snapshot: false,
        }
    }

    fn test_service() -> BackupService {
        let mock = Arc::new(MockGitCas::new());
        BackupService::with_config(mock, test_config())
    }

    // REQ: BACKUP-SNAPSHOT-001 — Snapshot of tracked type produces commits
    #[tokio::test]
    async fn snapshot_tracked_type_produces_commits() {
        let svc = test_service();
        let artifacts = vec![(
            ArtifactType::Template,
            "tpl-1".to_string(),
            b"template data".to_vec(),
        )];
        let result = svc
            .snapshot(BackupScope::ByType(ArtifactType::Template), &artifacts)
            .await
            .unwrap();
        assert_eq!(result.artifact_count, 1);
        assert!(!result.commits.is_empty());
    }

    // REQ: BACKUP-SNAPSHOT-002 — Snapshot of untracked type is rejected
    #[tokio::test]
    async fn snapshot_untracked_type_rejected() {
        let svc = test_service();
        let artifacts = vec![(
            ArtifactType::MemoryTriple,
            "mem-1".to_string(),
            b"memory data".to_vec(),
        )];
        let result = svc
            .snapshot(BackupScope::ByType(ArtifactType::MemoryTriple), &artifacts)
            .await;
        assert!(matches!(result, Err(BackupError::NotTracked(_))));
    }

    // REQ: BACKUP-SNAPSHOT-003 — Full snapshot with no tracked types errors
    #[tokio::test]
    async fn full_snapshot_no_tracked_types_errors() {
        let mock = Arc::new(MockGitCas::new());
        let svc = BackupService::with_config(mock, BackupConfig::default());
        let result = svc.snapshot(BackupScope::Full, &[]).await;
        assert!(matches!(result, Err(BackupError::Config(_))));
    }

    // REQ: BACKUP-RESTORE-001 — Restore reproduces artifact state
    #[tokio::test]
    async fn restore_reproduces_state() {
        let mock = Arc::new(MockGitCas::new());
        let svc = BackupService::with_config(mock.clone(), test_config());

        // First, snapshot some data
        let data = serde_json::to_vec(&serde_json::json!({"name": "test-tpl"})).unwrap();
        let artifacts = vec![(ArtifactType::Template, "tpl-1".to_string(), data)];
        let snap = svc
            .snapshot(BackupScope::ByType(ArtifactType::Template), &artifacts)
            .await
            .unwrap();

        // Restore from that snapshot
        let commit = &snap.commits[0].1;
        let restored = svc
            .restore(commit, RestoreScope::ByType(ArtifactType::Template))
            .await
            .unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].0, ArtifactType::Template);
        assert_eq!(restored[0].1, "tpl-1");
    }

    // REQ: BACKUP-LIST-001 — List returns snapshots for tracked repos
    #[tokio::test]
    async fn list_returns_snapshots() {
        let svc = test_service();
        let artifacts = vec![(
            ArtifactType::Template,
            "tpl-1".to_string(),
            b"data".to_vec(),
        )];
        svc.snapshot(BackupScope::ByType(ArtifactType::Template), &artifacts)
            .await
            .unwrap();

        let snapshots = svc.list(ListFilter::default()).await.unwrap();
        assert!(!snapshots.is_empty());
    }

    // REQ: BACKUP-PRUNE-001 — Prune with retention removes old snapshots
    #[tokio::test]
    async fn prune_with_retention_identifies_expired() {
        let mock = Arc::new(MockGitCas::new());
        let config = BackupConfig {
            tracked_types: vec![ArtifactType::Template],
            retention: Some(RetentionPolicy {
                max_age_secs: 0, // everything is expired
                min_keep: 1,     // but keep at least 1
            }),
            auto_snapshot: true,
            verify_after_snapshot: false,
        };
        let svc = BackupService::with_config(mock.clone(), config);

        // Create a snapshot
        let artifacts = vec![(
            ArtifactType::Template,
            "tpl-1".to_string(),
            b"data".to_vec(),
        )];
        svc.snapshot(BackupScope::ByType(ArtifactType::Template), &artifacts)
            .await
            .unwrap();

        let report = svc.prune(true).await.unwrap();
        assert!(report.dry_run);
        assert!(report.evaluated > 0);
        // With min_keep=1, the most recent snapshot is retained
        assert_eq!(report.retained, 1);
    }

    // REQ: BACKUP-VERIFY-001 — Verify returns reports for tracked repos
    #[tokio::test]
    async fn verify_returns_reports() {
        let svc = test_service();
        let reports = svc.verify().await.unwrap();
        // MockGitCas returns empty reports for repos with no blobs
        assert!(!reports.is_empty());
    }

    // REQ: BACKUP-CONFIG-004 — Update config persists and reflects changes
    #[tokio::test]
    async fn update_config_persists_and_reflects() {
        let mut svc = test_service();
        let new_config = BackupConfig {
            tracked_types: vec![ArtifactType::MemoryTriple],
            ..test_config()
        };
        svc.update_config(new_config.clone()).unwrap();
        assert_eq!(svc.config().tracked_types, new_config.tracked_types);
    }
}
