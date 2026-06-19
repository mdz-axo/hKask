//! YAML schema validation tests — Wave 6 Task 6.1
//!
//! Validates that all registry manifest YAML files are well-formed
//! and contain required fields. Catches malformed manifests at test time
//! rather than at runtime.
//!
//! # Principle grounding
//! - P8 (Semantic Grounding): config errors should be caught before runtime
//! - P11 (Digital Public/Private Sphere): visibility must be canonical

use serde::Deserialize;
use std::path::Path;

/// Minimal manifest structure for validation.
#[derive(Debug, Deserialize)]
struct ManifestFile {
    manifest: ManifestHeader,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ManifestHeader {
    id: String,
    name: String,
    description: String,
    version: String,
    #[serde(default)]
    visibility: Option<String>,
    #[serde(default)]
    functional_role: Option<String>,
}

// [P3] Motivating: Generative Space — validates registry manifests are well-formed
// [P8] Constraining: Semantic Grounding — required fields present and correctly typed
// All registry manifests are well-formed YAML with required fields.

#[test]
fn all_skill_manifests_are_well_formed() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_dir.join("../..");
    let manifest_dir = workspace_root.join("registry/manifests");
    if !manifest_dir.exists() {
        eprintln!("{} not found — skipping test", manifest_dir.display());
        return;
    }

    let mut errors = Vec::new();
    let mut count = 0;

    for entry in walkdir::WalkDir::new(manifest_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
    {
        count += 1;
        let path = entry.path();
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_yaml_neo::from_str::<ManifestFile>(&content) {
                Ok(mf) => {
                    assert!(
                        !mf.manifest.id.is_empty(),
                        "{}: manifest.id is empty",
                        path.display()
                    );
                    assert!(
                        !mf.manifest.name.is_empty(),
                        "{}: manifest.name is empty",
                        path.display()
                    );
                    // P3: description must be present (Generative Space requires discoverability)
                    assert!(
                        !mf.manifest.description.is_empty(),
                        "{}: manifest.description is empty",
                        path.display()
                    );
                    // P7: version must be present (Evolutionary Architecture requires versioning)
                    assert!(
                        !mf.manifest.version.is_empty(),
                        "{}: manifest.version is empty",
                        path.display()
                    );
                    // P11: visibility must be present and canonical (Public or Private only)
                    let vis = mf.manifest.visibility.as_deref().unwrap_or("");
                    assert!(
                        !vis.is_empty(),
                        "{}: manifest.visibility is missing",
                        path.display()
                    );
                    assert!(
                        vis == "Public" || vis == "Private",
                        "{}: manifest.visibility is '{vis}' — must be Public or Private (P11)",
                        path.display()
                    );
                    // functional_role should be present if the manifest uses it
                    // (Note: some manifests like kata and improv use alternative structural schemas)
                }
                Err(e) => {
                    errors.push(format!("{}: YAML parse error: {}", path.display(), e));
                }
            },
            Err(e) => {
                errors.push(format!("{}: IO error: {}", path.display(), e));
            }
        }
    }

    if !errors.is_empty() {
        panic!(
            "{} of {} manifests failed validation:\n{}",
            errors.len(),
            count,
            errors.join("\n")
        );
    }

    eprintln!("Validated {} manifests — all well-formed", count);
}

// [P3] Motivating: Generative Space — validates registry manifests are well-formed
// [P8] Constraining: Semantic Grounding — required fields present and correctly typed
#[test]
fn invalid_yaml_is_rejected() {
    let invalid = "id: 123\nname: []\n"; // name should be string, not array
    let result = serde_yaml_neo::from_str::<ManifestFile>(invalid);
    assert!(result.is_err(), "invalid YAML should be rejected");
}
