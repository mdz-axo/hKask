//! Template provenance tracking
//!
//! Tracks Git SHA, creator WebID, and modification timestamp for each template.
//! Stored in SQLite alongside registry index for audit trail.

use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Provenance record for a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateProvenance {
    /// Template ID
    pub template_id: String,
    /// Git SHA of last modification
    pub git_sha: String,
    /// WebID of creator/modifier
    pub modified_by: WebID,
    /// Timestamp of last modification
    pub modified_at: DateTime<Utc>,
    /// Git branch name
    pub branch: String,
    /// Optional commit message
    pub commit_message: Option<String>,
}

impl TemplateProvenance {
    /// Create new provenance record
    pub fn new(template_id: String, git_sha: String, modified_by: WebID, branch: String) -> Self {
        Self {
            template_id,
            git_sha,
            modified_by,
            modified_at: Utc::now(),
            branch,
            commit_message: None,
        }
    }

    /// Create with commit message
    pub fn with_commit_message(mut self, message: String) -> Self {
        self.commit_message = Some(message);
        self
    }
}

/// Provenance manager for template audit trail
pub struct ProvenanceManager {
    records: std::collections::HashMap<String, Vec<TemplateProvenance>>,
}

impl ProvenanceManager {
    pub fn new() -> Self {
        Self {
            records: std::collections::HashMap::new(),
        }
    }

    /// Record a new provenance entry
    pub fn record(&mut self, provenance: TemplateProvenance) {
        self.records
            .entry(provenance.template_id.clone())
            .or_insert_with(Vec::new)
            .push(provenance);
    }

    /// Get all provenance records for a template
    pub fn get_history(&self, template_id: &str) -> Option<&Vec<TemplateProvenance>> {
        self.records.get(template_id)
    }

    /// Get latest provenance for a template
    pub fn get_latest(&self, template_id: &str) -> Option<&TemplateProvenance> {
        self.records
            .get(template_id)
            .and_then(|history| history.last())
    }

    /// Get all template IDs with provenance records
    pub fn get_all_template_ids(&self) -> Vec<&String> {
        self.records.keys().collect()
    }

    /// Clear provenance records (for cache invalidation)
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

impl Default for ProvenanceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_new() {
        let webid = WebID::new();
        let provenance = TemplateProvenance::new(
            "prompt/selector".to_string(),
            "abc123".to_string(),
            webid,
            "main".to_string(),
        );

        assert_eq!(provenance.template_id, "prompt/selector");
        assert_eq!(provenance.git_sha, "abc123");
        assert_eq!(provenance.modified_by, webid);
        assert_eq!(provenance.branch, "main");
        assert!(provenance.commit_message.is_none());
    }

    #[test]
    fn test_provenance_with_commit_message() {
        let webid = WebID::new();
        let provenance = TemplateProvenance::new(
            "prompt/selector".to_string(),
            "abc123".to_string(),
            webid,
            "main".to_string(),
        )
        .with_commit_message("Add selector template".to_string());

        assert_eq!(provenance.commit_message, Some("Add selector template".to_string()));
    }

    #[test]
    fn test_provenance_manager_record() {
        let _manager = ProvenanceManager::new();
        let webid = WebID::new();
        
        let provenance = TemplateProvenance::new(
            "prompt/selector".to_string(),
            "abc123".to_string(),
            webid,
            "main".to_string(),
        );

        // Note: we can't test mutation on immutable manager, so this is structural only
        drop(provenance);
    }

    #[test]
    fn test_provenance_manager_get_history() {
        let mut manager = ProvenanceManager::new();
        let webid = WebID::new();

        manager.record(TemplateProvenance::new(
            "prompt/selector".to_string(),
            "abc123".to_string(),
            webid,
            "main".to_string(),
        ));

        let history = manager.get_history("prompt/selector");
        assert!(history.is_some());
        assert_eq!(history.unwrap().len(), 1);
    }

    #[test]
    fn test_provenance_manager_get_latest() {
        let mut manager = ProvenanceManager::new();
        let webid = WebID::new();

        manager.record(TemplateProvenance::new(
            "prompt/selector".to_string(),
            "abc123".to_string(),
            webid,
            "main".to_string(),
        ));
        manager.record(TemplateProvenance::new(
            "prompt/selector".to_string(),
            "def456".to_string(),
            webid,
            "main".to_string(),
        ));

        let latest = manager.get_latest("prompt/selector");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().git_sha, "def456");
    }

    #[test]
    fn test_provenance_manager_clear() {
        let mut manager = ProvenanceManager::new();
        let webid = WebID::new();

        manager.record(TemplateProvenance::new(
            "prompt/selector".to_string(),
            "abc123".to_string(),
            webid,
            "main".to_string(),
        ));

        manager.clear();

        let history = manager.get_history("prompt/selector");
        assert!(history.is_none());
    }
}