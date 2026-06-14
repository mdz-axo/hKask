//! Kata command — list, inspect, and execute kata manifests.
//!
//! `kask kata start` runs a full kata cycle: loads the manifest, walks its
//! steps/questions/practices, renders templates, calls inference, and
//! accumulates state. Uses the centralized inference router.

use crate::cli::KataAction;
use hkask_inference::InferenceConfig;
use hkask_services::{CliExperienceRecorder, KataEngine, KataError};
use hkask_templates::SqliteRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolve the registry/manifests directory relative to the project root.
fn manifests_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join("registry").join("manifests")
}

pub fn run(action: KataAction, registry: &SqliteRegistry) {
    match action {
        KataAction::List => list_manifests(),
        KataAction::Show { name } => show_manifest(&name),
        KataAction::Start {
            name,
            bot,
            context,
            save,
            resume,
        } => start_kata(
            &name,
            &bot,
            &context,
            save.as_deref(),
            resume.as_deref(),
            registry,
        ),
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

fn start_kata(
    name: &str,
    bot: &str,
    context: &[String],
    save_path: Option<&Path>,
    resume_path: Option<&Path>,
    registry: &SqliteRegistry,
) {
    let dir = manifests_dir();
    let path = dir.join(format!("{}.yaml", name));

    // Parse context key=value pairs
    let mut ctx = HashMap::new();
    for pair in context {
        if let Some((k, v)) = pair.split_once('=') {
            ctx.insert(k.to_string(), v.to_string());
        }
    }

    // Build engine with shared registry (has bootstrapped templates)
    let inf_cfg = InferenceConfig::from_env();
    let inference = hkask_inference::InferenceRouter::new(inf_cfg);
    let inference_port: std::sync::Arc<dyn hkask_types::ports::InferencePort> =
        std::sync::Arc::new(inference);

    // Clone the registry's connection for the engine
    let engine = KataEngine::new(inference_port, registry.clone());

    // Load manifest
    let manifest = match KataEngine::load_manifest(&path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load manifest '{}': {}", name, e);
            std::process::exit(1);
        }
    };

    // Resume from saved state if provided
    let initial_state = if let Some(rp) = resume_path {
        match hkask_services::KataState::load(rp) {
            Ok(state) => {
                eprintln!(
                    "Resumed state from {} (step {}/{})",
                    rp.display(),
                    state.current_step,
                    if manifest.manifest.kata_type == "improvement" {
                        manifest.steps.len()
                    } else {
                        manifest.questions.len()
                    }
                );
                Some(state)
            }
            Err(e) => {
                eprintln!("Failed to load state: {} — starting fresh", e);
                None
            }
        }
    } else {
        None
    };

    eprintln!("=== Executing {} ===", manifest.manifest.name);
    eprintln!("Type: {}", manifest.manifest.kata_type);
    eprintln!("Bot: {}", bot);
    eprintln!("Gas cap: {}", manifest.gas.cap);
    if resume_path.is_some() {
        eprintln!("Resume: true");
    }
    eprintln!();

    // Execute
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let result = if let Some(mut state) = initial_state {
        // Resume: continue from saved state
        rt.block_on(async {
            match manifest.manifest.kata_type.as_str() {
                "improvement" => engine.run_improvement_from(&manifest, &mut state).await,
                "coaching" => engine.run_coaching_from(&manifest, &mut state).await,
                "starter" => engine.run_starter(&manifest, &mut state).await,
                other => Err(KataError::UnknownType(other.to_string())),
            }
        })
    } else {
        rt.block_on(engine.execute(&manifest, bot, ctx))
    };

    match result {
        Ok(mut result) => {
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

            // Save state if requested
            if let Some(sp) = save_path {
                result.state.manifest_id = manifest.manifest.id.clone();
                match result.state.save(sp) {
                    Ok(()) => eprintln!("State saved to {}", sp.display()),
                    Err(e) => eprintln!("Failed to save state: {}", e),
                }
            }

            eprintln!();
            eprintln!("Step outputs:");
            for (key, value) in &result.state.step_outputs {
                let display =
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
                eprintln!("  [{}]: {}", key, display);
            }

            // Record experience via daemon — agent learns from kata
            let recorder = CliExperienceRecorder::new();
            let kata_type = result.kata_type.clone();
            let steps = result.steps_completed;
            let gas = result.gas_consumed;
            let bot_name = bot.to_string();
            rt.spawn(async move {
                recorder
                    .record(
                        &bot_name,
                        "kata_execute",
                        &kata_type,
                        "success",
                        serde_json::json!({
                            "kata_type": kata_type,
                            "steps_completed": steps,
                            "gas_consumed": gas,
                        }),
                    )
                    .await;
            });
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
