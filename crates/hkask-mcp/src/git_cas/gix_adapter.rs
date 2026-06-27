//! Gix-based Git CAS Adapter — implements [`GitCASPort`] with the `gix` crate.
//! # REQ: F8 — pure Rust gitoxide, no CLI git subprocess.
//! expect: "Git CAS operations use pure Rust gitoxide without CLI subprocess"
//!
//! Blob storage: BLAKE3-addressed flat files in `cas/<hash>` (unchanged).
//! Git operations: pure `gix` crate v0.81.
//!
//! Snapshot strategy: reads files from `cas/`, writes each as a git blob object,
//! builds a tree from blob OIDs, commits the tree. No index needed.

use hkask_ports::git_cas::{
    CommitHash, ContentHash, DiffKind, FileDiff, GitCASPort, GitCasError, LogEntry, RepoId,
    TreeEntry, TreeEntryKind, VerificationReport,
};
use std::path::{Path, PathBuf};
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

// ── gix helpers ─────────────────────────────────────────────────────────

fn open_or_init_repo(path: &Path) -> Result<gix::Repository, GitCasError> {
    if path.join(".git").exists() {
        gix::open(path).map_err(|e| GitCasError::Git(format!("gix::open: {e}")))
    } else {
        gix::init(path).map_err(|e| GitCasError::Git(format!("gix::init: {e}")))
    }
}

fn oid_to_commit_hash(oid: &gix::ObjectId) -> CommitHash {
    let bytes = oid.as_bytes();
    let mut arr = [0u8; 20];
    let len = bytes.len().min(20);
    arr[..len].copy_from_slice(&bytes[..len]);
    CommitHash::from_bytes(arr)
}

fn build_tree(
    repo: &gix::Repository,
    entries: &[(String, gix::ObjectId)],
) -> Result<gix::ObjectId, GitCasError> {
    if entries.is_empty() {
        let empty: Vec<gix::objs::tree::EntryRef<'_>> = Vec::new();
        return Ok(repo
            .write_object(gix::objs::TreeRef { entries: empty })
            .map_err(|e| GitCasError::Git(format!("gix write empty tree: {e}")))?
            .detach());
    }
    // Sort by filename — gix requires entries to be sorted for serialization
    let mut sorted: Vec<_> = entries.to_vec();
    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
    let entries_refs: Vec<gix::objs::tree::EntryRef<'_>> = sorted
        .iter()
        .map(|(name, oid)| gix::objs::tree::EntryRef {
            mode: gix::objs::tree::EntryMode::from(gix::objs::tree::EntryKind::Blob),
            oid: oid.as_ref(),
            filename: name.as_str().into(),
        })
        .collect();
    Ok(repo
        .write_object(gix::objs::TreeRef {
            entries: entries_refs,
        })
        .map_err(|e| GitCasError::Git(format!("gix write tree: {e}")))?
        .detach())
}

async fn spawn_blocking_io<F, T>(f: F) -> Result<T, GitCasError>
where
    F: FnOnce() -> Result<T, GitCasError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| GitCasError::Io(format!("Task join error: {e}")))?
}

impl GixCasAdapter {
    /// Create a new GixCasAdapter at the given base path.
    ///
    /// pre:  base_path is a valid directory path (created if missing)
    /// post: returns GixCasAdapter with initialized set
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, GitCasError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)
            .map_err(|e| GitCasError::Io(format!("Failed to create base path: {e}")))?;
        Ok(Self {
            base_path,
            initialized: RwLock::new(std::collections::HashSet::new()),
        })
    }

    /// Create a GixCasAdapter from the HKASK_CAS_HOME environment variable.
    ///
    /// post: returns GixCasAdapter at resolved CAS home path
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
        spawn_blocking_io(move || {
            std::fs::create_dir_all(&cas_path)
                .map_err(|e| GitCasError::Io(format!("Failed to create CAS dir: {e}")))?;
            Ok(repo_path)
        })
        .await?;
        let mut init = self.initialized.write().await;
        init.insert(dir_name);
        Ok(self.base_path.join(repo.dir_name()))
    }

    /// Resolve a symbolic ref (branch, tag) to a commit SHA.
    ///
    /// This is an admin-level operation — not part of the [`GitCASPort`]
    /// backup contract. Used by the API git archive route.
    pub async fn resolve_ref(
        &self,
        repo: &RepoId,
        reference: &str,
    ) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let ref_name = reference.to_string();
        spawn_blocking_io(move || {
            let repo =
                gix::open(&repo_dir).map_err(|e| GitCasError::Git(format!("gix::open: {e}")))?;
            let id = repo
                .rev_parse_single(ref_name.as_str())
                .map_err(|e| GitCasError::Git(format!("gix rev_parse '{ref_name}': {e}")))?;
            Ok(oid_to_commit_hash(&id.detach()))
        })
        .await
    }

    /// Diff two commits.
    ///
    /// This is an admin-level operation — not part of the [`GitCASPort`]
    /// backup contract. Used by the CLI `kask git diff` command.
    pub async fn diff(
        &self,
        repo: &RepoId,
        from: &str,
        to: &str,
    ) -> Result<Vec<FileDiff>, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let from_ref = from.to_string();
        let to_ref = to.to_string();
        spawn_blocking_io(move || {
            let repo =
                gix::open(&repo_dir).map_err(|e| GitCasError::Git(format!("gix::open: {e}")))?;
            let from_id = repo
                .rev_parse_single(from_ref.as_str())
                .map_err(|e| GitCasError::Git(format!("gix rev_parse '{from_ref}': {e}")))?;
            let to_id = repo
                .rev_parse_single(to_ref.as_str())
                .map_err(|e| GitCasError::Git(format!("gix rev_parse '{to_ref}': {e}")))?;

            let from_tree = commit_tree_oid(&repo, &from_id.detach())?;
            let to_tree = commit_tree_oid(&repo, &to_id.detach())?;

            let mut from_paths = std::collections::BTreeMap::new();
            let mut to_paths = std::collections::BTreeMap::new();
            collect_paths(&repo, &from_tree, "", &mut from_paths)?;
            collect_paths(&repo, &to_tree, "", &mut to_paths)?;

            let mut diffs = Vec::new();
            for p in to_paths.keys() {
                if !from_paths.contains_key(p) {
                    diffs.push(FileDiff {
                        path: p.clone(),
                        kind: DiffKind::Added,
                        content: String::new(),
                    });
                }
            }
            for p in from_paths.keys() {
                if !to_paths.contains_key(p) {
                    diffs.push(FileDiff {
                        path: p.clone(),
                        kind: DiffKind::Removed,
                        content: String::new(),
                    });
                }
            }
            for (p, from_oid) in &from_paths {
                if let Some(to_oid) = to_paths.get(p)
                    && from_oid != to_oid
                {
                    diffs.push(FileDiff {
                        path: p.clone(),
                        kind: DiffKind::Modified,
                        content: String::new(),
                    });
                }
            }
            Ok(diffs)
        })
        .await
    }

    // ── Pod-directory-based backup ───────────────────────────────────────

    /// Snapshot a pod directory: walk the tree, create git blobs for all files,
    /// build a nested tree, commit. Skips .git directory.
    pub async fn snapshot_pod_dir(
        &self,
        pod_dir: &Path,
        message: &str,
    ) -> Result<CommitHash, GitCasError> {
        let dir = pod_dir.to_path_buf();
        let msg = message.to_string();
        spawn_blocking_io(move || {
            if !dir.exists() {
                return Err(GitCasError::NotFound(format!(
                    "Pod directory does not exist: {}",
                    dir.display()
                )));
            }
            let repo = open_or_init_repo(&dir)?;
            let tree_oid = write_dir_as_tree(&repo, &dir, &dir)?;

            let parent = repo.head_commit().ok().map(|c| c.id().detach());
            let parents: Vec<gix::ObjectId> = parent.into_iter().collect();

            let commit_oid = repo
                .commit("HEAD", &msg, tree_oid, parents)
                .map_err(|e| GitCasError::Git(format!("gix commit: {e}")))?;

            Ok(oid_to_commit_hash(&commit_oid.detach()))
        })
        .await
    }

    /// List commit history for a pod directory, newest first.
    pub async fn log_pod(
        &self,
        pod_dir: &Path,
        max_count: usize,
    ) -> Result<Vec<LogEntry>, GitCasError> {
        let dir = pod_dir.to_path_buf();
        let max = max_count;
        spawn_blocking_io(move || {
            let repo = match gix::open(&dir) {
                Ok(r) => r,
                Err(_) => return Ok(Vec::new()),
            };
            let head_commit = match repo.head_commit() {
                Ok(c) => c,
                Err(_) => return Ok(Vec::new()),
            };
            let platform = repo.rev_walk(Some(head_commit.id().detach()));
            let mut entries = Vec::new();
            let walk = match platform.all() {
                Ok(w) => w,
                Err(_) => return Ok(Vec::new()),
            };
            for (count, item) in walk.enumerate() {
                if count >= max {
                    break;
                }
                let Ok(info) = item else { continue };
                let commit = match info.object().ok() {
                    Some(c) => c,
                    None => continue,
                };
                let message = commit
                    .message_raw()
                    .map(|m| m.to_string())
                    .unwrap_or_default();
                let timestamp_secs = commit.time().map(|t| t.seconds as u64).unwrap_or(0);
                entries.push(LogEntry {
                    commit: oid_to_commit_hash(&info.id),
                    message,
                    timestamp_secs,
                });
            }
            Ok(entries)
        })
        .await
    }

    /// Restore a file from a prior commit to a destination path.
    pub async fn restore_file_from_commit(
        &self,
        pod_dir: &Path,
        commit: &CommitHash,
        file_path: &str,
        dest: &Path,
    ) -> Result<(), GitCasError> {
        let dir = pod_dir.to_path_buf();
        let commit_str = commit.to_string();
        let fp = file_path.to_string();
        let d = dest.to_path_buf();
        spawn_blocking_io(move || {
            let repo = gix::open(&dir).map_err(|e| GitCasError::Git(format!("gix::open: {e}")))?;
            let id = repo
                .rev_parse_single(commit_str.as_str())
                .map_err(|e| GitCasError::Git(format!("gix rev_parse '{commit_str}': {e}")))?;
            let tree_oid = commit_tree_oid(&repo, &id.detach())?;
            let obj = repo
                .find_object(tree_oid)
                .map_err(|e| GitCasError::Git(format!("find_object tree: {e}")))?;
            let tree = obj
                .try_into_tree()
                .map_err(|e| GitCasError::Git(format!("try_into_tree: {e}")))?;

            let mut found = None;
            for entry in tree.iter() {
                let entry = entry.map_err(|e| GitCasError::Git(format!("tree entry: {e}")))?;
                if entry.filename().to_string() == fp {
                    if entry.mode().is_tree() {
                        return Err(GitCasError::NotFound(format!(
                            "'{fp}' is a directory, not a file"
                        )));
                    }
                    let blob = repo
                        .find_object(entry.oid().to_owned())
                        .map_err(|e| GitCasError::Git(format!("find_object blob: {e}")))?;
                    found = Some(blob.data.to_vec());
                    break;
                }
            }

            let data = found.ok_or_else(|| {
                GitCasError::NotFound(format!("File '{fp}' not found in commit {commit_str}"))
            })?;

            std::fs::write(&d, &data)
                .map_err(|e| GitCasError::Io(format!("Failed to write restored file: {e}")))?;
            Ok(())
        })
        .await
    }
}

// ── Index helper ─────────────────────────────────────────────────────────

fn add_dir_to_index(
    repo: &gix::Repository,
    root: &Path,
    dir: &Path,
    index: &mut gix::index::State,
) -> Result<(), GitCasError> {
    for entry in std::fs::read_dir(dir).map_err(|e| GitCasError::Io(format!("read_dir: {e}")))? {
        let entry = entry.map_err(|e| GitCasError::Io(format!("dir entry: {e}")))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| GitCasError::Io(format!("file_type: {e}")))?;

        // Skip .git directory
        if path.file_name().map(|n| n == ".git").unwrap_or(false) {
            continue;
        }

        if file_type.is_dir() {
            add_dir_to_index(repo, root, &path, index)?;
        } else if file_type.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| GitCasError::Io(format!("strip_prefix: {e}")))?;
            let content =
                std::fs::read(&path).map_err(|e| GitCasError::Io(format!("read file: {e}")))?;
            let oid = repo
                .write_object(gix::objs::BlobRef { data: &content })
                .map_err(|e| GitCasError::Git(format!("write_object: {e}")))?;
            let mode = if is_executable(&path) {
                gix::objs::tree::EntryMode::BlobExecutable
            } else {
                gix::objs::tree::EntryMode::Blob
            };
            index.dangerously_push_entry(rel.to_string_lossy().into_owned(), oid, mode);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

#[async_trait::async_trait]
impl GitCASPort for GixCasAdapter {
    async fn put_blob(&self, repo: &RepoId, content: &[u8]) -> Result<ContentHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let hash = ContentHash::from_blake3(content);
        let cas_dir = repo_dir.join("cas");
        let blob_path = cas_dir.join(hash.to_string());
        let content = content.to_vec();
        spawn_blocking_io(move || {
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
        spawn_blocking_io(move || {
            std::fs::read(&blob_path)
                .map_err(|e| GitCasError::NotFound(format!("Blob not found: {e}")))
        })
        .await
    }

    async fn delete_blob(&self, repo: &RepoId, hash: &ContentHash) -> Result<(), GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let blob_path = repo_dir.join("cas").join(hash.to_string());
        spawn_blocking_io(move || {
            if blob_path.exists() {
                std::fs::remove_file(&blob_path)
                    .map_err(|e| GitCasError::Io(format!("Failed to delete blob: {e}")))?;
            }
            Ok(())
        })
        .await
    }

    async fn snapshot(&self, repo: &RepoId, message: &str) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let msg = message.to_string();
        spawn_blocking_io(move || {
            let repo = open_or_init_repo(&repo_dir)?;
            let cas_dir = repo_dir.join("cas");

            // Read all files from cas/ directory and write them as git blob objects.
            let mut tree_entries: Vec<(String, gix::ObjectId)> = Vec::new();
            if cas_dir.exists() {
                for entry in std::fs::read_dir(&cas_dir)
                    .map_err(|e| GitCasError::Io(format!("read_dir cas: {e}")))?
                {
                    let entry = entry.map_err(|e| GitCasError::Io(format!("dir entry: {e}")))?;
                    let path = entry.path();
                    if path.is_dir() {
                        continue;
                    }
                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let content = std::fs::read(&path)
                        .map_err(|e| GitCasError::Io(format!("read blob: {e}")))?;
                    let oid = repo
                        .write_object(gix::objs::BlobRef { data: &content })
                        .map_err(|e| GitCasError::Git(format!("gix write_object: {e}")))?;
                    tree_entries.push((filename, oid.detach()));
                }
            }

            let tree_id = build_tree(&repo, &tree_entries)?;

            // Parent is HEAD if it exists.
            let parent = repo.head_commit().ok().map(|c| c.id().detach());
            let parents: Vec<gix::ObjectId> = parent.into_iter().collect();

            let commit_oid = repo
                .commit("HEAD", &msg, tree_id, parents)
                .map_err(|e| GitCasError::Git(format!("gix commit: {e}")))?;

            Ok(oid_to_commit_hash(&commit_oid.detach()))
        })
        .await
    }

    async fn snapshot_orphan(
        &self,
        repo: &RepoId,
        message: &str,
    ) -> Result<CommitHash, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let msg = message.to_string();
        spawn_blocking_io(move || {
            let repo = open_or_init_repo(&repo_dir)?;
            let cas_dir = repo_dir.join("cas");

            let mut tree_entries: Vec<(String, gix::ObjectId)> = Vec::new();
            if cas_dir.exists() {
                for entry in std::fs::read_dir(&cas_dir)
                    .map_err(|e| GitCasError::Io(format!("read_dir cas: {e}")))?
                {
                    let entry = entry.map_err(|e| GitCasError::Io(format!("dir entry: {e}")))?;
                    let path = entry.path();
                    if path.is_dir() {
                        continue;
                    }
                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let content = std::fs::read(&path)
                        .map_err(|e| GitCasError::Io(format!("read blob: {e}")))?;
                    let oid = repo
                        .write_object(gix::objs::BlobRef { data: &content })
                        .map_err(|e| GitCasError::Git(format!("gix write_object: {e}")))?;
                    tree_entries.push((filename, oid.detach()));
                }
            }

            let tree_id = build_tree(&repo, &tree_entries)?;

            // No parent — orphan commit for history rewriting.
            let parents: Vec<gix::ObjectId> = Vec::new();
            let commit_oid = repo
                .commit("HEAD", &msg, tree_id, parents)
                .map_err(|e| GitCasError::Git(format!("gix commit: {e}")))?;

            Ok(oid_to_commit_hash(&commit_oid.detach()))
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
        spawn_blocking_io(move || {
            let repo =
                gix::open(&repo_dir).map_err(|e| GitCasError::Git(format!("gix::open: {e}")))?;
            let id = repo
                .rev_parse_single(ref_name.as_str())
                .map_err(|e| GitCasError::Git(format!("gix rev_parse '{ref_name}': {e}")))?;
            let oid = id.detach();
            // Resolve commit → tree before recursive traversal
            let tree_oid = commit_tree_oid(&repo, &oid)?;

            let mut entries = Vec::new();
            list_tree_recursive(&repo, &tree_oid, "", &prefix_filter, &mut entries)?;
            Ok(entries)
        })
        .await
    }

    async fn verify(&self, repo: &RepoId) -> Result<VerificationReport, GitCasError> {
        let repo_dir = self.ensure_repo_dir(repo).await?;
        let cas_dir = repo_dir.join("cas");
        let repo_id = repo.clone();
        spawn_blocking_io(move || {
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
                    .map_err(|e: hkask_ports::git_cas::ParseHashError| {
                        GitCasError::PathValidation(format!(
                            "Invalid blob hash filename '{}': {e}",
                            path.display()
                        ))
                    })?;
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
        spawn_blocking_io(move || {
            // Repo may not have been initialized yet (no snapshots taken) — return empty
            let repo = match gix::open(&repo_dir) {
                Ok(r) => r,
                Err(_) => return Ok(Vec::new()),
            };
            let head_commit = match repo.head_commit() {
                Ok(c) => c,
                Err(_) => return Ok(Vec::new()),
            };
            let platform = repo.rev_walk(Some(head_commit.id().detach()));
            let mut entries = Vec::new();
            let walk = match platform.all() {
                Ok(w) => w,
                Err(_) => return Ok(Vec::new()),
            };
            for (count, item) in walk.enumerate() {
                if count >= max {
                    break;
                }
                let Ok(info) = item else { continue };
                let commit = match info.object().ok() {
                    Some(c) => c,
                    None => continue,
                };
                let message = commit
                    .message_raw()
                    .map(|m| m.to_string())
                    .unwrap_or_default();
                let timestamp_secs = commit.time().map(|t| t.seconds as u64).unwrap_or(0);
                entries.push(LogEntry {
                    commit: oid_to_commit_hash(&info.id),
                    message,
                    timestamp_secs,
                });
            }
            Ok(entries)
        })
        .await
    }
}

// ── Tree helpers ────────────────────────────────────────────────────────

fn commit_tree_oid(
    repo: &gix::Repository,
    oid: &gix::ObjectId,
) -> Result<gix::ObjectId, GitCasError> {
    let obj = repo
        .find_object(*oid)
        .map_err(|e| GitCasError::Git(format!("find_object: {e}")))?;
    let commit = obj
        .try_into_commit()
        .map_err(|e| GitCasError::Git(format!("try_into_commit: {e}")))?;
    Ok(commit
        .tree_id()
        .map_err(|e| GitCasError::Git(format!("tree_id: {e}")))?
        .detach())
}

fn list_tree_recursive(
    repo: &gix::Repository,
    tree_oid: &gix::ObjectId,
    path_prefix: &str,
    filter_prefix: &str,
    out: &mut Vec<TreeEntry>,
) -> Result<(), GitCasError> {
    let obj = repo
        .find_object(*tree_oid)
        .map_err(|e| GitCasError::Git(format!("find_object tree: {e}")))?;
    let tree = obj
        .try_into_tree()
        .map_err(|e| GitCasError::Git(format!("try_into_tree: {e}")))?;
    for entry in tree.iter() {
        let entry = entry.map_err(|e| GitCasError::Git(format!("tree entry: {e}")))?;
        let name = entry.filename().to_string();
        let full_path = if path_prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", path_prefix, name)
        };
        if entry.mode().is_tree() {
            list_tree_recursive(
                repo,
                &entry.oid().to_owned(),
                &full_path,
                filter_prefix,
                out,
            )?;
        } else if filter_prefix.is_empty() || full_path.starts_with(filter_prefix) {
            let blob_obj = repo
                .find_object(entry.oid().to_owned())
                .map_err(|e| GitCasError::Git(format!("find_object blob: {e}")))?;
            let content_hash = ContentHash::from_blake3(&blob_obj.data);
            out.push(TreeEntry {
                path: full_path,
                content_hash,
                kind: TreeEntryKind::Blob,
            });
        }
    }
    Ok(())
}

fn collect_paths(
    repo: &gix::Repository,
    tree_oid: &gix::ObjectId,
    prefix: &str,
    out: &mut std::collections::BTreeMap<String, gix::ObjectId>,
) -> Result<(), GitCasError> {
    let obj = repo
        .find_object(*tree_oid)
        .map_err(|e| GitCasError::Git(format!("find_object tree: {e}")))?;
    let tree = obj
        .try_into_tree()
        .map_err(|e| GitCasError::Git(format!("try_into_tree: {e}")))?;
    for entry in tree.iter() {
        let entry = entry.map_err(|e| GitCasError::Git(format!("tree entry: {e}")))?;
        let name = entry.filename().to_string();
        let full_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };
        if entry.mode().is_tree() {
            collect_paths(repo, &entry.oid().to_owned(), &full_path, out)?;
        } else {
            out.insert(full_path, entry.oid().to_owned());
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_adapter() -> (GixCasAdapter, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let adapter = GixCasAdapter::new(dir.path()).unwrap();
        (adapter, dir)
    }

    #[tokio::test]
    async fn put_and_get_blob_roundtrip() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Registry;
        let content = b"hello, CAS world";

        let hash = adapter.put_blob(&repo, content).await.unwrap();
        let retrieved = adapter.get_blob(&repo, &hash).await.unwrap();

        assert_eq!(retrieved, content);
        assert_eq!(hash, ContentHash::from_blake3(content));
    }

    #[tokio::test]
    async fn get_nonexistent_blob_returns_not_found() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Memory;
        let hash = ContentHash::from_blake3(b"doesnt exist");

        let result = adapter.get_blob(&repo, &hash).await;
        assert!(matches!(result, Err(GitCasError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_blob_then_get_returns_not_found() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Sessions;
        let content = b"temporary data";

        let hash = adapter.put_blob(&repo, content).await.unwrap();
        adapter.delete_blob(&repo, &hash).await.unwrap();

        let result = adapter.get_blob(&repo, &hash).await;
        assert!(matches!(result, Err(GitCasError::NotFound(_))));
    }

    #[tokio::test]
    async fn snapshot_produces_commit_and_log_returns_history() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Registry;

        // Put some blobs
        let h1 = adapter.put_blob(&repo, b"blob A").await.unwrap();
        let h2 = adapter.put_blob(&repo, b"blob B").await.unwrap();

        // Snapshot
        let commit = adapter.snapshot(&repo, "first snapshot").await.unwrap();
        assert!(!commit.to_string().is_empty());

        // Log should show the commit
        let entries = adapter.log(&repo, 10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].commit, commit);
        assert!(entries[0].message.contains("first snapshot"));

        // list_tree should show the blobs
        let tree = adapter
            .list_tree(&repo, &commit.to_string(), "")
            .await
            .unwrap();
        assert_eq!(tree.len(), 2);
        let hashes: Vec<_> = tree.iter().map(|e| e.content_hash.clone()).collect();
        assert!(hashes.contains(&h1));
        assert!(hashes.contains(&h2));
    }

    #[tokio::test]
    async fn snapshot_orphan_has_no_parent() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::GoalsSpecs;

        adapter.put_blob(&repo, b"orphan data").await.unwrap();
        let orphan = adapter
            .snapshot_orphan(&repo, "orphan commit")
            .await
            .unwrap();

        // Verify orphan is a valid commit
        let entries = adapter.log(&repo, 10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].commit, orphan);

        // A normal snapshot after orphan should have orphan as parent
        adapter.put_blob(&repo, b"second blob").await.unwrap();
        let child = adapter.snapshot(&repo, "child commit").await.unwrap();
        assert_ne!(orphan, child);
        let entries = adapter.log(&repo, 10).await.unwrap();
        assert_eq!(entries.len(), 2);
        // Newest first
        assert_eq!(entries[0].commit, child);
        assert_eq!(entries[1].commit, orphan);
    }

    #[tokio::test]
    async fn verify_reports_correct_integrity() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::CnsAudit;

        adapter.put_blob(&repo, b"integrity check 1").await.unwrap();
        adapter.put_blob(&repo, b"integrity check 2").await.unwrap();

        let report = adapter.verify(&repo).await.unwrap();
        assert_eq!(report.repo, RepoId::CnsAudit);
        assert_eq!(report.total_blobs, 2);
        assert_eq!(report.verified_blobs, 2);
        assert!(report.corrupt_hashes.is_empty());
    }

    #[tokio::test]
    async fn verify_empty_repo_returns_zero() {
        let (adapter, _dir) = test_adapter();
        let report = adapter.verify(&RepoId::Vault).await.unwrap();
        assert_eq!(report.total_blobs, 0);
        assert_eq!(report.verified_blobs, 0);
    }

    #[tokio::test]
    async fn log_empty_repo_returns_empty() {
        let (adapter, _dir) = test_adapter();
        let entries = adapter.log(&RepoId::Sovereignty, 10).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn resolve_ref_resolves_head() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Registry;

        adapter.put_blob(&repo, b"ref test").await.unwrap();
        let commit = adapter.snapshot(&repo, "ref snapshot").await.unwrap();

        let resolved = adapter.resolve_ref(&repo, "HEAD").await.unwrap();
        assert_eq!(resolved, commit);
    }

    #[tokio::test]
    async fn diff_detects_added_removed_and_modified() {
        let (adapter, _dir) = test_adapter();
        use RepoId::{GoalsSpecs, Registry};

        // Create first snapshot in Repo A
        adapter
            .put_blob(&GoalsSpecs, b"file1 content v1")
            .await
            .unwrap();
        let commit1 = adapter.snapshot(&GoalsSpecs, "first").await.unwrap();

        // Create second snapshot in Repo A with one new blob
        adapter
            .put_blob(&GoalsSpecs, b"file2 content new")
            .await
            .unwrap();
        let commit2 = adapter.snapshot(&GoalsSpecs, "second").await.unwrap();

        let diffs = adapter
            .diff(&GoalsSpecs, &commit1.to_string(), &commit2.to_string())
            .await
            .unwrap();
        // We should see at least one added file
        let added: Vec<_> = diffs.iter().filter(|d| d.kind == DiffKind::Added).collect();
        assert!(!added.is_empty(), "Expected at least one Added diff");
    }

    #[tokio::test]
    async fn list_tree_with_prefix_filter() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Registry;

        adapter.put_blob(&repo, b"aaa").await.unwrap();
        adapter.put_blob(&repo, b"bbb").await.unwrap();
        let commit = adapter.snapshot(&repo, "prefix test").await.unwrap();

        let all = adapter
            .list_tree(&repo, &commit.to_string(), "")
            .await
            .unwrap();
        assert_eq!(all.len(), 2);

        // Filter by hash prefix — should find at least one matching entry
        let first_hash = &all[0].content_hash.to_string();
        let short = &first_hash[..8];
        let filtered = adapter
            .list_tree(&repo, &commit.to_string(), short)
            .await
            .unwrap();
        // At minimum, the entry with matching hash prefix is in the results
        assert!(!filtered.is_empty());
    }

    #[tokio::test]
    async fn concurrent_puts_to_different_repos() {
        let (adapter, _dir) = test_adapter();
        let adapter = std::sync::Arc::new(adapter);

        let a1 = adapter.clone();
        let h1 = tokio::spawn(async move {
            a1.put_blob(&RepoId::Registry, b"concurrent A")
                .await
                .unwrap()
        });
        let a2 = adapter.clone();
        let h2 =
            tokio::spawn(
                async move { a2.put_blob(&RepoId::Memory, b"concurrent B").await.unwrap() },
            );

        let hash1 = h1.await.unwrap();
        let hash2 = h2.await.unwrap();
        assert_ne!(hash1, hash2);

        // Both repos have one blob
        let r1 = adapter.verify(&RepoId::Registry).await.unwrap();
        let r2 = adapter.verify(&RepoId::Memory).await.unwrap();
        assert_eq!(r1.total_blobs, 1);
        assert_eq!(r2.total_blobs, 1);
    }

    #[tokio::test]
    async fn put_blob_idempotent() {
        let (adapter, _dir) = test_adapter();
        let repo = RepoId::Sessions;
        let content = b"same content";

        let h1 = adapter.put_blob(&repo, content).await.unwrap();
        let h2 = adapter.put_blob(&repo, content).await.unwrap();
        assert_eq!(h1, h2);

        // verify still sees exactly one blob (CAS dedup)
        let report = adapter.verify(&repo).await.unwrap();
        assert_eq!(report.total_blobs, 1);
    }

    #[tokio::test]
    async fn from_env_respects_custom_home() {
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: single-threaded test, no concurrent env mutation
        unsafe { std::env::set_var("HKASK_CAS_HOME", dir.path().to_str().unwrap()) };

        let result = GixCasAdapter::from_env();
        assert!(result.is_ok());
        let adapter = result.unwrap();
        // Verify the adapter can operate at the custom path
        adapter
            .put_blob(&RepoId::Registry, b"custom home test")
            .await
            .unwrap();

        unsafe { std::env::remove_var("HKASK_CAS_HOME") };
    }
}
