//! Gix-based Git CAS Adapter — implements [`GitCASPort`] with gix + BLAKE3 blob storage.

use hkask_types::ports::git_cas::{
    CommitHash, ContentHash, DiffKind, FileDiff, GitCASPort, GitCasError, LogEntry, RepoId,
    TreeEntry, TreeEntryKind, VerificationReport,
};
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::sync::RwLock;

pub struct GixCasAdapter {
    base_path: PathBuf,
    initialized: RwLock<std::collections::HashSet<String>>,
}

pub(crate) fn resolve_cas_home() -> PathBuf {
    std::env::var("HKASK_CAS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".hkask").join("repos")
        })
}

// ── I/O helpers ──────────────────────────────────────────────────────────

fn check_git(output: &Output, cmd: &str) -> Result<(), GitCasError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitCasError::Git(format!(
            "git {cmd} failed: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

fn parse_commit_hash(output: &Output) -> Result<CommitHash, GitCasError> {
    let sha_hex = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes =
        hex::decode(&sha_hex).map_err(|e| GitCasError::Git(format!("Invalid SHA hex: {e}")))?;
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes[..20]);
    Ok(CommitHash::from_bytes(arr))
}

fn git_cmd(dir: &Path, args: &[&str]) -> Result<Output, GitCasError> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| GitCasError::Io(format!("git {} failed: {e}", args[0])))
}

async fn spawn_git_op<F, T>(f: F) -> Result<T, GitCasError>
where
    F: FnOnce() -> Result<T, GitCasError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
}

fn parse_diff_line(line: &str) -> Option<FileDiff> {
    let (kind, path) = match line.chars().next()? {
        'A' => (DiffKind::Added, line[1..].trim()),
        'D' => (DiffKind::Removed, line[1..].trim()),
        'M' => (DiffKind::Modified, line[1..].trim()),
        _ => return None,
    };
    Some(FileDiff {
        path: path.to_string(),
        kind,
        content: String::new(),
    })
}

impl GixCasAdapter {
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, GitCasError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)
            .map_err(|e| GitCasError::Io(format!("Failed to create base path: {e}")))?;
        Ok(Self {
            base_path,
            initialized: RwLock::new(std::collections::HashSet::new()),
        })
    }

    pub fn from_env() -> Result<Self, GitCasError> {
        Self::new(resolve_cas_home())
    }

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
        spawn_git_op(move || {
            std::fs::create_dir_all(&cas_path)
                .map_err(|e| GitCasError::Io(format!("Failed to create CAS dir: {e}")))?;
            Ok(repo_path)
        })
        .await?;
        let mut init = self.initialized.write().await;
        init.insert(dir_name);
        Ok(self.base_path.join(repo.dir_name()))
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
        spawn_git_op(move || {
            std::fs::create_dir_all(&cas_dir)
                .map_err(|e| GitCasError::Io(format!("Failed to create CAS dir: {e}")))?;
            std::fs::write(&blob_path, &content)
                .map_err(|e| GitCasError::Io(format!("Failed to write blob: {e}")))
        })
        .await?;
        Ok(hash)
    }

    async fn get_blob(&self, repo: &RepoId, hash: &ContentHash) -> Result<Vec<u8>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let blob_path = repo_dir.join("cas").join(hash.to_string());
        spawn_git_op(move || {
            std::fs::read(&blob_path)
                .map_err(|e| GitCasError::NotFound(format!("Blob not found: {e}")))
        })
        .await
    }

    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let msg = message.to_string();
        spawn_git_op(move || {
            if !repo_dir.join(".git").exists() {
                check_git(&git_cmd(&repo_dir, &["init"])?, "init")?;
            }
            let output = git_cmd(&repo_dir, &["add", "-A"])?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() && !stderr.contains("nothing") {
                return Err(GitCasError::Git(format!(
                    "git add failed: {}",
                    stderr.trim()
                )));
            }
            let output = git_cmd(&repo_dir, &["commit", "-m", &msg])?;
            if !output.status.success()
                && !String::from_utf8_lossy(&output.stderr).contains("nothing to commit")
            {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(GitCasError::Git(format!(
                    "git commit failed: {}",
                    stderr.trim()
                )));
            }
            parse_commit_hash(&git_cmd(&repo_dir, &["rev-parse", "HEAD"])?)
        })
        .await
    }

    async fn resolve_ref(&self, repo: &RepoId, reference: &str) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let ref_name = reference.to_string();
        spawn_git_op(move || {
            let output = git_cmd(&repo_dir, &["rev-parse", &ref_name])?;
            check_git(&output, &format!("rev-parse '{ref_name}'"))?;
            parse_commit_hash(&output)
        })
        .await
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
        spawn_git_op(move || {
            let mut args = vec!["ls-tree", &ref_name];
            if !prefix_filter.is_empty() {
                args.extend(["--", &prefix_filter]);
            }
            let output = git_cmd(&repo_dir, &args)?;
            check_git(&output, "ls-tree")?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut entries = Vec::new();
            for line in stdout.lines() {
                let (meta, path) = match line.split_once('\t') {
                    Some(p) => p,
                    None => continue,
                };
                let meta_parts: Vec<&str> = meta.split_whitespace().collect();
                if meta_parts.len() < 3 {
                    continue;
                }
                let kind = if meta_parts[1] == "tree" {
                    TreeEntryKind::Tree
                } else {
                    TreeEntryKind::Blob
                };
                let content_hash = if kind == TreeEntryKind::Blob {
                    let blob_output = git_cmd(&repo_dir, &["cat-file", "-p", meta_parts[2]])?;
                    ContentHash::from_blake3(&blob_output.stdout)
                } else {
                    ContentHash::from_blake3(meta_parts[2].as_bytes())
                };
                entries.push(TreeEntry {
                    path: path.to_string(),
                    content_hash,
                    kind,
                });
            }
            Ok(entries)
        })
        .await
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
        spawn_git_op(move || {
            let output = git_cmd(&repo_dir, &["diff", "--name-status", &from_ref, &to_ref])?;
            check_git(&output, "diff")?;
            Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(parse_diff_line)
                .collect())
        })
        .await
    }

    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let cas_dir = repo_dir.join("cas");
        let repo_id = repo.clone();
        spawn_git_op(move || {
            if !cas_dir.exists() {
                return Ok(VerificationReport {
                    repo: repo_id,
                    total_blobs: 0,
                    verified_blobs: 0,
                    corrupt_hashes: vec![],
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
                let expected_hash: ContentHash = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
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
    }

    async fn log(&self, repo: &RepoId, max_count: usize) -> Result<Vec<LogEntry>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let max = max_count;
        spawn_git_op(move || {
            let output = git_cmd(
                &repo_dir,
                &[
                    "log",
                    "--oneline",
                    "--pretty=format:%H %ct %s",
                    &format!("-{max}"),
                ],
            )?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
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
            Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if line.is_empty() {
                        return None;
                    }
                    let (hash_str, rest) = line.split_once(' ')?;
                    let (ts_str, message) = rest.split_once(' ')?;
                    let commit: CommitHash = hash_str
                        .parse()
                        .map_err(|e: String| {
                            GitCasError::Git(format!("Invalid commit hash in log: {e}"))
                        })
                        .ok()?;
                    let timestamp_secs = ts_str.parse::<u64>().unwrap_or(0);
                    Some(LogEntry {
                        commit,
                        message: message.to_string(),
                        timestamp_secs,
                    })
                })
                .collect())
        })
        .await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
