//! Gix-based Git CAS Adapter
//!
//! Implements [`GitCASPort`] using the `gix` crate for git operations
//! and BLAKE3-addressed files for blob storage.
//!
//! Each [`RepoId`] maps to a directory under the base path containing:
//! - `cas/` — BLAKE3-addressed blob files (content = filename, BLAKE3 hex)
//! - `.git/` — bare git repository for snapshots and refs

use hkask_types::ports::git_cas::{
    CommitHash, ContentHash, DiffKind, FileDiff, GitCASPort, GitCasError, LogEntry, RepoId,
    TreeEntry, TreeEntryKind, VerificationReport,
};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// Gix-based adapter implementing [`GitCASPort`].
///
/// Manages multiple directories (one per [`RepoId`]) under a base path.
/// Blobs are stored as files named by their BLAKE3 hex hash in a `cas/`
/// subdirectory. Git operations (snapshot, resolve_ref, etc.) use gix.
pub struct GixCasAdapter {
    base_path: PathBuf,
    /// Cache of which repos have been initialized (dir_name -> whether init happened)
    initialized: RwLock<std::collections::HashSet<String>>,
}

/// Resolve the CAS home directory from environment or default.
///
/// Resolution order:
/// 1. `HKASK_CAS_HOME` env var (explicit path)
/// 2. `~/.hkask/repos/` (default)
pub(crate) fn resolve_cas_home() -> PathBuf {
    std::env::var("HKASK_CAS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".hkask").join("repos")
        })
}

impl GixCasAdapter {
    /// Create a new adapter rooted at `base_path`.
    ///
    /// The base path is created if it doesn't exist.
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, GitCasError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)
            .map_err(|e| GitCasError::Io(format!("Failed to create base path: {e}")))?;
        Ok(Self {
            base_path,
            initialized: RwLock::new(std::collections::HashSet::new()),
        })
    }

    /// Create a new adapter using the HKASK_CAS_HOME environment variable.
    ///
    /// Resolution order:
    /// 1. `HKASK_CAS_HOME` env var (explicit path)
    /// 2. `~/.hkask/repos/` (default)
    pub fn from_env() -> Result<Self, GitCasError> {
        let base_path = resolve_cas_home();
        Self::new(base_path)
    }

    /// Ensure the repo directory and CAS subdirectory exist.
    async fn ensure_repo_dir(&self, repo: &RepoId) -> Result<PathBuf, GitCasError> {
        let dir_name = repo.dir_name().to_string();
        {
            let init = self.initialized.read().await;
            if init.contains(&dir_name) {
                return Ok(self.base_path.join(&dir_name));
            }
        }

        let repo_path = self.base_path.join(&dir_name);
        let cas_path = repo_path.join("cas");

        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&cas_path)
                .map_err(|e| GitCasError::Io(format!("Failed to create CAS dir: {e}")))?;
            Ok(repo_path)
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))??;

        let mut init = self.initialized.write().await;
        init.insert(dir_name);
        Ok(self.base_path.join(repo.dir_name()))
    }

    /// Validate a path component to prevent directory traversal.
    ///
    /// Rejects paths containing `..`, null bytes, or absolute paths.
    #[allow(dead_code)]
    fn validate_path(path: &Path) -> Result<(), GitCasError> {
        let path_str = path.to_string_lossy();

        if path_str.contains('\0') {
            return Err(GitCasError::PathValidation(
                "Path contains null bytes".to_string(),
            ));
        }

        if path.is_absolute() {
            return Err(GitCasError::PathValidation(
                "Absolute paths not allowed".to_string(),
            ));
        }

        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                return Err(GitCasError::PathValidation(
                    "Parent directory traversal not allowed".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl GitCASPort for GixCasAdapter {
    async fn put_blob(&self, repo: &RepoId, content: &[u8]) -> Result<ContentHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let hash = ContentHash::from_blake3(content);
        let cas_dir = repo_dir.join("cas");
        let blob_path = cas_dir.join(hash.to_string());
        let content = content.to_vec();

        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&cas_dir)
                .map_err(|e| GitCasError::Io(format!("Failed to create CAS dir: {e}")))?;
            std::fs::write(&blob_path, &content)
                .map_err(|e| GitCasError::Io(format!("Failed to write blob: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))??;

        Ok(hash)
    }

    async fn get_blob(&self, repo: &RepoId, hash: &ContentHash) -> Result<Vec<u8>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let blob_path = repo_dir.join("cas").join(hash.to_string());

        tokio::task::spawn_blocking(move || {
            std::fs::read(&blob_path)
                .map_err(|e| GitCasError::NotFound(format!("Blob not found: {e}")))
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }

    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let msg = message.to_string();

        tokio::task::spawn_blocking(move || {
            // Lazy git init: if no .git directory exists, initialize one.
            if !repo_dir.join(".git").exists() {
                let init_output = std::process::Command::new("git")
                    .args(["init"])
                    .current_dir(&repo_dir)
                    .output()
                    .map_err(|e| GitCasError::Io(format!("git init failed: {e}")))?;

                if !init_output.status.success() {
                    let stderr = String::from_utf8_lossy(&init_output.stderr);
                    return Err(GitCasError::Git(format!(
                        "git init failed: {}",
                        stderr.trim()
                    )));
                }
            }

            // Stage all CAS content
            let output = std::process::Command::new("git")
                .args(["add", "-A"])
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Io(format!("git add failed: {e}")))?;

            if !output.status.success()
                && !String::from_utf8_lossy(&output.stderr).contains("nothing to commit")
            {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // "nothing to commit" is fine — we'll still create a snapshot
                if !stderr.contains("nothing") {
                    return Err(GitCasError::Git(format!(
                        "git add failed: {}",
                        stderr.trim()
                    )));
                }
            }

            let output = std::process::Command::new("git")
                .args(["commit", "-m", &msg])
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Io(format!("git commit failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("nothing to commit") {
                    // No changes — return current HEAD
                    let rev_output = std::process::Command::new("git")
                        .args(["rev-parse", "HEAD"])
                        .current_dir(&repo_dir)
                        .output()
                        .map_err(|e| GitCasError::Io(format!("git rev-parse failed: {e}")))?;

                    let sha_hex = String::from_utf8_lossy(&rev_output.stdout)
                        .trim()
                        .to_string();
                    let bytes = hex::decode(&sha_hex)
                        .map_err(|e| GitCasError::Git(format!("Invalid SHA hex: {e}")))?;
                    let mut arr = [0u8; 20];
                    arr.copy_from_slice(&bytes[..20]);
                    return Ok(CommitHash::from_bytes(arr));
                }
                return Err(GitCasError::Git(format!(
                    "git commit failed: {}",
                    stderr.trim()
                )));
            }

            // Get the commit SHA
            let rev_output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Io(format!("git rev-parse failed: {e}")))?;

            let sha_hex = String::from_utf8_lossy(&rev_output.stdout)
                .trim()
                .to_string();
            let bytes = hex::decode(&sha_hex)
                .map_err(|e| GitCasError::Git(format!("Invalid SHA hex: {e}")))?;
            let mut arr = [0u8; 20];
            arr.copy_from_slice(&bytes[..20]);
            Ok(CommitHash::from_bytes(arr))
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }

    async fn resolve_ref(&self, repo: &RepoId, reference: &str) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let ref_name = reference.to_string();

        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new("git")
                .args(["rev-parse", &ref_name])
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Io(format!("git rev-parse failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(GitCasError::Git(format!(
                    "Failed to resolve ref '{}': {}",
                    ref_name,
                    stderr.trim()
                )));
            }

            let sha_hex = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let bytes = hex::decode(&sha_hex)
                .map_err(|e| GitCasError::Git(format!("Invalid SHA hex: {e}")))?;
            let mut arr = [0u8; 20];
            arr.copy_from_slice(&bytes[..20]);
            Ok(CommitHash::from_bytes(arr))
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }

    async fn list_tree(
        &self,
        repo: &RepoId,
        reference: &str,
        prefix: &str,
    ) -> Result<Vec<TreeEntry>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let ref_name = reference.to_string();
        let prefix_filter = prefix.to_string();

        tokio::task::spawn_blocking(move || {
            let mut args = vec!["ls-tree", &ref_name];
            if !prefix_filter.is_empty() {
                args.push("--");
                args.push(&prefix_filter);
            }

            let output = std::process::Command::new("git")
                .args(&args)
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Io(format!("git ls-tree failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(GitCasError::Git(format!(
                    "git ls-tree failed: {}",
                    stderr.trim()
                )));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut entries = Vec::new();

            for line in stdout.lines() {
                // git ls-tree output format: <mode> <type> <hash>\t<path>
                let parts: Vec<&str> = line.splitn(2, '\t').collect();
                if parts.len() != 2 {
                    continue;
                }
                let meta = parts[0];
                let path = parts[1].to_string();

                let meta_parts: Vec<&str> = meta.split_whitespace().collect();
                if meta_parts.len() < 3 {
                    continue;
                }

                let kind = if meta_parts[1] == "tree" {
                    TreeEntryKind::Tree
                } else {
                    TreeEntryKind::Blob
                };

                // For blobs, read the content and hash it with BLAKE3
                let content_hash = if kind == TreeEntryKind::Blob {
                    let blob_output = std::process::Command::new("git")
                        .args(["cat-file", "-p", meta_parts[2]])
                        .current_dir(&repo_dir)
                        .output()
                        .map_err(|e| GitCasError::Io(format!("git cat-file failed: {e}")))?;
                    ContentHash::from_blake3(&blob_output.stdout)
                } else {
                    // For trees, hash the OID
                    ContentHash::from_blake3(meta_parts[2].as_bytes())
                };

                entries.push(TreeEntry {
                    path,
                    content_hash,
                    kind,
                });
            }

            Ok(entries)
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }

    async fn diff(
        &self,
        repo: &RepoId,
        from: &str,
        to: &str,
    ) -> Result<Vec<FileDiff>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let from_ref = from.to_string();
        let to_ref = to.to_string();

        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new("git")
                .args(["diff", "--name-status", &from_ref, &to_ref])
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Io(format!("git diff failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(GitCasError::Git(format!(
                    "git diff failed: {}",
                    stderr.trim()
                )));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut diffs = Vec::new();

            for line in stdout.lines() {
                let chars: Vec<char> = line.chars().collect();
                if chars.is_empty() {
                    continue;
                }

                let (kind, path) = match chars[0] {
                    'A' => (DiffKind::Added, line[1..].trim()),
                    'D' => (DiffKind::Removed, line[1..].trim()),
                    'M' => (DiffKind::Modified, line[1..].trim()),
                    _ => continue,
                };

                diffs.push(FileDiff {
                    path: path.to_string(),
                    kind,
                    content: String::new(), // Full unified diff would require a separate call
                });
            }

            Ok(diffs)
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }

    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let cas_dir = repo_dir.join("cas");
        let repo_id = repo.clone();

        tokio::task::spawn_blocking(move || {
            if !cas_dir.exists() {
                return Ok(VerificationReport {
                    repo: repo_id,
                    total_blobs: 0,
                    verified_blobs: 0,
                    corrupt_hashes: Vec::new(),
                });
            }

            let entries = std::fs::read_dir(&cas_dir)
                .map_err(|e| GitCasError::Io(format!("Failed to read CAS dir: {e}")))?;

            let mut total = 0usize;
            let mut verified = 0usize;
            let mut corrupt = Vec::new();

            for entry in entries {
                let entry = entry.map_err(|e| GitCasError::Io(format!("Dir entry error: {e}")))?;
                let path = entry.path();

                if path.is_dir() {
                    continue;
                }

                let content = std::fs::read(&path)
                    .map_err(|e| GitCasError::Io(format!("Failed to read blob: {e}")))?;

                let actual_hash = ContentHash::from_blake3(&content);
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let expected_hash: ContentHash = file_name
                    .parse()
                    .map_err(|e: String| GitCasError::Io(format!("Invalid hash filename: {e}")))?;

                total += 1;
                if actual_hash == expected_hash {
                    verified += 1;
                } else {
                    corrupt.push(expected_hash);
                }
            }

            Ok(VerificationReport {
                repo: repo_id,
                total_blobs: total,
                verified_blobs: verified,
                corrupt_hashes: corrupt,
            })
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }

    async fn log(&self, repo: &RepoId, max_count: usize) -> Result<Vec<LogEntry>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let max = max_count;

        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new("git")
                .args([
                    "log",
                    "--oneline",
                    "--pretty=format:%H %ct %s",
                    &format!("-{}", max),
                ])
                .current_dir(&repo_dir)
                .output()
                .map_err(|e| GitCasError::Git(format!("git log failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // No commits yet is not an error — return empty
                if stderr.contains("does not have any commits")
                    || stderr.contains("ambiguous argument")
                {
                    return Ok(Vec::new());
                }
                return Err(GitCasError::Git(format!(
                    "git log failed: {}",
                    stderr.trim()
                )));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut entries = Vec::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // Format: <40-char-hex-sha> <unix-timestamp> <message>
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() < 3 {
                    continue;
                }
                let hash_str = parts[0];
                let timestamp_str = parts[1];
                let message = parts[2].to_string();

                let commit: CommitHash = hash_str.parse().map_err(|e: String| {
                    GitCasError::Git(format!("Invalid commit hash in log: {e}"))
                })?;
                let timestamp_secs = timestamp_str.parse::<u64>().unwrap_or(0);

                entries.push(LogEntry {
                    commit,
                    message,
                    timestamp_secs,
                });
            }
            Ok(entries)
        })
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
