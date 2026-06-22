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
//! 8. `revert` — revert a pod to a prior snapshot with safety snapshot
//! 9. `spawn_agent` — fork a new agent pod from a prior snapshot

use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::config::{BackupConfig, EncryptionConfig, RetentionPolicy};
use crate::metadata::{
    PruneReport, RevertReport, SnapshotMetadata, SnapshotTrigger, SpawnAgentReport,
};
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
    in_progress: AtomicBool,
}

impl BackupService {
    /// Create a new backup service wrapping a CAS port.
    ///
    /// The config is loaded from disk via [`crate::config::load_backup_config`]
    /// at construction time. Use [`Self::update_config`] to change it.
    ///
    /// If an encryption passphrase is available via the `HKASK_BACKUP_PASSPHRASE`
    /// env var or OS keychain, encryption is enabled automatically.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cas must be a valid GitCASPort
    /// post: returns BackupService with config loaded from disk and encryption key derived if passphrase available
    pub fn new(cas: Arc<dyn GitCASPort>) -> Self {
        let config = crate::config::load_backup_config();
        let encryption_key = Self::derive_key(&config);
        Self {
            cas,
            config,
            encryption_key,
            in_progress: AtomicBool::new(false),
        }
    }

    /// Create a new backup service with an explicit config (for testing).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cas must be a valid GitCASPort; config must be a valid BackupConfig
    /// post: returns BackupService with explicit config and derived encryption key
    pub fn with_config(cas: Arc<dyn GitCASPort>, config: BackupConfig) -> Self {
        let encryption_key = Self::derive_key(&config);
        Self {
            cas,
            config,
            encryption_key,
            in_progress: AtomicBool::new(false),
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

    /// Encrypt blob content with AES-256-GCM.
    /// Returns (nonce_bytes || ciphertext).
    fn encrypt_blob(&self, data: &[u8]) -> Result<Vec<u8>, BackupError> {
        let key = self
            .encryption_key
            .as_ref()
            .ok_or_else(|| BackupError::Encryption("Encryption not configured".into()))?;
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
    fn decrypt_blob(&self, data: &[u8]) -> Result<Vec<u8>, BackupError> {
        let key = self
            .encryption_key
            .as_ref()
            .ok_or_else(|| BackupError::Encryption("Encryption not configured".into()))?;
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
                self.encrypt_blob(bytes)?
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
                    self.decrypt_blob(&raw)?
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

    /// Rewrite git history: collect retained blobs, create an orphan commit
    /// with no parent, effectively starting a new pruned history chain.
    async fn rewrite_history(
        &self,
        repo_id: &RepoId,
        entries: &[LogEntry],
        policy: &RetentionPolicy,
        now_secs: u64,
    ) -> Result<(), BackupError> {
        let count = entries.len();

        // Collect blobs from retained commits only.
        for (i, entry) in entries.iter().enumerate() {
            let commit_index = count - 1 - i;
            if !policy.should_keep(commit_index, entry.timestamp_secs, now_secs) {
                continue;
            }
            let tree_entries = self
                .cas
                .list_tree(repo_id, &entry.commit.to_string(), "")
                .await?;
            for te in &tree_entries {
                let blob = self.cas.get_blob(repo_id, &te.content_hash).await?;
                self.cas.put_blob(repo_id, &blob).await?;
            }
        }

        // Create an orphan commit with all retained blobs — no parent,
        // effectively pruning old history by starting a new chain.
        let new_commit = self
            .cas
            .snapshot_orphan(repo_id, "backup: history pruned (retained snapshots)")
            .await?;

        info!(
            target: "cns.backup",
            repo = %repo_id.dir_name(),
            new_head = %new_commit,
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
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  auto_snapshot must be enabled in config
    /// post: returns SnapshotMetadata from full snapshot; Err on snapshot failure
    pub async fn run_daily_snapshot(&self) -> Result<SnapshotMetadata, BackupError> {
        self.acquire_gate()?;
        let _guard = GateGuard { service: self };
        info!(target: "cns.backup", "CNS");
        // Snapshot all tracked types. Artifact data is collected by
        // scanning all repos for current state — the caller provides
        // artifacts. For the scheduler, we snapshot whatever is in
        // the CAS repos (put there by prior artifact writes).
        self.snapshot(BackupScope::Full, &[]).await
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

    // ── Pod revert / spawn_agent (operations 8–9) ────────────────────────

    /// 8. Revert a pod to a prior snapshot.
    ///
    /// Takes a safety snapshot of the current pod state before restoring
    /// the target commit. The safety snapshot is the bail-out point —
    /// restore it to undo the revert.
    ///
    /// CNS span: `cns.agent_pod.revert` — records pod_id, safety_commit,
    /// target_commit, reason.
    ///
    /// pre:  pod_id is a non-empty pod identifier; target_commit is a valid
    ///       CommitHash in the Pods repo; pod_db_path must exist
    /// post: returns RevertReport with safety_commit, target_commit, artifact_count;
    ///       a safety snapshot was created BEFORE the restore;
    ///       pod_db_path now contains state from target_commit;
    ///       cns.agent_pod.revert span emitted
    #[instrument(skip(self), fields(pod_id, safety_commit, target_commit))]
    pub async fn revert(
        &self,
        pod_id: &str,
        target_commit: &CommitHash,
        pod_db_path: &Path,
        reason: &str,
    ) -> Result<RevertReport, BackupError> {
        self.acquire_gate()?;
        let _guard = GateGuard { service: self };
        let start = Instant::now();

        // 1. Safety snapshot: capture current pod state BEFORE revert
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "revert.safety_snapshot",
            reason = reason,
            "CNS"
        );

        if !pod_db_path.exists() {
            return Err(BackupError::PodNotFound(pod_id.to_string()));
        }
        let current_state = std::fs::read(pod_db_path)
            .map_err(|e| BackupError::Config(format!("Failed to read pod.db: {e}")))?;

        // Serialize the safety snapshot as PodState artifact
        let safety_artifact = crate::serialization::serialize_artifact(
            &ArtifactType::PodState,
            &format!("{pod_id}-safety"),
            &serde_json::json!({"pod_id": pod_id, "reason": reason}),
        )
        .map_err(|e| BackupError::Serialization(format!("Safety snapshot: {e}")))?;

        let safety_blob = if self.encryption_key.is_some() {
            self.encrypt_blob(&safety_artifact)?
        } else {
            safety_artifact
        };

        let repo_id = ArtifactType::PodState.repo_id();
        self.cas.put_blob(&repo_id, &safety_blob).await?;
        // Also store the raw pod.db for full-state preservation
        let pod_blob = if self.encryption_key.is_some() {
            self.encrypt_blob(&current_state)?
        } else {
            current_state
        };
        self.cas.put_blob(&repo_id, &pod_blob).await?;
        let safety_commit = self
            .cas
            .snapshot(
                &repo_id,
                &format!(
                    "revert: safety snapshot for {pod_id} — {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S")
                ),
            )
            .await?;

        tracing::Span::current().record("safety_commit", safety_commit.to_string());
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "revert.safety_complete",
            safety_commit = %safety_commit,
            "CNS"
        );

        // 2. Restore target state to pod.db
        let target_str = target_commit.to_string();
        let prefix = format!("{}/", ArtifactType::PodState.label());
        let entries = self.cas.list_tree(&repo_id, &target_str, &prefix).await?;

        if entries.is_empty() {
            return Err(BackupError::NoSnapshots);
        }

        let mut restored_count = 0usize;
        for entry in entries {
            let raw = self.cas.get_blob(&repo_id, &entry.content_hash).await?;
            let blob = if self.encryption_key.is_some() {
                self.decrypt_blob(&raw)?
            } else {
                raw
            };

            // Try envelope first; if it fails, treat as raw pod.db blob
            if serde_json::from_slice::<crate::serialization::ArtifactEnvelopeValue>(&blob).is_ok()
            {
                continue; // Skip envelope blobs, find the raw pod.db
            } else {
                std::fs::write(pod_db_path, &blob)
                    .map_err(|e| BackupError::Config(format!("Failed to write pod.db: {e}")))?;
                restored_count += 1;
                break;
            }
        }

        if restored_count == 0 {
            return Err(BackupError::NoSnapshots);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::Span::current().record("target_commit", target_str);
        tracing::Span::current().record("pod_id", pod_id);
        info!(
            target: "cns.agent_pod",
            pod_id = pod_id,
            operation = "revert",
            safety_commit = %safety_commit,
            target_commit = %target_commit,
            reason = reason,
            duration_ms = duration_ms,
            "CNS"
        );

        Ok(RevertReport {
            pod_id: pod_id.to_string(),
            safety_commit,
            target_commit: target_commit.clone(),
            artifact_count: restored_count,
            timestamp: Utc::now(),
        })
    }

    /// 9. Spawn a new agent pod from a prior snapshot.
    ///
    /// Restores pod state from a source commit to a new pod location.
    /// The caller is responsible for identity assignment (new WebID/PodID).
    ///
    /// This is DISTINCT from kanban's `spawn_subagent` which creates
    /// sub-agent replicants for task delegation. `spawn_agent` is a
    /// full pod fork — new identity, new pod.db, sovereign agent.
    ///
    /// CNS span: `cns.agent_pod.spawn` — records source_pod_id,
    /// new_pod_id, source_commit.
    ///
    /// pre:  source_commit is a valid CommitHash in the Pods repo;
    ///       output_db_path parent directory must exist
    /// post: returns SpawnAgentReport with restored state written to output_db_path;
    ///       cns.agent_pod.spawn span emitted
    #[instrument(skip(self), fields(source_pod_id, new_pod_id, source_commit))]
    pub async fn spawn_agent(
        &self,
        source_pod_id: &str,
        source_commit: &CommitHash,
        new_pod_id: &str,
        output_db_path: &Path,
    ) -> Result<SpawnAgentReport, BackupError> {
        self.acquire_gate()?;
        let _guard = GateGuard { service: self };
        let start = Instant::now();
        let repo_id = ArtifactType::PodState.repo_id();
        let target_str = source_commit.to_string();
        let prefix = format!("{}/", ArtifactType::PodState.label());
        let entries = self.cas.list_tree(&repo_id, &target_str, &prefix).await?;

        if entries.is_empty() {
            return Err(BackupError::NoSnapshots);
        }

        let mut restored = false;
        for entry in entries {
            let raw = self.cas.get_blob(&repo_id, &entry.content_hash).await?;
            let blob = if self.encryption_key.is_some() {
                self.decrypt_blob(&raw)?
            } else {
                raw
            };

            // Skip envelope blobs, find the raw pod.db
            if serde_json::from_slice::<crate::serialization::ArtifactEnvelopeValue>(&blob).is_ok()
            {
                continue;
            } else {
                std::fs::write(output_db_path, &blob)
                    .map_err(|e| BackupError::Config(format!("Failed to write new pod.db: {e}")))?;
                restored = true;
                break;
            }
        }

        if !restored {
            return Err(BackupError::NoSnapshots);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::Span::current().record("source_pod_id", source_pod_id);
        tracing::Span::current().record("new_pod_id", new_pod_id);
        tracing::Span::current().record("source_commit", target_str);
        info!(
            target: "cns.agent_pod",
            operation = "spawn",
            source_pod_id = source_pod_id,
            new_pod_id = new_pod_id,
            source_commit = %source_commit,
            duration_ms = duration_ms,
            "CNS"
        );

        Ok(SpawnAgentReport {
            source_pod_id: source_pod_id.to_string(),
            new_pod_id: new_pod_id.to_string(),
            source_commit: source_commit.clone(),
            new_db_path: output_db_path.to_string_lossy().to_string(),
            timestamp: Utc::now(),
        })
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
        BackupService::with_config(mock, test_config())
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
        assert_eq!(result.artifact_count, 1);
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
        let svc = BackupService::with_config(mock, BackupConfig::default());
        let result = svc.snapshot(BackupScope::Full, &[]).await;
        assert!(matches!(result, Err(BackupError::Config(_))));
    }

    #[tokio::test]
    async fn restore_reproduces_state() {
        let mock = Arc::new(MockGitCas::new());
        let svc = BackupService::with_config(mock.clone(), test_config());

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
