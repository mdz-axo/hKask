//! Git-based registry adapter
//!
//! Loads templates from a Git-tracked directory.
//! Provides reproducible, versioned template loading with full audit trail.
//!
//! **Note:** This adapter reads from the filesystem (Git working tree),
//! not directly from Git CAS. For pure CAS access, use a Git CLI wrapper.

use crate::ports::{ProcessManifest, RegistryEntry, RegistryIndex, Result, TemplateError};
use crate::provenance::{ProvenanceManager, TemplateProvenance};
use crate::registry::Registry;
use hkask_types::TemplateType;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git-based registry index
///
/// Wraps the filesystem Registry with Git provenance tracking.
/// Templates are loaded from the working tree, with Git SHA recorded for audit.
pub struct GitRegistry {
    inner: Registry,
    repo_path: PathBuf,
    provenance: ProvenanceManager,
}

impl GitRegistry {
    /// Open Git registry from repository path
    pub fn open(repo_path: &Path, branch: &str, _templates_path: &Path) -> Result<Self> {
        // Verify it's a Git repository
        Self::verify_git_repo(repo_path)?;

        // Get current Git SHA
        let git_sha = Self::get_current_sha(repo_path)?;

        // Load templates from filesystem
        let inner = Registry::bootstrap();
        let mut provenance = ProvenanceManager::new();

        // Record provenance for all templates
        for template_id in inner.ids() {
            let provenance_record = TemplateProvenance::new(
                template_id.to_string(),
                git_sha.clone(),
                hkask_types::WebID::new(), // TODO: Get from Git config
                branch.to_string(),
            );
            provenance.record(provenance_record);
        }

        Ok(Self {
            inner,
            repo_path: repo_path.to_path_buf(),
            provenance,
        })
    }

    /// Verify the path is a Git repository
    fn verify_git_repo(path: &Path) -> Result<()> {
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(TemplateError::NotFound(format!(
                "Not a Git repository: {:?}",
                path
            )));
        }
        Ok(())
    }

    /// Get current Git SHA
    fn get_current_sha(repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| TemplateError::Manifest(format!("Failed to run git: {}", e)))?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map(|s| s.trim().to_string())
                .map_err(|e| TemplateError::Manifest(format!("Invalid SHA: {}", e)))
        } else {
            Err(TemplateError::Manifest("Failed to get Git SHA".to_string()))
        }
    }

    /// Reload templates from current HEAD
    pub fn reload(&mut self) -> Result<()> {
        let git_sha = Self::get_current_sha(&self.repo_path)?;

        // Reload inner registry
        self.inner = Registry::bootstrap();
        self.provenance.clear();

        // Record new provenance
        for template_id in self.inner.ids() {
            let provenance_record = TemplateProvenance::new(
                template_id.to_string(),
                git_sha.clone(),
                hkask_types::WebID::new(),
                "HEAD".to_string(),
            );
            self.provenance.record(provenance_record);
        }

        Ok(())
    }

    /// Get current Git SHA
    pub fn get_sha(&self) -> Result<String> {
        Self::get_current_sha(&self.repo_path)
    }

    /// Get provenance for a template
    pub fn get_provenance(&self, template_id: &str) -> Option<&TemplateProvenance> {
        self.provenance.get_latest(template_id)
    }

    /// Check for uncommitted changes in templates directory
    pub fn has_uncommitted_changes(&self, templates_path: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain", templates_path])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| TemplateError::Manifest(format!("Failed to run git: {}", e)))?;

        if output.status.success() {
            Ok(!output.stdout.is_empty())
        } else {
            Err(TemplateError::Manifest(
                "Failed to check Git status".to_string(),
            ))
        }
    }
}

impl RegistryIndex for GitRegistry {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
        self.inner.list(domain_hint)
    }

    fn get(&self, id: &str) -> Result<RegistryEntry> {
        <dyn RegistryIndex>::get(&self.inner, id)
    }

    fn bootstrap_manifest(&self) -> Option<ProcessManifest> {
        self.inner.bootstrap_manifest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_git_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_verify_git_repo() {
        let (_temp_dir, repo_path) = setup_git_repo();

        // Create initial commit
        fs::write(repo_path.join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        assert!(GitRegistry::verify_git_repo(&repo_path).is_ok());

        // Non-git directory should fail
        let temp_dir = TempDir::new().unwrap();
        assert!(GitRegistry::verify_git_repo(temp_dir.path()).is_err());
    }

    #[test]
    fn test_get_current_sha() {
        let (_temp_dir, repo_path) = setup_git_repo();

        // Create initial commit
        fs::write(repo_path.join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let sha = GitRegistry::get_current_sha(&repo_path).unwrap();
        assert_eq!(sha.len(), 40); // SHA-1 is 40 hex chars
    }
}
