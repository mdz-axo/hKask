//! Kata command — list and inspect kata manifests
//!
//! Establishes the code path for kata manifest loading. Full execution
//! (step rendering, gas tracking, CNS spans) is future work.

use crate::cli::KataAction;
use std::path::PathBuf;

/// Resolve the registry/manifests directory relative to the project root.
fn manifests_dir() -> PathBuf {
    // Walk up from the binary's location or use CARGO_MANIFEST_DIR heuristic.
    // In development, the binary runs from the workspace root.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join("registry").join("manifests")
}

pub fn run(action: KataAction) {
    match action {
        KataAction::List => list_manifests(),
        KataAction::Show { name } => show_manifest(&name),
    }
}

fn list_manifests() {
    let dir = manifests_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!(
                "Failed to read manifests directory {}: {}",
                dir.display(),
                e
            );
            std::process::exit(1);
        }
    };

    let mut kata_manifests: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains("kata") && name.ends_with(".yaml") {
            kata_manifests.push(name.trim_end_matches(".yaml").to_string());
        }
    }

    if kata_manifests.is_empty() {
        eprintln!("No kata manifests found in {}", dir.display());
        return;
    }

    kata_manifests.sort();
    eprintln!("Kata manifests ({}):", kata_manifests.len());
    for m in &kata_manifests {
        eprintln!("  {}", m);
    }
}

fn show_manifest(name: &str) {
    let dir = manifests_dir();
    let path = dir.join(format!("{}.yaml", name));

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Failed to read manifest '{}' at {}: {}",
                name,
                path.display(),
                e
            );
            std::process::exit(1);
        }
    };

    // Parse as YAML to validate structure
    let parsed: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse manifest '{}': {}", name, e);
            std::process::exit(1);
        }
    };

    // Print structured summary
    if let Some(manifest) = parsed.get("manifest") {
        eprintln!("=== {} ===", name);
        if let Some(n) = manifest.get("name") {
            eprintln!("Name: {}", n.as_str().unwrap_or("?"));
        }
        if let Some(d) = manifest.get("description") {
            eprintln!("Description: {}", d.as_str().unwrap_or("?"));
        }
        if let Some(kt) = manifest.get("kata_type") {
            eprintln!("Type: {}", kt.as_str().unwrap_or("?"));
        }
    }

    if let Some(gas) = parsed.get("gas") {
        if let Some(cap) = gas.get("cap") {
            eprintln!("Gas cap: {}", cap.as_u64().unwrap_or(0));
        }
    }

    if let Some(steps) = parsed.get("steps") {
        if let Some(arr) = steps.as_sequence() {
            eprintln!("Steps: {}", arr.len());
            for (i, step) in arr.iter().enumerate() {
                let action = step.get("action").and_then(|a| a.as_str()).unwrap_or("?");
                let desc = step
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("?");
                eprintln!("  {}. {} — {}", i + 1, action, desc);
            }
        }
    }

    if let Some(practices) = parsed.get("practices") {
        if let Some(arr) = practices.as_sequence() {
            eprintln!("Practices: {}", arr.len());
            for p in arr {
                if let Some(n) = p.get("name").and_then(|v| v.as_str()) {
                    eprintln!("  - {}", n);
                }
            }
        }
    }

    if let Some(cns) = parsed.get("cns") {
        if let Some(ns) = cns.get("span_namespace").and_then(|v| v.as_str()) {
            eprintln!("CNS namespace: {}", ns);
        }
    }
}
