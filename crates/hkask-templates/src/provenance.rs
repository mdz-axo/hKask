//! Template provenance tracking
//!
//! Tracks Git SHA, creator WebID, and modification timestamp for each template.
//! Stored in SQLite alongside registry index for audit trail.

use chrono::{DateTime, Utc};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};

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
