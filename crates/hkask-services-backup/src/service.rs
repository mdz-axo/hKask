//! BackupService — policy layer on top of GitCASPort.
//! # REQ: P1 (User Sovereignty) — user controls what is tracked.
//! expect: "My backup data is tracked under my sovereignty control"
//! # REQ: P4 (Clear Boundaries) — delegates to hexagonal GitCASPort, never raw git.
//! expect: "Backup operations delegate through OCAP boundaries"
//!
//! The backup service adds backup-specific semantics (scoped snapshot/restore,
//! retention pruning, CNS alerting) on top of the content-addressed git storage
//! provided by [`hkask_ports::git_cas::GitCASPort`].
//!
//! Public API (7 operations, per essentialist G2):
//! 1. `snapshot` — capture artifact state to git
//! 2. `restore` — restore artifacts from a prior snapshot
//! 3. `list` — list snapshot history
//! 4. `prune` — remove expired snapshots per retention policy
//! 5. `verify` — integrity check with CNS alerting
//! 6. `config` — get current backup configuration
//! 7. `update_config` — update backup configuration
//!
//! Pod-level operations (`revert`, `spawn_agent`) live in [`super::pod_ops::PodBackupOps`];
//! construct via [`BackupService::pod_ops`].

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::config::{BackupConfig, EncryptionConfig, RetentionPolicy};
use crate::metadata::{PruneReport, SnapshotMetadata, SnapshotTrigger};
use crate::scope::{ArtifactType, BackupScope, ListFilter, RestoreScope};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use argon2::Argon2;
use chrono::Utc;
use hkask_ports::git_cas::{CommitHash, GitCASPort, GitCasError, LogEntry, RepoId};
use rand::RngCore;
use rand::rng;
use tracing::{info, instrument, warn};

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

    /// No pod found with the given identifier.
    #[error("Pod not found: {0}")]
    PodNotFound(String),

    /// Encryption/decryption failed.
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// A backup operation is already in progress — retry later.
    #[error("Backup already in progress — another snapshot, revert, prune, or spawn is running")]
    BackupInProgress,
}

impl From<BackupError> for hkask_services_core::ServiceError {
    fn from(e: BackupError) -> Self {
        let msg = e.to_string();
        hkask_services_core::ServiceError::Backup {
            source: Some(Box::new(e)),
            message: msg,
        }
    }
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

    /// Derived AES-256-GCM key (if encryption is configured).
    encryption_key: Option<[u8; 32]>,

    /// Mutual exclusion gate — true while a mutating backup operation is running.
    in_progress: Arc<AtomicBool>,
}

impl BackupService {
    /// Create a new backup service wrapping a CAS port with explicit config.
    ///
    /// Config is provided by the caller — no hidden filesystem I/O.
    /// Use [`crate::config::load_backup_config`] to load from disk.
    ///
    /// If an encryption passphrase is available via the `HKASK_BACKUP_PASSPHRASE`
    /// env var or OS keychain, encryption is enabled automatically.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cas must be a valid GitCASPort; config must be a valid BackupConfig
    /// post: returns BackupService with provided config and encryption key derived if passphrase available
    pub fn new(cas: Arc<dyn GitCASPort>, config: BackupConfig) -> Self {
        let encryption_key = Self::derive_key(&config);
        // CNS: warn if encryption is configured but key derivation failed
        if config.encryption.is_some() && encryption_key.is_none() {
            warn!(
                target: "cns.backup",
                operation = "encryption.key_derive_failed",
                "HKASK_BACKUP_PASSPHRASE missing or salt invalid — blobs will be unencrypted"
            );
        }
        Self {
            cas,
            config,
            encryption_key,
            in_progress: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Derive AES-256 key from the configured passphrase.
    fn derive_key(config: &BackupConfig) -> Option<[u8; 32]> {
        let enc = config.encryption.as_ref()?;
        let passphrase = std::env::var("HKASK_BACKUP_PASSPHRASE").ok()?;
        let salt = hex::decode(&enc.salt_hex).ok()?;
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
            .ok()?;
        Some(key)
    }

    /// Acquire the backup gate. Returns `Ok(())` if no operation is in
    /// progress, or `Err(BackupInProgress)` if another operation holds it.
    ///
    /// The caller MUST call `release_gate()` after the operation completes.
    fn acquire_gate(&self) -> Result<(), BackupError> {
        if self
            .in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(BackupError::BackupInProgress);
        }
        Ok(())
    }

    fn release_gate(&self) {
        self.in_progress.store(false, Ordering::Release);
    }

    // ── Config queries (for BackupLoop and external consumers) ───────────

    /// Should the daemon loop run automatic snapshots?
    pub fn auto_snapshot_enabled(&self) -> bool {
        self.config.auto_snapshot
    }

    /// Should integrity verification run after each snapshot?
    pub fn verify_after_snapshot_enabled(&self) -> bool {
        self.config.verify_after_snapshot
    }

    /// Is a retention policy configured (should pruning run)?
    pub fn retention_configured(&self) -> bool {
        self.config.retention.is_some()
    }

    /// Get a clone of the mutual-exclusion gate for sharing with PodBackupOps.
    pub fn gate(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.in_progress)
    }

    /// Access the underlying CAS port (for artifact producers).
    pub(crate) fn cas(&self) -> &Arc<dyn GitCASPort> {
        &self.cas
    }

    /// Create a PodBackupOps sharing this service's CAS port, encryption key, and gate.
    pub fn pod_ops(&self) -> crate::pod_ops::PodBackupOps {
        crate::pod_ops::PodBackupOps::new(
            Arc::clone(&self.cas) as Arc<dyn GitCASPort>,
            self.encryption_key,
            self.gate(),
        )
    }

    // ── Public API (9 operations) ────────────────────────────────────────

    /// 1. Snapshot artifacts to git.
    ///
    /// Resolves the scope to concrete artifacts, serializes each to a
    /// deterministic blob, stores via `put_blob`, then commits via `snapshot`.
    /// The commit DAG IS the changelog — each commit records what changed.
    ///
    /// CNS span: `backup.snapshot` — records artifact_count, repos, duration_ms.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  scope must be a valid BackupScope; artifacts must be non-empty after filtering tracked types
    /// post: returns SnapshotMetadata with commits, artifact_count, trigger=Manual, and timestamp; Err(NoSnapshots) if no artifacts after filtering; Err(Config) if scope types not tracked
    #[instrument(skip(self, artifacts), fields(artifact_count, repo_count))]
    pub async fn snapshot(
        &self,
        scope: BackupScope,
        artifacts: &[(ArtifactType, String, Vec<u8>)],
    ) -> Result<SnapshotMetadata, BackupError> {
        self.acquire_gate()?;
        let _guard = GateGuard { service: self };
        let start = Instant::now();
        // Validate: all artifact types in scope must be tracked
        self.validate_scope(&scope)?;

        // Group artifacts by repo
        let mut by_repo: std::collections::HashMap<RepoId, Vec<(String, Vec<u8>)>> =
            std::collections::HashMap::new();
        let mut artifact_count = 0usize;

        for (artifact_type, artifact_id, bytes) in artifacts {
            if !self.is_tracked(artifact_type) {
                continue;
            }
            let repo_id = artifact_type.repo_id();

            // Encrypt if configured.
            let encrypted = if self.encryption_key.is_some() {
                encrypt_blob(&self.encryption_key, bytes)?
            } else {
                bytes.clone()
            };

            by_repo
                .entry(repo_id)
                .or_default()
                .push((artifact_id.to_string(), encrypted));
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

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::Span::current().record("artifact_count", artifact_count);
        tracing::Span::current().record("repo_count", by_repo.len());
        info!(
            target: "cns.backup",
            artifact_count = artifact_count,
            repo_count = by_repo.len(),
            duration_ms = duration_ms,
            "CNS"
        );

        Ok(SnapshotMetadata {
            commits,
            artifact_count: Some(artifact_count),
            trigger: Some(SnapshotTrigger::Manual),
            timestamp: Utc::now(),
        })
    }

    /// 2. Restore artifacts from a prior snapshot.
    ///
    /// Resolves the target commit, lists the tree at that commit filtered
    /// by scope, reads each blob, and returns the deserialized artifact data.
    /// Callers are responsible for writing restored data back to the
    /// appropriate store (registry, memory, etc.).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  target must be a valid CommitHash; scope must be a valid RestoreScope
    /// post: returns Vec<(ArtifactType, String, `Vec<u8>`)> of restored artifacts; empty Vec if none match; Err on CAS or deserialization failure
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
                let raw = self.cas.get_blob(repo_id, &entry.content_hash).await?;

                // Decrypt if encrypted.
                let blob = if self.encryption_key.is_some() {
                    decrypt_blob(&self.encryption_key, &raw)?
                } else {
                    raw
                };

                // Parse the envelope to extract artifact type and ID
                let envelope: crate::serialization::ArtifactEnvelopeValue =
                    serde_json::from_slice(&blob).map_err(|e| {
                        BackupError::Serialization(format!(
                            "Failed to deserialize artifact at {}: {e}",
                            entry.path
                        ))
                    })?;

                // Resolve artifact type from the envelope label
                let artifact_type =
                    ArtifactType::from_str(&envelope.artifact_type).map_err(|_| {
                        BackupError::Serialization(format!(
                            "Unknown artifact type in blob: {}",
                            envelope.artifact_type
                        ))
                    })?;

                // If scoped to specific IDs, filter
                if let RestoreScope::ByIds { ref ids, .. } = scope
                    && !ids.contains(&envelope.artifact_id)
                {
                    continue;
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
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  filter.limit defaults to 20 if None
    /// post: returns `Vec<SnapshotMetadata>` sorted by timestamp descending, truncated to limit; Err(NoSnapshots) if no snapshots found
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
                    artifact_count: None,
                    trigger: None,
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

    /// 4. Prune expired snapshots per the 3-tier retention policy.
    ///
    /// Retention: daily snapshots kept for 3 weeks, then weekly for 3 months,
    /// then monthly beyond. In dry-run mode, reports what WOULD be removed.
    /// In execute mode, rewrites git history to remove pruned commits.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  retention policy must be configured; dry_run=true only reports, dry_run=false executes pruning
    /// post: returns PruneReport with evaluated count, removed commits, and retained count; empty report if no retention policy configured
    pub async fn prune(&self, dry_run: bool) -> Result<PruneReport, BackupError> {
        // Only gate actual (non-dry-run) prunes — dry runs are read-only.
        let _guard = if !dry_run {
            self.acquire_gate()?;
            Some(GateGuard { service: self })
        } else {
            None
        };
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
        let mut total_evaluated = 0usize;
        let mut total_removed = Vec::new();
        let mut total_retained = 0usize;

        let now_secs = Utc::now().timestamp() as u64;

        for repo_id in &repos {
            let mut entries: Vec<LogEntry> = self.cas.log(repo_id, 1000).await?;
            entries.sort_by_key(|e| e.timestamp_secs);
            total_evaluated += entries.len();

            let count = entries.len();
            let mut repo_removed: Vec<CommitHash> = Vec::new();
            for (i, entry) in entries.iter().enumerate() {
                let commit_index = count - 1 - i;
                if policy.should_keep(commit_index, entry.timestamp_secs, now_secs) {
                    total_retained += 1;
                } else {
                    repo_removed.push(entry.commit.clone());
                    total_removed.push((repo_id.clone(), entry.commit.clone()));
                }
            }

            // Execute actual pruning if not dry run and this repo has removals.
            if !dry_run && !repo_removed.is_empty() {
                self.rewrite_history(repo_id, &entries, &policy, now_secs)
                    .await?;
            }
        }

        Ok(PruneReport {
            dry_run,
            evaluated: total_evaluated,
            removed: total_removed,
            retained: total_retained,
        })
    }

    /// Rewrite git history: delete pruned blobs from CAS, collect retained
    /// blobs, create an orphan commit with no parent.
    async fn rewrite_history(
        &self,
        repo_id: &RepoId,
        entries: &[LogEntry],
        policy: &RetentionPolicy,
        now_secs: u64,
    ) -> Result<(), BackupError> {
        let count = entries.len();

        // Collect ContentHashes from retained and pruned commits.
        let mut retained_hashes: HashSet<hkask_ports::git_cas::ContentHash> = HashSet::new();
        let mut pruned_hashes: HashSet<hkask_ports::git_cas::ContentHash> = HashSet::new();

        for (i, entry) in entries.iter().enumerate() {
            let commit_index = count - 1 - i;
            let tree_entries = self
                .cas
                .list_tree(repo_id, &entry.commit.to_string(), "")
                .await?;
            let hashes: Vec<_> = tree_entries
                .iter()
                .map(|te| te.content_hash.clone())
                .collect();
            if policy.should_keep(commit_index, entry.timestamp_secs, now_secs) {
                retained_hashes.extend(hashes);
            } else {
                pruned_hashes.extend(hashes);
            }
        }

        // Delete blobs that are ONLY in pruned commits (not shared with retained).
        for hash in pruned_hashes.difference(&retained_hashes) {
            self.cas.delete_blob(repo_id, hash).await?;
        }

        // Create an orphan commit with remaining blobs in cas/.
        let new_commit = self
            .cas
            .snapshot_orphan(repo_id, "backup: history pruned (retained snapshots)")
            .await?;

        info!(
            target: "cns.backup",
            repo = %repo_id.dir_name(),
            new_head = %new_commit,
            deleted = pruned_hashes.len() - retained_hashes.intersection(&pruned_hashes).count(),
            "CNS"
        );

        Ok(())
    }

    /// 5. Verify integrity of all tracked repositories.
    ///
    /// Delegates to [`GitCASPort::verify`] for each tracked repo.
    /// Returns the combined verification report.
    ///
    /// CNS span: `backup.verify` — records total_blobs, corrupt_count per repo.
    /// CNS alert: `backup.integrity_failure` if any repo has corrupt blobs.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  tracked repos must be accessible via CAS
    /// post: returns `Vec<VerificationReport>` per repo with total_blobs and corrupt_hashes; empty Vec if no tracked repos
    #[instrument(skip(self), fields(repo_count, total_blobs, corrupt_count))]
    pub async fn verify(
        &self,
    ) -> Result<Vec<hkask_ports::git_cas::VerificationReport>, BackupError> {
        let repos = self.tracked_repos();
        let mut reports = Vec::new();
        let mut total_blobs = 0usize;
        let mut corrupt_count = 0usize;

        for repo_id in &repos {
            let report = self.cas.verify(repo_id).await?;
            total_blobs += report.total_blobs;
            corrupt_count += report.corrupt_hashes.len();
            if !report.corrupt_hashes.is_empty() {
                warn!(
                    target: "cns.backup",
                    repo = %repo_id.dir_name(),
                    corrupt = report.corrupt_hashes.len(),
                    total = report.total_blobs,
                    "CNS"
                );
            }
            reports.push(report);
        }

        tracing::Span::current().record("repo_count", repos.len());
        tracing::Span::current().record("total_blobs", total_blobs);
        tracing::Span::current().record("corrupt_count", corrupt_count);

        if corrupt_count > 0 {
            warn!(
                target: "cns.backup",
                repo_count = repos.len(),
                total_blobs = total_blobs,
                corrupt_count = corrupt_count,
                "CNS"
            );
        } else {
            info!(
                target: "cns.backup",
                repo_count = repos.len(),
                total_blobs = total_blobs,
                "CNS"
            );
        }

        Ok(reports)
    }

    /// 6. Get current backup configuration.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  none (always succeeds)
    /// post: returns reference to current BackupConfig
    pub fn config(&self) -> &BackupConfig {
        &self.config
    }

    /// 7. Update backup configuration and persist to disk.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be a valid BackupConfig
    /// post: config is persisted to disk and self.config is updated; encryption key is re-derived; Err(Config) on save failure
    pub fn update_config(&mut self, config: BackupConfig) -> Result<(), BackupError> {
        self.encryption_key = Self::derive_key(&config);
        crate::config::save_backup_config(&config)
            .map_err(|e| BackupError::Config(format!("Failed to save config: {e}")))?;
        self.config = config;
        Ok(())
    }

    /// Enable encryption with a passphrase.
    /// Generates a random salt, derives the key, and saves the config.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  passphrase must be non-empty
    /// post: encryption is enabled with random salt; config is persisted; encryption_key is derived; Err(Config) on save failure; Err(Encryption) on Argon2 failure
    pub fn enable_encryption(&mut self, passphrase: &str) -> Result<(), BackupError> {
        let mut salt = [0u8; 32];
        rng().fill_bytes(&mut salt);
        let salt_hex = hex::encode(salt);

        self.config.encryption = Some(EncryptionConfig {
            salt_hex,
            memory_kb: 19456, // Argon2 default
            iterations: 2,    // Argon2 default
        });
        crate::config::save_backup_config(&self.config)
            .map_err(|e| BackupError::Config(format!("Failed to save config: {e}")))?;

        // Derive key from the new passphrase.
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
            .map_err(|e| BackupError::Encryption(format!("Argon2: {e}")))?;
        self.encryption_key = Some(key);
        Ok(())
    }

    /// Run a daily backup snapshot of all tracked artifact types.
    /// Called by the backup scheduler (daemon loop).
    ///
    /// Snapshots ALL repos (not just tracked types) because artifact
    /// producers may push blobs to any repo. The tracking config controls
    /// which types are collected, not which repos are committed.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  auto_snapshot must be enabled in config
    /// post: returns SnapshotMetadata from full snapshot of all repos;
    ///       Err on CAS failure
    pub async fn run_daily_snapshot(&self) -> Result<SnapshotMetadata, BackupError> {
        self.acquire_gate()?;
        let _guard = GateGuard { service: self };
        info!(target: "cns.backup", "CNS");

        let repos = hkask_ports::git_cas::RepoId::all();
        let mut commits = Vec::new();
        for repo_id in repos {
            let message = format!(
                "backup: daily snapshot — {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S")
            );
            let commit_hash = self.cas.snapshot(repo_id, &message).await?;
            commits.push((repo_id.clone(), commit_hash));
        }

        info!(
            target: "cns.backup",
            repo_count = repos.len(),
            operation = "daily_snapshot",
            "CNS"
        );

        Ok(SnapshotMetadata {
            commits,
            artifact_count: None,
            trigger: Some(SnapshotTrigger::Auto),
            timestamp: Utc::now(),
        })
    }

    /// Restore artifacts at a specific scope level.
    ///
    /// - `RestoreScope::Full`: restore ALL tracked artifact types (system-level)
    /// - `RestoreScope::ByType`: restore all artifacts of one type (registry-level)
    /// - `RestoreScope::ByIds`: restore specific artifacts by ID (file-level)
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  target must be a valid CommitHash; scope must be a valid RestoreScope
    /// post: returns Vec of restored artifacts matching the scope; delegates to restore()
    pub async fn scoped_restore(
        &self,
        target: &CommitHash,
        scope: RestoreScope,
    ) -> Result<Vec<(ArtifactType, String, Vec<u8>)>, BackupError> {
        match &scope {
            RestoreScope::Full => {
                info!(target: "cns.backup", commit=%target, "CNS");
            }
            RestoreScope::ByType(at) => {
                info!(target: "cns.backup", commit=%target, artifact_type=%at.label(), "CNS");
            }
            RestoreScope::ByIds { artifact_type, ids } => {
                info!(target: "cns.backup", commit=%target, artifact_type=%artifact_type.label(), ids=?ids, "CNS");
            }
        }
        self.restore(target, scope).await
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

/// Encrypt blob content with AES-256-GCM.
/// Returns (nonce_bytes || ciphertext).
pub(crate) fn encrypt_blob(key: &Option<[u8; 32]>, data: &[u8]) -> Result<Vec<u8>, BackupError> {
    let key = key.as_ref().ok_or_else(|| {
        warn!(
            target: "cns.backup",
            operation = "encryption.encrypt_failed",
            "Encryption not configured — storing blob unencrypted"
        );
        BackupError::Encryption("Encryption not configured".into())
    })?;
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| BackupError::Encryption(format!("AES init: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| BackupError::Encryption(format!("AES encrypt: {e}")))?;
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt blob content.
/// Expects (nonce_bytes || ciphertext).
pub(crate) fn decrypt_blob(key: &Option<[u8; 32]>, data: &[u8]) -> Result<Vec<u8>, BackupError> {
    let key = key.as_ref().ok_or_else(|| {
        warn!(
            target: "cns.backup",
            operation = "encryption.decrypt_failed",
            "Encryption not configured — cannot decrypt blob"
        );
        BackupError::Encryption("Encryption not configured".into())
    })?;
    if data.len() < 12 {
        return Err(BackupError::Encryption("Data too short for nonce".into()));
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| BackupError::Encryption(format!("AES init: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| BackupError::Encryption(format!("AES decrypt: {e}")))
}

/// RAII guard that releases the backup gate on drop.
/// Ensures the gate is released even if the operation panics.
struct GateGuard<'a> {
    service: &'a BackupService,
}
impl Drop for GateGuard<'_> {
    fn drop(&mut self) {
        self.service.release_gate();
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RetentionPolicy;
    use crate::serialization::serialize_artifact;
    use hkask_ports::git_cas::MockGitCas;

    fn test_config() -> BackupConfig {
        BackupConfig {
            tracked_types: vec![ArtifactType::Template, ArtifactType::Goal],
            retention: None,
            auto_snapshot: true,
            verify_after_snapshot: false,
            encryption: None,
        }
    }

    fn test_service() -> BackupService {
        let mock = Arc::new(MockGitCas::new());
        BackupService::new(mock, test_config())
    }

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
        assert_eq!(result.artifact_count, Some(1));
        assert!(!result.commits.is_empty());
    }

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

    #[tokio::test]
    async fn full_snapshot_no_tracked_types_errors() {
        let mock = Arc::new(MockGitCas::new());
        let svc = BackupService::new(mock, BackupConfig::default());
        let result = svc.snapshot(BackupScope::Full, &[]).await;
        assert!(matches!(result, Err(BackupError::Config(_))));
    }

    #[tokio::test]
    async fn restore_reproduces_state() {
        let mock = Arc::new(MockGitCas::new());
        let svc = BackupService::new(mock.clone(), test_config());

        // First, snapshot properly serialized data
        let payload = serde_json::json!({"name": "test-tpl"});
        let data = serialize_artifact(&ArtifactType::Template, "tpl-1", &payload).unwrap();
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

    #[tokio::test]
    async fn prune_with_retention_identifies_expired() {
        let mock = Arc::new(MockGitCas::new());
        let config = BackupConfig {
            tracked_types: vec![ArtifactType::Template],
            retention: Some(RetentionPolicy {
                daily_days: 1,
                weekly_weeks: 1,
            }),
            auto_snapshot: true,
            verify_after_snapshot: false,
            encryption: None,
        };
        let svc = BackupService::new(mock.clone(), config);

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
        // The most recent snapshot (index 0) is always kept.
        assert_eq!(report.retained, 1);
    }

    #[tokio::test]
    async fn verify_returns_reports() {
        let svc = test_service();
        let reports = svc.verify().await.unwrap();
        // MockGitCas returns empty reports for repos with no blobs
        assert!(!reports.is_empty());
    }

    #[tokio::test]
    async fn update_config_persists_and_reflects() {
        let mut svc = test_service();
        let new_config = BackupConfig {
            tracked_types: vec![ArtifactType::MemoryTriple],
            encryption: None,
            ..test_config()
        };
        svc.update_config(new_config.clone()).unwrap();
        assert_eq!(svc.config().tracked_types, new_config.tracked_types);
    }
}
