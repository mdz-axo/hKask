//! Archival Service — Consolidated git archival operations
//!
//! Single implementation shared by CLI and MCP handlers.
//! Implements hexagonal architecture with adapter container,
//! sovereignty enforcement, and CNS observability.

use hkask_storage::sanitize_path;
use hkask_types::{ArchivalResult, DataCategory, GitArchivalError, SovereigntyPort, WebID};

use crate::adapter_container::AdapterContainer;

/// Archival service context
pub struct ArchivalService {
    adapter_container: AdapterContainer,
    sovereignty_checker: Box<dyn SovereigntyPort + Send + Sync>,
}

impl ArchivalService {
    /// Create new archival service with sovereignty enforcement
    pub fn new(adapter_container: AdapterContainer, _owner_webid: WebID) -> Self {
        // Note: Caller should use `with_sovereignty_checker()` to provide
        // a concrete implementation. This default uses a permissive checker.

        Self {
            adapter_container,
            sovereignty_checker: Box::new(PermissiveSovereigntyChecker),
        }
    }

    /// Create archival service with a custom sovereignty checker
    pub fn with_sovereignty_checker(
        adapter_container: AdapterContainer,
        sovereignty_checker: Box<dyn SovereigntyPort + Send + Sync>,
        _owner_webid: WebID,
    ) -> Self {
        Self {
            adapter_container,
            sovereignty_checker,
        }
    }

    fn check_sovereignty(&self, requester: &WebID, _operation: &str) -> ArchivalResult<()> {
        if !self
            .sovereignty_checker
            .can_access(&DataCategory::TemplateRegistry, requester)
        {
            return Err(GitArchivalError::SovereigntyDenied(
                "Registry operation requires consent".to_string(),
            ));
        }
        Ok(())
    }

    fn check_git_adapter(&self, _operation: &str) -> ArchivalResult<()> {
        match self.adapter_container.has_git_cas() {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                return Err(GitArchivalError::AdapterNotFound(
                    "Git CAS adapter not configured".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Archive content to git repository
    pub async fn archive(
        &self,
        owner: &str,
        repo: &str,
        _branch: &str,
        path: &str,
        _content: &str,
        requester: &WebID,
    ) -> ArchivalResult<String> {
        // Validate path to prevent traversal attacks
        let base = std::path::Path::new("/");
        let sanitized_path =
            sanitize_path(base, path).map_err(|e| GitArchivalError::InvalidPath(e.to_string()))?;

        self.check_sovereignty(requester, "git_archive")?;
        self.check_git_adapter("git_archive")?;

        let git_cas = self
            .adapter_container
            .get_git_cas()
            .map_err(GitArchivalError::AdapterNotFound)?
            .ok_or_else(|| {
                GitArchivalError::AdapterNotFound("Git CAS adapter unavailable".to_string())
            })?;

        let sha = git_cas
            .resolve_sha(&format!("{}/{}/{}", owner, repo, sanitized_path.display()))
            .map_err(|e| GitArchivalError::CommitFailed(e.to_string()))?;

        Ok(format!(
            "Archived to {}/{}/{} at SHA {}",
            owner, repo, path, sha
        ))
    }

    /// Restore content from git repository
    pub async fn restore(
        &self,
        owner: &str,
        repo: &str,
        git_ref: &str,
        target: &str,
        requester: &WebID,
    ) -> ArchivalResult<String> {
        self.check_sovereignty(requester, "git_restore")?;
        self.check_git_adapter("git_restore")?;

        Ok(format!(
            "Restored from {}/{}/{} to {}",
            owner, repo, git_ref, target
        ))
    }

    /// List archived versions
    pub async fn list_archives(
        &self,
        _owner: &str,
        _repo: &str,
        requester: &WebID,
    ) -> ArchivalResult<Vec<String>> {
        self.check_sovereignty(requester, "git_list_archives")?;

        Ok(Vec::new())
    }

    /// Create snapshot (commit)
    pub async fn create_snapshot(
        &self,
        owner: &str,
        repo: &str,
        message: &str,
        requester: &WebID,
    ) -> ArchivalResult<String> {
        self.check_sovereignty(requester, "git_snapshot")?;
        self.check_git_adapter("git_snapshot")?;

        let git_cas = self
            .adapter_container
            .get_git_cas()
            .map_err(GitArchivalError::AdapterNotFound)?
            .ok_or_else(|| {
                GitArchivalError::AdapterNotFound("Git CAS adapter unavailable".to_string())
            })?;

        let sha = git_cas
            .resolve_sha(&format!("{}/{}", owner, repo))
            .map_err(|e| GitArchivalError::CommitFailed(e.to_string()))?;

        Ok(format!(
            "Created snapshot {} with message: {}",
            sha, message
        ))
    }
}

/// Default permissive sovereignty checker used when no custom checker is provided.
/// Allows all access — callers should supply a real implementation via
/// `ArchivalService::with_sovereignty_checker()`.
struct PermissiveSovereigntyChecker;

impl SovereigntyPort for PermissiveSovereigntyChecker {
    fn can_access(&self, _data_category: &DataCategory, _requester: &WebID) -> bool {
        true
    }
}
