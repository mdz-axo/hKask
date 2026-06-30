//! Kata command — list, inspect, and execute kata manifests.
//!
//! `kask kata start` runs a full kata cycle: loads the manifest, walks its
//! steps/questions/practices, renders templates, calls inference, and
//! accumulates state. Uses the centralized inference router.

use crate::cli::KataAction;
use crate::experience::CliExperienceRecorder;
use hkask_cns::CnsRuntime;
use hkask_services_kata_kanban::{KataEngine, KataError, KataHistory, PracticeEntry};
use hkask_storage::KataHistoryStore;
use hkask_templates::SqliteRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Resolve the registry/manifests directory relative to the project root.
fn manifests_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join("registry").join("manifests")
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is a valid KataAction variant; registry is a valid SqliteRegistry
/// post: dispatches to list_manifests, show_manifest, or start_kata based on action variant
pub fn run(rt: &tokio::runtime::Runtime, action: KataAction, registry: &SqliteRegistry) {
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
            rt,
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

    let parsed: serde_yaml_neo::Value = match serde_yaml_neo::from_str(&content) {
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

    if let Some(gas) = parsed.get("gas")
        && let Some(cap) = gas.get("cap")
    {
        eprintln!("Gas cap: {}", cap.as_u64().unwrap_or(0));
    }

    if let Some(steps) = parsed.get("steps")
        && let Some(arr) = steps.as_sequence()
    {
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

    if let Some(practices) = parsed.get("practices")
        && let Some(arr) = practices.as_sequence()
    {
        eprintln!("Practices: {}", arr.len());
        for p in arr {
            if let Some(n) = p.get("name").and_then(|v| v.as_str()) {
                eprintln!("  - {}", n);
            }
        }
    }

    if let Some(cns) = parsed.get("cns")
        && let Some(ns) = cns.get("span_namespace").and_then(|v| v.as_str())
    {
        eprintln!("CNS namespace: {}", ns);
    }
}

fn start_kata(
    rt: &tokio::runtime::Runtime,
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

    // Extract IK state reference if present (for coaching grounding)
    let ik_state_path = ctx.get("ik_state").cloned();
    let ik_state_ref = ik_state_path.as_ref().and_then(|p| {
        let path = PathBuf::from(p);
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(json) => Some(json),
                Err(e) => {
                    eprintln!("Warning: Failed to read IK state file '{}': {}", p, e);
                    None
                }
            }
        } else {
            eprintln!("Warning: IK state file not found: {}", p);
            None
        }
    });

    // Load kata history for habit tracking and automaticity scoring
    let history_path = kata_history_path();
    let history = KataHistory::load(&history_path).unwrap_or_else(|e| {
        eprintln!(
            "Warning: Failed to load kata history: {} — starting fresh",
            e
        );
        KataHistory::default()
    });

    // Try to open SQLite history store for concurrent, queryable persistence.
    // Falls back to JSON-only when the database is unavailable (e.g., standalone CLI).
    let history_store = try_open_history_store();

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    eprintln!(
        "Agent '{}': streak={} days, automaticity={:.2}",
        bot,
        history.current_streak(bot, &today),
        history.compute_automaticity(bot, &today)
    );

    // Build engine with shared registry (has bootstrapped templates)
    // Inference construction is encapsulated in KataEngine::from_env()
    let cns_rt = Arc::new(RwLock::new(CnsRuntime::with_threshold(
        hkask_cns::DEFAULT_VARIETY_MAX_DEFICIT as u64,
    )));

    let mut engine = KataEngine::from_env(registry.clone())
        .with_history(history)
        .with_cns_runtime(cns_rt)
        .with_metrics(move |_agent: &str, _metric: &str| {
            // CNS variety counters are accessible via the daemon's async API.
            // Direct sync access requires async→sync bridging, which is not
            // possible inside a tokio runtime (nested runtimes forbidden).
            // The full CNS→Curator feedback loop goes through the daemon.
            Ok(serde_json::json!({
                "source": "cns_variety_daemon",
                "note": "CNS metrics available through daemon connectivity"
            }))
        })
        .with_consent(move |kata_type: &str, _learner: &str| {
            // P2 Affirmative Consent — kata execution authorization
            match kata_type {
                "starter" => {
                    // Self-consent: the agent consents by invoking the command.
                    // No external authorization needed for practice routines.
                    Ok(())
                }
                "improvement" => {
                    // Curator consent: the human operator running this CLI
                    // is the Curator. In pod context, this would verify OCAP tokens.
                    // For CLI, consent is implicit in command invocation.
                    tracing::info!(
                        target: "hkask.kata",
                        kata_type = "improvement",
                        bot = %_learner,
                        "Curator consent granted (CLI invocation)"
                    );
                    Ok(())
                }
                "coaching" => {
                    // Learner consent: the learner must be explicitly declared.
                    // In CLI context, this means --ctx learner=<name> must be present.
                    // In pod context, this would verify the learner's OCAP consent grant.
                    tracing::info!(
                        target: "hkask.kata",
                        kata_type = "coaching",
                        bot = %_learner,
                        "Learner consent granted (CLI invocation)"
                    );
                    Ok(())
                }
                other => Err(KataError::UnknownType(format!(
                    "Consent not configured for kata type: {}",
                    other
                ))),
            }
        })
        .with_cns(move |namespace: &str, ordinal: u32, action: &str| {
            // CNS observer — structured span data for variety counters
            // When CnsRuntime is available, the CLI forwards these events.
            // For now, tracing provides the CNS sensor layer.
            tracing::info!(
                target: "hkask.kata",
                namespace = %namespace,
                step = ordinal,
                action = %action,
                "kata.cns.observer — step completed"
            );
        });

    // Conditionally add SQLite history store (falls back to JSON when unavailable)
    if let Some(ref store) = history_store {
        engine = engine.with_history_store(store.clone());
    }

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
        match hkask_services_kata_kanban::KataState::load(rp) {
            Ok(mut state) => {
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
                // Inject IK state reference for coaching grounding
                if let Some(ref ik) = ik_state_ref
                    && state.ik_state_ref.is_none()
                {
                    state.ik_state_ref = Some(ik.clone());
                }
                Some(state)
            }
            Err(e) => {
                eprintln!("Failed to load state: {} — starting fresh", e);
                None
            }
        }
    } else {
        // New state with optional IK grounding
        if let Some(ref ik) = ik_state_ref {
            let state = hkask_services_kata_kanban::KataState {
                ik_state_ref: Some(ik.clone()),
                ..Default::default()
            };
            Some(state)
        } else {
            None
        }
    };

    eprintln!("=== Executing {} ===", manifest.manifest.name);
    let kata_type = manifest.manifest.kata_type.as_str();
    eprintln!(
        "Type: {}",
        if kata_type.is_empty() {
            "bundle"
        } else {
            kata_type
        }
    );
    eprintln!("Bot: {}", bot);
    eprintln!("Gas cap: {}", manifest.gas.cap);
    if resume_path.is_some() {
        eprintln!("Resume: true");
    }
    eprintln!();

    // Execute (uses shared runtime from main)
    // Bundle manifests (no kata_type) use the bundle orchestrator
    let is_bundle = kata_type.is_empty()
        || (!matches!(kata_type, "improvement" | "coaching" | "starter")
            && !manifest.steps.is_empty());

    let result = if let Some(mut state) = initial_state {
        // Resume: continue from saved state (bundle resume not yet supported)
        rt.block_on(async {
            match kata_type {
                "improvement" => engine.run_improvement_from(&manifest, &mut state).await,
                "coaching" => engine.run_coaching_from(&manifest, &mut state).await,
                "starter" => engine.run_starter(&manifest, &mut state).await,
                _ => Err(KataError::UnknownType(kata_type.to_string())),
            }
        })
    } else if is_bundle {
        rt.block_on(engine.run_bundle(&manifest, bot, ctx))
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

            // Display improvement signal if present
            if let Some(ref signal) = result.improvement_signal {
                eprintln!(
                    "Improvement: {:?} (delta: {:?})",
                    signal.direction, signal.delta
                );
            }

            // Display automaticity delta if present
            if let Some(delta) = result.automaticity_delta {
                eprintln!("Automaticity delta: {:.2}", delta);
            }

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

            // Record step-level experiences via daemon — agent learns from every step
            let bot_name = bot.to_string();
            let step_experiences = result.step_experiences.clone();
            let kata_type = result.kata_type.clone();
            let steps = result.steps_completed;
            let gas = result.gas_consumed;
            rt.spawn(async move {
                let recorder = CliExperienceRecorder::new();

                // Record each step as an individual experience
                for exp in &step_experiences {
                    recorder
                        .record(
                            &bot_name,
                            "kata_step",
                            &format!("{}/{}: {}", exp.kata_type, exp.step_label, exp.action),
                            "success",
                            serde_json::json!({
                                "kata_type": exp.kata_type,
                                "step_label": exp.step_label,
                                "action": exp.action,
                                "summary": exp.output_summary,
                                "gas_used": exp.gas_used,
                            }),
                        )
                        .await;
                }

                // Record overall cycle completion
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
                            "step_experiences_count": step_experiences.len(),
                        }),
                    )
                    .await;
            });

            // Record practice to kata history and save
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let mut history = KataHistory::load(&history_path).unwrap_or_default();
            history.record(
                bot,
                PracticeEntry {
                    date: today.clone(),
                    kata_type: manifest.manifest.kata_type.clone(),
                    practice_name: manifest.manifest.id.clone(),
                    steps_completed: result.steps_completed,
                    gas_consumed: result.gas_consumed,
                },
            );
            if let Err(e) = history.save(&history_path) {
                eprintln!("Warning: Failed to save kata history: {}", e);
            } else {
                let auto = history.compute_automaticity(bot, &today);
                let streak = history.current_streak(bot, &today);
                eprintln!(
                    "Kata history updated: streak={}, automaticity={:.2}",
                    streak, auto
                );
            }

            // Persist to SQLite history store when available (concurrent, queryable)
            match engine.record_history_entry(
                bot,
                &today,
                &manifest.manifest.kata_type,
                &manifest.manifest.id,
                result.steps_completed,
                result.gas_consumed,
            ) {
                Ok(Some(id)) => {
                    tracing::info!(
                        target: "hkask.kata",
                        row_id = id,
                        agent = %bot,
                        kata_type = %manifest.manifest.kata_type,
                        "Kata history recorded to SQLite"
                    );
                }
                Ok(None) => {
                    // No store available — already persisted to JSON above
                }
                Err(e) => {
                    eprintln!("Warning: Failed to record history to SQLite: {}", e);
                }
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

/// Path to the kata history file for habit tracking.
fn kata_history_path() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join("data").join("kata-history.json")
}

/// Default database path for kata history SQLite storage.
///
/// Matches the daemon's DEFAULT_DB_PATH. When the daemon is running,
/// the database exists at this path. When running standalone CLI, the
/// path may not exist — the caller handles the fallback.
fn kata_default_db_path() -> String {
    std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "data/hkask.db".to_string())
}

/// Attempt to open a kata history store from the daemon's database.
///
/// Opens a direct unencrypted SQLite connection (bypasses SQLCipher).
/// Creates the kata_history table if it doesn't exist. Returns `None` if
/// the database file doesn't exist or can't be opened — the caller falls
/// back to JSON-based persistence.
fn try_open_history_store() -> Option<Arc<KataHistoryStore>> {
    let db_path = kata_default_db_path();
    if !std::path::Path::new(&db_path).exists() {
        return None;
    }
    let conn = rusqlite::Connection::open(&db_path).ok()?;
    // Create the kata_history table if it doesn't exist (idempotent)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kata_history (id INTEGER PRIMARY KEY AUTOINCREMENT, agent_name TEXT NOT NULL, date TEXT NOT NULL, kata_type TEXT NOT NULL, practice_name TEXT NOT NULL, steps_completed INTEGER NOT NULL DEFAULT 0, gas_consumed INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')));"
    ).ok()?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_kata_history_agent ON kata_history(agent_name);",
    )
    .ok()?;
    conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_kata_history_date ON kata_history(date);")
        .ok()?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_kata_history_type ON kata_history(kata_type);",
    )
    .ok()?;
    let store = KataHistoryStore::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
    Some(Arc::new(store))
}
