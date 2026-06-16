//! YAML schema validation tests — Wave 6 Task 6.1
//!
//! Validates that all registry manifest YAML files are well-formed
//! and contain required fields. Catches malformed manifests at test time
//! rather than at runtime.
//!
//! # Principle grounding
//! - P8 (Semantic Grounding): config errors should be caught before runtime

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
}

// REQ: YML-001 — Manifest schema validation (P8)
// All registry manifests are well-formed YAML with required fields.

#[test]
fn all_skill_manifests_are_well_formed() {
    let manifest_dir = Path::new("registry/manifests");
    if !manifest_dir.exists() {
        eprintln!("registry/manifests/ not found — skipping test");
        return;
    }

    let mut errors = Vec::new();
    let mut count = 0;

    for entry in walkdir::WalkDir::new(manifest_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml"))
    {
        count += 1;
        let path = entry.path();
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_yaml::from_str::<ManifestFile>(&content) {
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

#[test]
fn invalid_yaml_is_rejected() {
    let invalid = "id: 123\nname: []\n"; // name should be string, not array
    let result = serde_yaml::from_str::<ManifestFile>(invalid);
    assert!(result.is_err(), "invalid YAML should be rejected");
}
