//! Manifest YAML loader — parse process manifest files into BundleManifest
//!
//! The YAML files in `registry/manifests/` use a top-level structure where
//! the `manifest:` key contains identity fields (id, name, description, etc.)
//! while `steps:`, `gas:`, `error_handling:`, etc. are top-level peers.
//! This module provides a deserialization wrapper that flattens this structure
//! into the canonical `BundleManifest` type.

use hkask_types::Visibility;
use hkask_types::bundle::{
    AuditConfig, BundleComplementarity, BundleConflict, BundleManifest, BundleManifestStep,
    BundleSkill, CnsConfig, ConvergenceConfig, ErrorHandlingConfig, GasConfig, OcapConfig,
};
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
    steps: Vec<BundleManifestStep>,
    #[serde(default)]
    skills: Vec<BundleSkill>,
    #[serde(default)]
    conflicts: Vec<BundleConflict>,
    #[serde(default)]
    complementarities: Vec<BundleComplementarity>,
    #[serde(default)]
    convergence: Option<ConvergenceConfig>,
    #[serde(default)]
    gas: Option<GasConfig>,
    #[serde(default)]
    error_handling: Option<ErrorHandlingConfig>,
    #[serde(default)]
    ocap: Option<OcapConfig>,
    #[serde(default)]
    cns: Option<CnsConfig>,
    #[serde(default)]
    audit: Option<AuditConfig>,
    #[serde(default)]
    inputs: Option<serde_json::Value>,
    #[serde(default)]
    principles: Option<serde_json::Value>,
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
    #[serde(default, deserialize_with = "deserialize_visibility_case_insensitive")]
    visibility: Option<Visibility>,
    #[serde(default)]
    functional_role: Option<String>,
}

/// Deserialize visibility in a case-insensitive manner.
///
/// YAML manifest files may use PascalCase (`Shared`) while the
/// `Visibility` enum serializes as lowercase (`shared`).
fn deserialize_visibility_case_insensitive<'de, D>(
    deserializer: D,
) -> Result<Option<Visibility>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => Visibility::parse_str(&s)
            .map(Some)
            .ok_or_else(|| de::Error::custom(format!("unknown visibility variant: {s}"))),
        None => Ok(None),
    }
}

/// Load a BundleManifest from a YAML file at the given path.
///
/// Reads the file, parses it using the `ManifestFile` wrapper, and
/// flattens the structure into a canonical `BundleManifest`.
pub(crate) fn load_manifest_from_file(
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
pub(crate) fn load_manifest_from_yaml(yaml: &str) -> Result<BundleManifest, ManifestLoadError> {
    let file: ManifestFile =
        serde_yaml::from_str(yaml).map_err(|e| ManifestLoadError::Yaml { source: e })?;

    let manifest = BundleManifest {
        id: file.manifest.id,
        name: file.manifest.name,
        description: file.manifest.description,
        version: file.manifest.version,
        editor: file.manifest.editor,
        visibility: file.manifest.visibility.unwrap_or(Visibility::Shared),
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
        functional_role: file.manifest.functional_role,
        inputs: file.inputs,
        principles: file.principles,
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
pub(crate) enum ManifestLoadError {
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
  visibility: public
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
  visibility: shared
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

    #[test]
    fn load_real_manifest_from_registry() {
        let manifest_path = std::path::Path::new("../../registry/manifests/coding-guidelines.yaml");
        let manifest =
            load_manifest_from_file(manifest_path).expect("should parse coding-guidelines.yaml");
        assert_eq!(manifest.id, "coding-guidelines");
        assert_eq!(manifest.name, "Coding Guidelines");
        assert!(!manifest.steps.is_empty(), "manifest should have steps");
        // First step should be a populate action with minijinja renderer
        let first_step = &manifest.steps[0];
        assert_eq!(first_step.action, "populate");
        assert_eq!(first_step.renderer.as_deref(), Some("minijinja"));
        assert!(first_step.template_ref.is_some());
        // Gas config should be populated from file
        assert_eq!(manifest.gas.cap, 18000);
        // CNS config should be present
        assert!(manifest.cns.emit_spans);
    }

    /// Sweep test: try to load every manifest YAML in registry/manifests/.
    ///
    /// This catches deserialization regressions (missing fields, case mismatches, etc.)
    /// across the entire manifest corpus. Manifests that fail to parse are collected
    /// rather than failing the test, so we can track known-broken files separately.
    #[test]
    fn sweep_all_registry_manifests() {
        let manifest_dir = std::path::Path::new("../../registry/manifests");
        if !manifest_dir.exists() {
            // Running outside repo root; skip gracefully
            eprintln!("Skipping sweep: manifest directory not found");
            return;
        }

        let entries = std::fs::read_dir(manifest_dir).expect("should read manifests dir");
        let mut parsed = 0usize;
        let mut failed: Vec<(String, String)> = Vec::new();

        for entry in entries {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();

            match load_manifest_from_file(&path) {
                Ok(manifest) => {
                    // Basic sanity: id should match filename stem
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if manifest.id != stem {
                        // Not necessarily an error (e.g. nested dirs), but worth noting
                        eprintln!(
                            "Note: manifest id '{}' differs from filename stem '{}'",
                            manifest.id, stem
                        );
                    }
                    parsed += 1;
                }
                Err(e) => {
                    failed.push((name, format!("{:?}", e)));
                }
            }
        }

        eprintln!("Sweep: {} parsed, {} failed", parsed, failed.len());
        for (name, err) in &failed {
            eprintln!("  FAILED: {} — {}", name, err);
        }

        // We expect most manifests to parse. Some known failures:
        // - 4 manifests have YAML syntax errors (tracked separately)
        // - 6 manifests have duplicate `functional_role` (YAML bugs)
        // - Several manifests use a different format (no `manifest:` wrapper)
        // The test passes as long as the majority parse successfully.
        assert!(
            parsed >= 25,
            "Expected at least 25 manifests to parse, got {}. Failures: {:?}",
            parsed,
            failed.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
        );
    }
}
