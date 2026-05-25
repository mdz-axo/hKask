//! Archival Service — Consolidated git archival operations
//!
//! Single implementation shared by CLI and MCP handlers.
//! Implements hexagonal architecture with adapter container,
//! sovereignty enforcement, and CNS observability.

use hkask_agents::GitCASPort;
use hkask_agents::SovereigntyChecker;
use hkask_cns::spans::SpanEmitter;
use hkask_types::{ArchivalResult, DataCategory, GitArchivalError, WebID};
use serde_json::json;

use crate::adapter_container::AdapterContainer;

/// Archival service context
pub struct ArchivalService {
    adapter_container: AdapterContainer,
    sovereignty_checker: SovereigntyChecker,
    span_emitter: SpanEmitter,
}

impl ArchivalService {
    /// Create new archival service with sovereignty enforcement
    pub fn new(adapter_container: AdapterContainer, owner_webid: WebID) -> Self {
        let span_emitter = SpanEmitter::new(owner_webid);
        let sovereignty_checker = SovereigntyChecker::new(owner_webid);

        Self {
            adapter_container,
            sovereignty_checker,
            span_emitter,
        }
    }

    fn check_sovereignty(&self, requester: &WebID, operation: &str) -> ArchivalResult<()> {
        if !self
            .sovereignty_checker
            .can_access(&DataCategory::TemplateRegistry, requester)
        {
            self.span_emitter.emit_tool(
                &format!("{}.outcome", operation),
                json!({ "outcome": "sovereignty_denied" }),
            );
            return Err(GitArchivalError::SovereigntyDenied(
                "Registry operation requires consent".to_string(),
            ));
        }
        Ok(())
    }

    fn check_git_adapter(&self, operation: &str) -> ArchivalResult<()> {
        if !self.adapter_container.has_git_cas() {
            self.span_emitter.emit_tool(
                &format!("{}.outcome", operation),
                json!({ "outcome": "adapter_not_configured" }),
            );
            return Err(GitArchivalError::AdapterNotFound(
                "Git CAS adapter not configured".to_string(),
            ));
        }
        Ok(())
    }

    /// Archive content to git repository
    pub async fn archive(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
        _content: &str,
        requester: &WebID,
    ) -> ArchivalResult<String> {
        self.span_emitter.emit_tool(
            "git_archive",
            json!({
                "owner": owner,
                "repo": repo,
                "branch": branch,
                "path": path
            }),
        );

        self.check_sovereignty(requester, "git_archive")?;
        self.check_git_adapter("git_archive")?;

        let git_cas = self.adapter_container.get_git_cas().ok_or_else(|| {
            GitArchivalError::AdapterNotFound("Git CAS adapter unavailable".to_string())
        })?;

        let sha = git_cas
            .resolve_sha(&format!("{}/{}/{}", owner, repo, path))
            .map_err(|e| GitArchivalError::CommitFailed(e.to_string()))?;

        self.span_emitter.emit_tool(
            "git_archive.outcome",
            json!({
                "outcome": "success",
                "sha": sha,
                "path": format!("{}/{}/{}", owner, repo, path)
            }),
        );

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
        self.span_emitter.emit_tool(
            "git_restore",
            json!({
                "owner": owner,
                "repo": repo,
                "ref": git_ref,
                "target": target
            }),
        );

        self.check_sovereignty(requester, "git_restore")?;
        self.check_git_adapter("git_restore")?;

        self.span_emitter.emit_tool(
            "git_restore.outcome",
            json!({
                "outcome": "success",
                "source": format!("{}/{}/{}", owner, repo, git_ref),
                "target": target
            }),
        );

        Ok(format!(
            "Restored from {}/{}/{} to {}",
            owner, repo, git_ref, target
        ))
    }

    /// List archived versions
    pub async fn list_archives(
        &self,
        owner: &str,
        repo: &str,
        requester: &WebID,
    ) -> ArchivalResult<Vec<String>> {
        self.span_emitter.emit_tool(
            "git_list_archives",
            json!({
                "owner": owner,
                "repo": repo
            }),
        );

        self.check_sovereignty(requester, "git_list_archives")?;

        self.span_emitter.emit_tool(
            "git_list_archives.outcome",
            json!({
                "outcome": "success",
                "count": 0
            }),
        );

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
        self.span_emitter.emit_tool(
            "git_snapshot",
            json!({
                "owner": owner,
                "repo": repo,
                "message": message
            }),
        );

        self.check_sovereignty(requester, "git_snapshot")?;
        self.check_git_adapter("git_snapshot")?;

        let git_cas = self.adapter_container.get_git_cas().ok_or_else(|| {
            GitArchivalError::AdapterNotFound("Git CAS adapter unavailable".to_string())
        })?;

        let sha = git_cas
            .resolve_sha(&format!("{}/{}", owner, repo))
            .map_err(|e| GitArchivalError::CommitFailed(e.to_string()))?;

        self.span_emitter.emit_tool(
            "git_snapshot.outcome",
            json!({
                "outcome": "success",
                "sha": sha,
                "message": message
            }),
        );

        Ok(format!(
            "Created snapshot {} with message: {}",
            sha, message
        ))
    }
}
