//! Gix-based Git CAS Adapter — implements [`GitCASPort`] with the `gix` crate.
//! # REQ: F8 — pure Rust gitoxide, no CLI git subprocess.
//!
//! Blob storage: BLAKE3-addressed flat files in `cas/<hash>` (unchanged).
//! Git operations: pure `gix` crate v0.81.
//!
//! Snapshot strategy: reads files from `cas/`, writes each as a git blob object,
//! builds a tree from blob OIDs, commits the tree. No index needed.

use hkask_types::ports::git_cas::{
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
    let entries_refs: Vec<gix::objs::tree::EntryRef<'_>> = entries
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
    /// REQ: MCP-023
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
    /// REQ: MCP-024
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

    async fn resolve_ref(&self, repo: &RepoId, reference: &str) -> Result<CommitHash, GitCasError> {
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

            let mut entries = Vec::new();
            list_tree_recursive(&repo, &oid, "", &prefix_filter, &mut entries)?;
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
        spawn_blocking_io(move || {
            let repo =
                gix::open(&repo_dir).map_err(|e| GitCasError::Git(format!("gix::open: {e}")))?;
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
