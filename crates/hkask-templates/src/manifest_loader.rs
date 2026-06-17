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
        serde_yaml_neo::from_str(yaml).map_err(|e| ManifestLoadError::Yaml { source: e })?;

    let manifest = BundleManifest {
        id: file.manifest.id,
        name: file.manifest.name,
        description: file.manifest.description,
        version: file.manifest.version,
        editor: file.manifest.editor,
        visibility: file.manifest.visibility.unwrap_or(Visibility::Public),
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
///
/// REQ: P3-tpl-resolve-manifest
/// \[P3\] Motivating: Generative Space — resolves template manifest references
/// \[P8\] Constraining: Semantic Grounding — manifest terms validated against hLexicon
/// pre:  reference is non-empty, registry is initialized
/// post: returns Some(BundleManifest) if found via registry or file path
/// post: returns None if not found (graceful degradation)
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
    Yaml { source: serde_yaml_neo::Error },
}
