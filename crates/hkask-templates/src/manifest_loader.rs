//! Manifest YAML loader — parse process manifest files into BundleManifest
//!
//! The YAML files in `registry/manifests/` use a top-level structure where
//! the `manifest:` key contains identity fields (id, name, description, etc.)
//! while `steps:`, `gas:`, `error_handling:`, etc. are top-level peers.
//! This module provides a deserialization wrapper that flattens this structure
//! into the canonical `BundleManifest` type.

use hkask_types::BundleManifest;
use serde::Deserialize;
use tracing::info;

/// Wrapper struct for deserializing YAML manifest files.
///
/// YAML manifest files have this structure:
/// ```yaml
/// manifest:
///   id: ...
///   name: ...
///   ...
/// steps:
///   - ordinal: 1
///     ...
/// gas:
///   ...
/// error_handling:
///   ...
/// ```
///
/// This wrapper flattens the `manifest:` inner fields with the top-level
/// config fields into a single `BundleManifest`.
#[derive(Debug, Deserialize)]
struct ManifestFile {
    manifest: ManifestHeader,
    #[serde(default)]
    steps: Vec<hkask_types::BundleManifestStep>,
    #[serde(default)]
    skills: Vec<hkask_types::BundleSkill>,
    #[serde(default)]
    conflicts: Vec<hkask_types::BundleConflict>,
    #[serde(default)]
    complementarities: Vec<hkask_types::BundleComplementarity>,
    #[serde(default)]
    convergence: Option<hkask_types::ConvergenceConfig>,
    #[serde(default)]
    gas: Option<hkask_types::GasConfig>,
    #[serde(default)]
    error_handling: Option<hkask_types::ErrorHandlingConfig>,
    #[serde(default)]
    ocap: Option<hkask_types::OcapConfig>,
    #[serde(default)]
    cns: Option<hkask_types::CnsConfig>,
    #[serde(default)]
    audit: Option<hkask_types::AuditConfig>,
}

/// Inner header from the `manifest:` key in YAML files.
#[derive(Debug, Deserialize)]
struct ManifestHeader {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    editor: String,
    #[serde(default)]
    visibility: Option<hkask_types::Visibility>,
}

/// Load a BundleManifest from a YAML file at the given path.
///
/// Reads the file, parses it using the `ManifestFile` wrapper, and
/// flattens the structure into a canonical `BundleManifest`.
pub fn load_manifest_from_file(
    path: &std::path::Path,
) -> Result<BundleManifest, ManifestLoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| ManifestLoadError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    load_manifest_from_yaml(&content)
}

/// Load a BundleManifest from a YAML string.
///
/// Parses the YAML using the `ManifestFile` wrapper and flattens
/// it into a canonical `BundleManifest`.
pub fn load_manifest_from_yaml(yaml: &str) -> Result<BundleManifest, ManifestLoadError> {
    let file: ManifestFile =
        serde_yaml::from_str(yaml).map_err(|e| ManifestLoadError::Yaml { source: e })?;

    let manifest = BundleManifest {
        id: file.manifest.id,
        name: file.manifest.name,
        description: file.manifest.description,
        version: file.manifest.version,
        editor: file.manifest.editor,
        visibility: file
            .manifest
            .visibility
            .unwrap_or(hkask_types::Visibility::Shared),
        skills: file.skills,
        conflicts: file.conflicts,
        complementarities: file.complementarities,
        steps: file.steps,
        convergence: file.convergence.unwrap_or_default(),
        gas: file.gas.unwrap_or_default(),
        error_handling: file.error_handling.unwrap_or_default(),
        ocap: file.ocap.unwrap_or_default(),
        cns: file.cns.unwrap_or_default(),
        audit: file.audit.unwrap_or_default(),
    };

    info!(
        target: "hkask.manifest_loader",
        id = %manifest.id,
        steps = manifest.steps.len(),
        "Loaded manifest from YAML"
    );

    Ok(manifest)
}

/// Resolve a process_manifest reference to a BundleManifest.
///
/// The `process_manifest` field on an agent definition can be:
/// - A file path (contains '/' or '.'): loaded from disk
/// - A manifest ID: looked up from the registry
///
/// Returns `None` if the manifest cannot be found or loaded (logs a warning).
pub fn resolve_manifest(
    reference: &str,
    registry: &dyn hkask_types::ports::BundleRegistryIndex,
) -> Option<BundleManifest> {
    // Try as a registry ID first
    if let Some(bundle) = registry.get_bundle(reference) {
        return Some(bundle);
    }

    // Try as a file path
    let path = std::path::Path::new(reference);
    if path.exists() {
        match load_manifest_from_file(path) {
            Ok(manifest) => {
                info!(
                    target: "hkask.manifest_loader",
                    id = %manifest.id,
                    path = reference,
                    "Loaded manifest from file"
                );
                return Some(manifest);
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.manifest_loader",
                    path = reference,
                    error = %e,
                    "Failed to load manifest from file"
                );
            }
        }
    }

    // Try as a relative path from CWD
    let cwd_path = std::path::PathBuf::from(reference);
    if cwd_path.exists() {
        match load_manifest_from_file(&cwd_path) {
            Ok(manifest) => {
                info!(
                    target: "hkask.manifest_loader",
                    id = %manifest.id,
                    path = reference,
                    "Loaded manifest from relative path"
                );
                return Some(manifest);
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.manifest_loader",
                    path = reference,
                    error = %e,
                    "Failed to load manifest from relative path"
                );
            }
        }
    }

    tracing::warn!(
        target: "hkask.manifest_loader",
        reference = reference,
        "Manifest not found in registry or filesystem"
    );
    None
}

/// Errors that can occur when loading a manifest from YAML.
#[derive(Debug, thiserror::Error)]
pub enum ManifestLoadError {
    #[error("IO error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("YAML parse error: {source}")]
    Yaml { source: serde_yaml::Error },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_minimal_manifest() {
        let yaml = r#"
manifest:
  id: test-manifest
  name: Test
  description: A test manifest
  version: "1.0"
  editor: test
  visibility: Public
steps:
  - ordinal: 1
    action: populate
    description: Test step
    gas_cap: 1000
    timeout_seconds: 30
    phase: Core
"#;
        let manifest = load_manifest_from_yaml(yaml).expect("should parse");
        assert_eq!(manifest.id, "test-manifest");
        assert_eq!(manifest.steps.len(), 1);
        assert_eq!(manifest.steps[0].action, "populate");
    }

    #[test]
    fn load_manifest_with_config() {
        let yaml = r#"
manifest:
  id: configured-manifest
  name: Configured
  description: A manifest with config
  version: "0.22.0"
  editor: curator
  visibility: Shared
gas:
  cap: 18000
  cost_per_token: 0.25
  alert_threshold: 0.8
  hard_limit: true
steps:
  - ordinal: 1
    action: select
    description: Select operation
    template_ref: test/selector
    model_tier: reasoning_local
    gas_cap: 5000
    timeout_seconds: 60
    phase: Pre
"#;
        let manifest = load_manifest_from_yaml(yaml).expect("should parse");
        assert_eq!(manifest.id, "configured-manifest");
        assert_eq!(manifest.gas.cap, 18000);
        assert_eq!(manifest.steps.len(), 1);
        assert_eq!(manifest.steps[0].action, "select");
    }

    #[test]
    fn load_manifest_defaults() {
        let yaml = r#"
manifest:
  id: minimal
  name: ""
  description: ""
  version: ""
  editor: ""
steps: []
"#;
        let manifest = load_manifest_from_yaml(yaml).expect("should parse");
        assert_eq!(manifest.id, "minimal");
        // Defaults should be applied
        assert!(manifest.gas.cap > 0);
        assert!(manifest.error_handling.max_retries > 0);
    }
}
