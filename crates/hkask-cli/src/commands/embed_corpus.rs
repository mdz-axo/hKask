//! Style corpus embedding command — thin CLI orchestrator

use crate::cli::EmbedCorpusAction;
use hkask_services::{EmbedProgress, EmbedService};

use std::path::PathBuf;
use std::sync::Arc;

pub fn run(rt: &tokio::runtime::Runtime, action: EmbedCorpusAction) {
    match action {
        EmbedCorpusAction::Run {
            config,
            db,
            passphrase,
            okapi_url,
        } => run_embed(rt, config, db, passphrase, okapi_url),
    }
}

fn run_embed(
    rt: &tokio::runtime::Runtime,
    config_path: PathBuf,
    db_path: PathBuf,
    passphrase: String,
    okapi_url: Option<String>,
) {
    let progress: hkask_services::ProgressFn = Arc::new(|p: &EmbedProgress| {
        eprint!("\r\x1b[K{}", p.format_full());
    });

    let result = rt.block_on(EmbedService::embed_corpus(
        &config_path,
        &db_path.to_string_lossy(),
        &passphrase,
        okapi_url.as_deref(),
        None,
        Some(progress),
    ));

    // Clear the progress line
    eprint!("\r\x1b[K");

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
                "Validation config: centroid_distance_max={}, exemplar_count={}..{}",
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
