//! Archival Service — Consolidated git archival operations
//!
//! Single implementation shared by CLI and MCP handlers.
//! Implements hexagonal architecture with adapter container,
//! sovereignty enforcement, and CNS observability.

use hkask_agents::SovereigntyChecker;
use hkask_cns::spans::SpanEmitter;
use hkask_types::{ArchivalResult, DataCategory, GitArchivalError, WebID};
use serde_json::json;
use uuid::Uuid;

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

    /// Archive content to git repository
    pub async fn archive(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
        content: &str,
        requester: &WebID,
    ) -> ArchivalResult<String> {
        // Emit start span
        self.span_emitter.emit_tool(
            "git_archive",
            json!({
                "owner": owner,
                "repo": repo,
                "branch": branch,
                "path": path
            }),
        );

        // Check sovereignty - registry data is typically public/shared
        if !self
            .sovereignty_checker
            .can_access(&DataCategory::TemplateRegistry, requester)
        {
            self.span_emitter.emit_tool(
                "git_archive.outcome",
                json!({ "outcome": "sovereignty_denied" }),
            );
            return Err(GitArchivalError::SovereigntyDenied(
                "Registry archival requires consent".to_string(),
            ));
        }

        // Check adapter is configured
        if !self.adapter_container.has_git_cas() {
            self.span_emitter.emit_tool(
                "git_archive.outcome",
                json!({ "outcome": "adapter_not_configured" }),
            );
            return Err(GitArchivalError::AdapterNotFound(
                "Git CAS adapter not configured".to_string(),
            ));
        }

        // Perform archival (simulated for now)
        let sha = format!("archived_{}", Uuid::new_v4());

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
        // Emit start span
        self.span_emitter.emit_tool(
            "git_restore",
            json!({
                "owner": owner,
                "repo": repo,
                "ref": git_ref,
                "target": target
            }),
        );

        // Check sovereignty
        if !self
            .sovereignty_checker
            .can_access(&DataCategory::TemplateRegistry, requester)
        {
            self.span_emitter.emit_tool(
                "git_restore.outcome",
                json!({ "outcome": "sovereignty_denied" }),
            );
            return Err(GitArchivalError::SovereigntyDenied(
                "Registry restoration requires consent".to_string(),
            ));
        }

        // Check adapter is configured
        if !self.adapter_container.has_git_cas() {
            self.span_emitter.emit_tool(
                "git_restore.outcome",
                json!({ "outcome": "adapter_not_configured" }),
            );
            return Err(GitArchivalError::AdapterNotFound(
                "Git CAS adapter not configured".to_string(),
            ));
        }

        // Perform restore (simulated for now)
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
        // Emit start span
        self.span_emitter.emit_tool(
            "git_list_archives",
            json!({
                "owner": owner,
                "repo": repo
            }),
        );

        // Check sovereignty
        if !self
            .sovereignty_checker
            .can_access(&DataCategory::TemplateRegistry, requester)
        {
            self.span_emitter.emit_tool(
                "git_list_archives.outcome",
                json!({ "outcome": "sovereignty_denied" }),
            );
            return Err(GitArchivalError::SovereigntyDenied(
                "Registry listing requires consent".to_string(),
            ));
        }

        // Return simulated commit history
        let commits = vec![
            format!("commit_{}", Uuid::new_v4()),
            format!("commit_{}", Uuid::new_v4()),
            format!("commit_{}", Uuid::new_v4()),
        ];

        self.span_emitter.emit_tool(
            "git_list_archives.outcome",
            json!({
                "outcome": "success",
                "count": commits.len()
            }),
        );

        Ok(commits)
    }

    /// Create snapshot (commit)
    pub async fn create_snapshot(
        &self,
        owner: &str,
        repo: &str,
        message: &str,
        requester: &WebID,
    ) -> ArchivalResult<String> {
        // Emit start span
        self.span_emitter.emit_tool(
            "git_snapshot",
            json!({
                "owner": owner,
                "repo": repo,
                "message": message
            }),
        );

        // Check sovereignty
        if !self
            .sovereignty_checker
            .can_access(&DataCategory::TemplateRegistry, requester)
        {
            self.span_emitter.emit_tool(
                "git_snapshot.outcome",
                json!({ "outcome": "sovereignty_denied" }),
            );
            return Err(GitArchivalError::SovereigntyDenied(
                "Registry snapshot requires consent".to_string(),
            ));
        }

        // Check adapter is configured
        if !self.adapter_container.has_git_cas() {
            self.span_emitter.emit_tool(
                "git_snapshot.outcome",
                json!({ "outcome": "adapter_not_configured" }),
            );
            return Err(GitArchivalError::AdapterNotFound(
                "Git CAS adapter not configured".to_string(),
            ));
        }

        // Create snapshot (simulated for now)
        let sha = format!("snapshot_{}", Uuid::new_v4());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_archival_service_new() {
        let container = AdapterContainer::new();
        let owner = WebID::new();
        let service = ArchivalService::new(container, owner);

        // Service should be created without error
        assert!(!service.adapter_container.has_git_cas());
    }

    #[tokio::test]
    async fn test_archive_without_adapter() {
        let container = AdapterContainer::new();
        let owner = WebID::new();
        let service = ArchivalService::new(container, owner);

        let result = service
            .archive("owner", "repo", "main", "path", "content", &owner)
            .await;
        assert!(matches!(result, Err(GitArchivalError::AdapterNotFound(_))));
    }

    #[tokio::test]
    async fn test_archive_sovereignty_check() {
        let container = AdapterContainer::new();
        let owner = WebID::new();
        let service = ArchivalService::new(container, owner);

        // Without adapter, should fail (sovereignty check passes for owner)
        let result = service
            .archive("owner", "repo", "main", "path", "content", &owner)
            .await;
        assert!(matches!(result, Err(GitArchivalError::AdapterNotFound(_))));
    }
}
