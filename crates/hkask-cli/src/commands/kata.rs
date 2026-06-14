//! Kata command — list, inspect, and execute kata manifests.
//!
//! `kask kata start` runs a full kata cycle: loads the manifest, walks its
//! steps/questions/practices, renders templates, calls inference, and
//! accumulates state. Uses the centralized inference router.

use crate::cli::KataAction;
use hkask_inference::InferenceConfig;
use hkask_services::{KataEngine, KataError};
use hkask_templates::SqliteRegistry;
use std::collections::HashMap;
use std::path::PathBuf;

/// Resolve the registry/manifests directory relative to the project root.
fn manifests_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join("registry").join("manifests")
}

pub fn run(action: KataAction) {
    match action {
        KataAction::List => list_manifests(),
        KataAction::Show { name } => show_manifest(&name),
        KataAction::Start { name, bot, context } => start_kata(&name, &bot, &context),
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

    let parsed: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse manifest '{}': {}", name, e);
            std::process::exit(1);
        }
    };

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

fn start_kata(name: &str, bot: &str, context: &[String]) {
    let dir = manifests_dir();
    let path = dir.join(format!("{}.yaml", name));

    // Parse context key=value pairs
    let mut ctx = HashMap::new();
    for pair in context {
        if let Some((k, v)) = pair.split_once('=') {
            ctx.insert(k.to_string(), v.to_string());
        }
    }

    // Build engine
    let inf_cfg = InferenceConfig::from_env();
    let inference = hkask_inference::InferenceRouter::new(inf_cfg);
    let inference_port: std::sync::Arc<dyn hkask_types::ports::InferencePort> =
        std::sync::Arc::new(inference);

    let registry = crate::commands::helpers::or_exit(
        SqliteRegistry::new(None),
        "Failed to initialize registry",
    );
    let engine = KataEngine::new(inference_port, registry);

    // Load manifest
    let manifest = match KataEngine::load_manifest(&path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load manifest '{}': {}", name, e);
            std::process::exit(1);
        }
    };

    eprintln!("=== Executing {} ===", manifest.manifest.name);
    eprintln!("Type: {}", manifest.manifest.kata_type);
    eprintln!("Bot: {}", bot);
    eprintln!("Gas cap: {}", manifest.gas.cap);
    eprintln!();

    // Execute
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    match rt.block_on(engine.execute(&manifest, bot, ctx)) {
        Ok(result) => {
            eprintln!("=== Kata complete ===");
            eprintln!(
                "Steps completed: {}/{}",
                result.steps_completed, result.total_steps
            );
            eprintln!(
                "Gas consumed: {} / {} ({:.0}%)",
                result.gas_consumed,
                result.gas_cap,
                if result.gas_cap > 0 {
                    (result.gas_consumed as f64 / result.gas_cap as f64) * 100.0
                } else {
                    0.0
                }
            );
            eprintln!();
            eprintln!("Step outputs:");
            for (key, value) in &result.state.step_outputs {
                let display =
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
                eprintln!("  [{}]: {}", key, display);
            }
        }
        Err(KataError::GasExceeded { consumed, cap }) => {
            eprintln!(
                "Kata aborted: gas exceeded (consumed {}, cap {})",
                consumed, cap
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Kata failed: {}", e);
            std::process::exit(1);
        }
    }
}
