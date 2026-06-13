//! Style corpus embedding command — thin CLI orchestrator

use hkask_services::{EmbedProgress, EmbedService};
use hkask_storage::user_store::UserStore;

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Authenticate a replicant and resolve the agent-specific DB path.
/// Returns (db_path, db_passphrase) after successful login.
fn resolve_replicant_db(replicant: &str, passphrase: &str) -> Result<(String, String), String> {
    let config = hkask_services::ServiceConfig::from_env()
        .map_err(|e| format!("Failed to resolve config: {e}"))?;
    let db = hkask_storage::Database::open(&config.db_path, &config.db_passphrase)
        .map_err(|e| format!("Failed to open system DB: {e}"))?;
    let store = Arc::new(Mutex::new(UserStore::new(db.conn_arc())));

    // Authenticate
    let session = store
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?
        .login(replicant, passphrase)
        .map_err(|_| format!("Authentication failed for replicant '{}'", replicant))?;

    eprintln!(
        "Authenticated as {} (session: {})",
        replicant, session.session_id
    );

    // Agent-specific DB path: ~/.config/hkask/agents/<replicant>.db
    let agent_db_path = hkask_services::settings_path()
        .parent()
        .unwrap()
        .join("agents")
        .join(format!("{}.db", replicant));
    let _ = std::fs::create_dir_all(agent_db_path.parent().unwrap());

    Ok((
        agent_db_path.to_string_lossy().to_string(),
        passphrase.to_string(),
    ))
}

pub fn run(
    rt: &tokio::runtime::Runtime,
    config: PathBuf,
    replicant: String,
    passphrase: String,
    db: Option<PathBuf>,
) {
    let (db_path, db_passphrase) = if let Some(ref manual_db) = db {
        // Manual DB override — skip auth, use as-is
        (manual_db.to_string_lossy().to_string(), passphrase)
    } else {
        match resolve_replicant_db(&replicant, &passphrase) {
            Ok((path, phrase)) => (path, phrase),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    };

    let progress: hkask_services::ProgressFn = Arc::new(|p: &EmbedProgress| {
        eprint!("\r\x1b[K{}", p.format_full());
        let _ = std::io::stderr().flush();
    });

    eprintln!("=== Embedding corpus ===");
    eprintln!("Config: {}", config.display());
    eprintln!("DB: {}", db_path);

    let result = rt.block_on(EmbedService::embed_corpus(
        &config,
        &db_path,
        &db_passphrase,
        None,
        Some(progress),
    ));

    // Clear the progress line
    eprint!("\r\x1b[K");
    let _ = std::io::stderr().flush();

    match result {
        Ok(r) => {
            eprintln!("Corpus: {} ({} embeddings)", r.author, r.total_passages);
            if r.purged > 0 {
                eprintln!(
                    "Purged {} existing embeddings for {} (idempotent re-run)",
                    r.purged, r.author
                );
            }
            eprintln!("Total passages embedded: {}", r.total_passages);
            eprintln!(
                "Budget: {} triples → {} passages tagged ({} embedding-only)",
                r.budget, r.tagged_passages, r.embedding_only
            );
            eprintln!(
                "Triples stored: {} / {} ({:.1}% utilized)",
                r.triples_stored,
                r.budget,
                if r.budget > 0 {
                    (r.triples_stored as f64 / r.budget as f64) * 100.0
                } else {
                    0.0
                }
            );
            eprintln!("Done. Centroid stored as: {}", r.centroid_ref);
            eprintln!(
                "Centroid computed from {} prose passages (stored: {})",
                r.passage_count, r.centroid_stored
            );
            eprintln!(
                "Validation config: centroid_distance_max={}, exemplar_count_min={}, exemplar_count_max={}",
                r.validation.centroid_distance_max,
                r.validation.exemplar_count_min,
                r.validation.exemplar_count_max,
            );
        }
        Err(e) => {
            eprintln!("Embedding failed: {e}");
            std::process::exit(1);
        }
    }
}
