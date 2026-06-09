//! Style corpus embedding command — thin CLI orchestrator

use crate::cli::EmbedCorpusAction;
use hkask_services::EmbedService;

use std::path::PathBuf;

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
    let result = rt.block_on(EmbedService::embed_corpus(
        &config_path,
        &db_path.to_string_lossy(),
        &passphrase,
        okapi_url.as_deref(),
        None,
    ));

    match result {
        Ok(r) => {
            eprintln!("Corpus: {} ({}d embeddings)", r.author, r.total_passages);
            if r.purged > 0 {
                eprintln!(
                    "Purged {} existing embeddings for {} (idempotent re-run)",
                    r.purged, r.author
                );
            }
            eprintln!("Total passages embedded: {}", r.total_passages);
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
