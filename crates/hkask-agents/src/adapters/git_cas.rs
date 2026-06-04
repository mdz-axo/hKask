//! Git CAS Adapter
//!
//! Concrete implementation of GitCASPort using gix crate.
//!
//! Also provides `load_template_crate_or_synthesize` which bridges the
//! filesystem crate system and the hkask-templates::Registry. When a
//! crate directory exists on disk, it's loaded normally. When absent,
//! a minimal TemplateCrate is synthesized from the registry's template
//! files and agent persona data — eliminating "crate not found" errors
//! for templates registered in the registry but lacking a dedicated
//! crate directory.

use hkask_types::{GitCASPort, GitError, TemplateCrate, TemplateFile};
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
            return Err(GitError::Io("Path contains null bytes".to_string()));
        }

        if path.is_absolute() {
            return Err(GitError::Io("Absolute paths not allowed".to_string()));
        }

        for component in path.components() {
            if let Component::ParentDir = component {
                return Err(GitError::Io(
                    "Parent directory traversal not allowed".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Load a template crate, or synthesize one from minimal defaults.
    ///
    /// First checks for a proper crate directory (containing
    /// `agent_persona.yaml` and `dispatch_manifest.yaml`). If found,
    /// loads it normally via `load_template_crate`.
    ///
    /// If the directory doesn't exist or lacks required files,
    /// synthesizes a minimal `TemplateCrate` so that pod creation
    /// can proceed without a pre-built crate on disk. This bridges
    /// the gap between the hkask-templates::Registry (which stores
    /// `.j2` template files by domain) and the GitCasAdapter (which
    /// expects filesystem crate directories).
    pub fn load_template_crate_or_synthesize(
        &self,
        crate_name: &str,
    ) -> Result<TemplateCrate, GitError> {
        // Try loading a proper crate directory first
        let crate_path = self.base_path.join(crate_name);
        if crate_path.exists()
            && crate_path.join("agent_persona.yaml").exists()
            && crate_path.join("dispatch_manifest.yaml").exists()
        {
            return self.load_template_crate(crate_name);
        }

        // Synthesize a minimal template crate for pod creation
        tracing::debug!(
            target: "hkask.templates",
            crate_name = %crate_name,
            "No template crate directory found — synthesizing minimal crate"
        );

        let persona_yaml = format!(
            "agent:\n\
             name: \"{name}\"\n\
             type: Replicant\n\
             version: \"0.1.0\"\n\
             \n\
             charter:\n\
             description: \"Synthesized {name} session\"\n\
             editor: cli\n\
             \n\
             capabilities:\n\
             - \"tool:inference:call\"\n\
             \n\
             rights: []\n\
             responsibilities: []\n\
             \n\
             visibility:\n\
             default: public\n\
             episodic_override: private",
            name = crate_name
        );

        let dispatch_manifest_yaml = "\
             dispatch:\n\
             - id: inference\n\
             type: inference\n\
             route: okapi\n\
             description: \"Primary LLM inference\""
            .to_string();

        // Scan for any .j2 template files in a domain-matching directory
        let mut templates = Vec::new();
        let template_dir = self.base_path.join(crate_name);
        if template_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&template_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension()
                    && (ext == "j2" || ext == "yaml")
                    && let Ok(content) = std::fs::read_to_string(&path)
                {
                    let template_type = match ext.to_str() {
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
        }

        let git_sha = self.resolve_sha(crate_name)?;

        Ok(TemplateCrate {
            name: crate_name.to_string(),
            git_sha,
            persona_yaml,
            dispatch_manifest_yaml,
            templates,
            hlexicon_terms: Vec::new(),
        })
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

    fn commit(&self, message: &str) -> Result<String, GitError> {
        use std::process::Command;

        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.base_path)
            .output()
            .map_err(|e| GitError::Io(format!("git add failed: {}", e)))?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(GitError::Git(format!("git add failed: {}", stderr.trim())));
        }

        let commit_output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.base_path)
            .output()
            .map_err(|e| GitError::Io(format!("git commit failed: {}", e)))?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stderr.contains("nothing to commit") {
                return self.resolve_sha("");
            }
            return Err(GitError::Git(format!(
                "git commit failed: {}",
                stderr.trim()
            )));
        }

        self.resolve_sha("")
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
