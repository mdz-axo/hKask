//! Git CAS Adapter
//
//! Concrete implementation for template crate loading and Git operations.
//! Also provides `load_template_crate_or_synthesize` which bridges the
//! filesystem crate system and the hkask-templates::Registry. When a
//! crate directory exists on disk, it's loaded normally. When absent,
//! a minimal TemplateCrate is synthesized from the registry's template
//! files and agent persona data — eliminating "crate not found" errors
//! for templates registered in the registry but lacking a dedicated
//! crate directory.

pub mod gix_adapter;

pub use gix_adapter::GixCasAdapter;

use hkask_types::InfrastructureError;
use hkask_types::template::{TemplateCrate, TemplateFile};
use std::path::{Component, Path};

/// Git CAS Adapter — Concrete implementation for template crate loading
pub struct GitCasAdapter {
    base_path: std::path::PathBuf,
}

impl GitCasAdapter {
    /// Create from base path without validation.
    ///
    /// REQ: MCP-021
    /// pre:  base_path is a valid directory path
    /// post: returns GitCasAdapter rooted at base_path
    pub fn from_path(base_path: std::path::PathBuf) -> Self {
        Self { base_path }
    }

    /// Validate a path to prevent directory traversal attacks
    pub(crate) fn validate_path(&self, path: &Path) -> Result<(), InfrastructureError> {
        let path_str = path.to_string_lossy();

        if path_str.contains('\0') {
            return Err(InfrastructureError::Io(
                "Path contains null bytes".to_string(),
            ));
        }

        if path.is_absolute() {
            return Err(InfrastructureError::Io(
                "Absolute paths not allowed".to_string(),
            ));
        }

        for component in path.components() {
            if let Component::ParentDir = component {
                return Err(InfrastructureError::Io(
                    "Parent directory traversal not allowed".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Load a template crate from the content-addressable store.
    ///
    /// REQ: MCP-022
    /// pre:  crate_name is a valid, non-traversal path
    /// post: returns TemplateCrate loaded from CAS
    /// post: returns Err(NotFound) if crate doesn't exist
    pub fn load_template_crate(
        &self,
        crate_name: &str,
    ) -> Result<TemplateCrate, InfrastructureError> {
        let crate_path = Path::new(crate_name);

        self.validate_path(crate_path)?;

        let full_path = self.base_path.join(crate_name);

        if !full_path.exists() {
            return Err(InfrastructureError::NotFound(format!(
                "Crate path does not exist: {:?}",
                full_path
            )));
        }

        let persona_path = full_path.join("agent_persona.yaml");
        let persona_yaml = std::fs::read_to_string(&persona_path)
            .map_err(|e| InfrastructureError::Io(format!("Failed to read persona: {}", e)))?;

        let manifest_path = full_path.join("dispatch_manifest.yaml");
        let dispatch_manifest_yaml = std::fs::read_to_string(&manifest_path)
            .map_err(|e| InfrastructureError::Io(format!("Failed to read manifest: {}", e)))?;

        let templates_dir = full_path.join("templates");
        let mut templates = Vec::new();

        if templates_dir.exists() {
            for entry in std::fs::read_dir(&templates_dir).map_err(|e| {
                InfrastructureError::Io(format!("Failed to read templates dir: {}", e))
            })? {
                let entry = entry
                    .map_err(|e| InfrastructureError::Io(format!("Failed to read entry: {}", e)))?;
                let path = entry.path();
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    InfrastructureError::Io(format!("Failed to read template: {}", e))
                })?;

                let template_type = match path.extension().and_then(|s| s.to_str()) {
                    Some("j2") => "KnowAct",
                    Some("yaml") => "FlowDef",
                    _ => "KnowAct",
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
                .map_err(|e| InfrastructureError::Io(format!("Failed to read hlexicon: {}", e)))?;
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

    /// Resolve the current SHA for a crate
    pub(crate) fn resolve_sha(&self, _crate_name: &str) -> Result<String, InfrastructureError> {
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
