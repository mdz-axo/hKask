//! Style corpus embedding command — thin CLI orchestrator

use hkask_services_corpus::{EmbedProgress, EmbedService};

use crate::experience::CliExperienceRecorder;

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

/// Authenticate a replicant and resolve the agent-specific DB path.
/// Returns (db_path, db_passphrase) after successful login.
fn resolve_replicant_db(replicant: &str, passphrase: &str) -> Result<(String, String), String> {
    let ctx = crate::commands::helpers::build_agent_service();
    let store = ctx.storage().users.clone();

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
    let agent_db_path = hkask_services_core::settings_path()
        .parent()
        .expect("embed corpus path")
        .join("agents")
        .join(format!("{}.db", replicant));
    let _ = std::fs::create_dir_all(agent_db_path.parent().expect("embed corpus path"));

    Ok((
        agent_db_path.to_string_lossy().to_string(),
        passphrase.to_string(),
    ))
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; config is a valid corpus.yaml path; replicant is non-empty; passphrase is valid
/// post: embeds corpus passages into the vector database; prints embedding stats, centroid info, and validation config
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

    let progress: hkask_services_corpus::ProgressFn = Arc::new(|p: &EmbedProgress| {
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
            if !r.dimension_centroids.is_empty() {
                eprintln!("Dimension centroids:");
                for dc in &r.dimension_centroids {
                    eprintln!(
                        "  {} → {} ({} passages)",
                        dc.name, dc.ref_name, dc.passage_count
                    );
                }
            }
            eprintln!(
                "Validation config: centroid_distance_max={}, exemplar_count_min={}, exemplar_count_max={}",
                r.validation.centroid_distance_max,
                r.validation.exemplar_count_min,
                r.validation.exemplar_count_max,
            );

            // Record experience via daemon
            let recorder = CliExperienceRecorder::new();
            let dim_centroids_json: Vec<serde_json::Value> = r
                .dimension_centroids
                .iter()
                .map(|dc| {
                    serde_json::json!({
                        "name": dc.name,
                        "ref_name": dc.ref_name,
                        "passage_count": dc.passage_count,
                    })
                })
                .collect();
            rt.spawn(async move {
                recorder
                    .record(
                        &replicant,
                        "embed_corpus",
                        &r.author,
                        "success",
                        serde_json::json!({
                            "author": r.author,
                            "total_passages": r.total_passages,
                            "tagged_passages": r.tagged_passages,
                            "triples_stored": r.triples_stored,
                            "budget": r.budget,
                            "centroid_ref": r.centroid_ref,
                            "centroid_stored": r.centroid_stored,
                            "dimension_centroids": dim_centroids_json,
                        }),
                    )
                    .await;
            });
        }
        Err(e) => {
            eprintln!("Embedding failed: {e}");
            std::process::exit(1);
        }
    }
}
