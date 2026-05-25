//! Git CAS Adapter
//!
//! Concrete implementation of GitCASPort using gix crate.

use crate::error::GitError;
use crate::ports::GitCASPort;
use crate::pod::{TemplateCrate, TemplateFile};
use std::path::{Component, Path};

/// Git CAS Adapter — Concrete implementation for template crate loading
pub struct GitCasAdapter {
    base_path: std::path::PathBuf,
}

impl GitCasAdapter {
    /// Create new Git CAS adapter
    pub fn new(base_path: &Path) -> Result<Self, GitError> {
        if !base_path.exists() {
            return Err(GitError::CrateNotFound(format!(
                "Base path does not exist: {:?}",
                base_path
            )));
        }

        Ok(Self {
            base_path: base_path.to_path_buf(),
        })
    }

    /// Create from base path without validation
    pub fn from_path(base_path: std::path::PathBuf) -> Self {
        Self { base_path }
    }

    /// Validate a path to prevent directory traversal attacks
    pub fn validate_path(&self, path: &Path) -> Result<(), GitError> {
        let path_str = path.to_string_lossy();

        if path_str.contains('\0') {
            return Err(GitError::InvalidPath(
                "Path contains null bytes".to_string(),
            ));
        }

        if path.is_absolute() {
            return Err(GitError::InvalidPath(
                "Absolute paths not allowed".to_string(),
            ));
        }

        for component in path.components() {
            if let Component::ParentDir = component {
                return Err(GitError::InvalidPath(
                    "Parent directory traversal not allowed".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl GitCASPort for GitCasAdapter {
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, GitError> {
        let crate_path = Path::new(crate_name);

        self.validate_path(crate_path)?;

        let full_path = self.base_path.join(crate_name);

        if !full_path.exists() {
            return Err(GitError::CrateNotFound(format!(
                "Crate path does not exist: {:?}",
                full_path
            )));
        }

        let persona_path = full_path.join("agent_persona.yaml");
        let persona_yaml = std::fs::read_to_string(&persona_path)
            .map_err(|e| GitError::Io(format!("Failed to read persona: {}", e)))?;

        let manifest_path = full_path.join("dispatch_manifest.yaml");
        let dispatch_manifest_yaml = std::fs::read_to_string(&manifest_path)
            .map_err(|e| GitError::Io(format!("Failed to read manifest: {}", e)))?;

        let templates_dir = full_path.join("templates");
        let mut templates = Vec::new();

        if templates_dir.exists() {
            for entry in std::fs::read_dir(&templates_dir)
                .map_err(|e| GitError::Io(format!("Failed to read templates dir: {}", e)))?
            {
                let entry =
                    entry.map_err(|e| GitError::Io(format!("Failed to read entry: {}", e)))?;
                let path = entry.path();
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| GitError::Io(format!("Failed to read template: {}", e)))?;

                let template_type = match path.extension().and_then(|s| s.to_str()) {
                    Some("j2") => "Prompt",
                    Some("yaml") => "Process",
                    _ => "Cognition",
                };

                templates.push(TemplateFile {
                    path: path.to_string_lossy().to_string(),
                    content,
                    template_type: template_type.to_string(),
                });
            }
        }

        let hlexicon_path = full_path.join("hlexicon.yaml");
        let hlexicon_terms = if hlexicon_path.exists() {
            let content = std::fs::read_to_string(&hlexicon_path)
                .map_err(|e| GitError::Io(format!("Failed to read hlexicon: {}", e)))?;
            parse_hlexicon_terms(&content)
        } else {
            Vec::new()
        };

        let git_sha = self.resolve_sha(crate_name)?;

        Ok(TemplateCrate {
            name: crate_name.to_string(),
            git_sha,
            persona_yaml,
            dispatch_manifest_yaml,
            templates,
            hlexicon_terms,
        })
    }

    fn resolve_sha(&self, _crate_name: &str) -> Result<String, GitError> {
        use std::process::Command;

        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.base_path)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    Ok(sha)
                } else {
                    Ok("0000000000000000000000000000000000000000".to_string())
                }
            }
            Err(_) => Ok("0000000000000000000000000000000000000000".to_string()),
        }
    }
}

fn parse_hlexicon_terms(content: &str) -> Vec<String> {
    let mut terms = Vec::new();

    if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        match value {
            serde_yaml::Value::Sequence(seq) => {
                for item in seq {
                    if let serde_yaml::Value::String(term) = item {
                        terms.push(term);
                    }
                }
            }
            serde_yaml::Value::String(term) => {
                terms.push(term);
            }
            _ => {}
        }
    }

    terms
}

/// Mock Git CAS for testing
pub struct MockGitCas;

impl MockGitCas {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockGitCas {
    fn default() -> Self {
        Self::new()
    }
}

impl GitCASPort for MockGitCas {
    fn load_template_crate(&self, _crate_name: &str) -> Result<TemplateCrate, GitError> {
        Ok(TemplateCrate {
            name: "mock".to_string(),
            git_sha: "0000000000000000000000000000000000000000".to_string(),
            persona_yaml: String::new(),
            dispatch_manifest_yaml: String::new(),
            templates: vec![],
            hlexicon_terms: vec![],
        })
    }

    fn resolve_sha(&self, _crate_name: &str) -> Result<String, GitError> {
        Ok("0000000000000000000000000000000000000000".to_string())
    }
}
