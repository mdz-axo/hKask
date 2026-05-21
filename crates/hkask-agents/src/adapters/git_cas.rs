//! Git CAS Adapter
//!
//! Concrete implementation of GitCASPort using gix crate.

use crate::pod::{GitCASPort, TemplateCrate, TemplateFile};
use std::path::Path;

/// Git CAS Adapter — Concrete implementation for template crate loading
pub struct GitCasAdapter {
    base_path: std::path::PathBuf,
}

impl GitCasAdapter {
    /// Create new Git CAS adapter
    pub fn new(base_path: &Path) -> Result<Self, String> {
        // Verify base path exists
        if !base_path.exists() {
            return Err(format!("Base path does not exist: {:?}", base_path));
        }

        Ok(Self {
            base_path: base_path.to_path_buf(),
        })
    }

    /// Create from base path without validation
    pub fn from_path(base_path: std::path::PathBuf) -> Self {
        Self { base_path }
    }
}

impl GitCASPort for GitCasAdapter {
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, String> {
        // Load template crate from Git CAS
        let crate_path = self.base_path.join(crate_name);

        if !crate_path.exists() {
            return Err(format!("Crate path does not exist: {:?}", crate_path));
        }

        // Read agent_persona.yaml
        let persona_path = crate_path.join("agent_persona.yaml");
        let persona_yaml = std::fs::read_to_string(&persona_path)
            .map_err(|e| format!("Failed to read persona: {}", e))?;

        // Read dispatch_manifest.yaml
        let manifest_path = crate_path.join("dispatch_manifest.yaml");
        let dispatch_manifest_yaml = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        // Load templates from templates/ directory
        let templates_dir = crate_path.join("templates");
        let mut templates = Vec::new();

        if templates_dir.exists() {
            for entry in std::fs::read_dir(&templates_dir)
                .map_err(|e| format!("Failed to read templates dir: {}", e))?
            {
                let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                let path = entry.path();
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read template: {}", e))?;

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

        // Read hlexicon.yaml
        let hlexicon_path = crate_path.join("hlexicon.yaml");
        let hlexicon_terms = if hlexicon_path.exists() {
            let content = std::fs::read_to_string(&hlexicon_path)
                .map_err(|e| format!("Failed to read hlexicon: {}", e))?;
            // Parse YAML to extract terms
            parse_hlexicon_terms(&content)
        } else {
            Vec::new()
        };

        // Get current Git SHA
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

    fn resolve_sha(&self, _crate_name: &str) -> Result<String, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_git_cas_adapter_new() {
        let temp_dir = std::env::temp_dir().join("hkask_git_test");
        fs::create_dir_all(&temp_dir).unwrap();

        let adapter = GitCasAdapter::new(&temp_dir);
        assert!(adapter.is_ok());

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_git_cas_adapter_nonexistent_path() {
        let nonexistent = std::path::PathBuf::from("/nonexistent/path/that/does/not/exist");
        let result = GitCasAdapter::new(&nonexistent);
        assert!(result.is_err());
    }
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
    fn load_template_crate(&self, _crate_name: &str) -> Result<TemplateCrate, String> {
        // Stub implementation for testing
        Ok(TemplateCrate {
            name: "mock".to_string(),
            git_sha: "0000000000000000000000000000000000000000".to_string(),
            persona_yaml: String::new(),
            dispatch_manifest_yaml: String::new(),
            templates: vec![],
            hlexicon_terms: vec![],
        })
    }

    fn resolve_sha(&self, _crate_name: &str) -> Result<String, String> {
        Ok("0000000000000000000000000000000000000000".to_string())
    }
}
