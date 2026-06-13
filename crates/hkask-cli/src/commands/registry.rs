//! Registry management commands — `kask list` and `kask rm`.
//!
//! General artifact management across registries (styles, bots, templates, etc.).
//! `kask list <registry>` lists artifacts. `kask rm <registry>-<artifact>` removes.

use hkask_memory::SemanticMemory;
use hkask_storage::{Database, EmbeddingStore, TripleStore};

use std::sync::Arc;

/// List artifacts in a registry.
///
/// Supported registries:
/// - `styles` — lists all built style corpora (queries `style:*:centroid` in EmbeddingStore)
pub fn run_list(rt: &tokio::runtime::Runtime, registry: String) {
    match registry.as_str() {
        "styles" => list_styles(rt),
        other => {
            eprintln!("Unknown registry: '{other}'");
            eprintln!("Supported: styles");
            std::process::exit(1);
        }
    }
}

/// Remove an artifact from a registry.
///
/// Format: `<registry>-<artifact>` (hyphen-separated).
/// Example: `styles-hemingway` removes the Hemingway style corpus.
pub fn run_rm(
    rt: &tokio::runtime::Runtime,
    target: String,
    db_path: Option<String>,
    passphrase: Option<String>,
) {
    // Parse "registry-artifact" format
    let hyphen_pos = target.find('-').unwrap_or_else(|| {
        eprintln!(
            "Error: target must be in format '<registry>-<artifact>' (e.g., 'styles-hemingway')"
        );
        std::process::exit(1);
    });

    let registry = &target[..hyphen_pos];
    let artifact = &target[hyphen_pos + 1..];

    if artifact.is_empty() {
        eprintln!("Error: artifact name is empty after hyphen in '{}'", target);
        std::process::exit(1);
    }

    match registry {
        "styles" => remove_style(rt, artifact, db_path, passphrase),
        other => {
            eprintln!("Unknown registry: '{other}'");
            eprintln!("Supported: styles");
            std::process::exit(1);
        }
    }
}

// ── Styles registry ──────────────────────────────────────────────────────────

fn list_styles(_rt: &tokio::runtime::Runtime) {
    let config = crate::commands::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );

    let db = crate::commands::helpers::or_exit(
        Database::open(&config.db_path, &config.db_passphrase),
        "Failed to open database",
    );

    let conn = db.conn_arc();
    let embedding_store = EmbeddingStore::new(Arc::clone(&conn));

    // Query all style-prefixed embeddings, then filter for centroids
    let all_refs = crate::commands::helpers::or_exit(
        embedding_store.query_by_prefix("style:"),
        "Failed to query embeddings",
    );

    let refs: Vec<String> = all_refs
        .into_iter()
        .filter(|r| r.ends_with(":centroid"))
        .collect();

    if refs.is_empty() {
        eprintln!("No style corpora found.");
        return;
    }

    eprintln!("Style corpora:");
    for entity_ref in &refs {
        // Extract author name from "style:{author}:centroid"
        let author = entity_ref
            .strip_prefix("style:")
            .and_then(|s| s.strip_suffix(":centroid"))
            .unwrap_or(entity_ref);
        eprintln!("  {}", author);
    }
    eprintln!("{} corpus(es) total.", refs.len());
}

fn remove_style(
    _rt: &tokio::runtime::Runtime,
    artifact: &str,
    db_path: Option<String>,
    passphrase: Option<String>,
) {
    let config = crate::commands::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );

    let db_path = db_path.as_deref().unwrap_or(&config.db_path);
    let passphrase = passphrase.as_deref().unwrap_or(&config.db_passphrase);

    let db = crate::commands::helpers::or_exit(
        Database::open(db_path, passphrase),
        "Failed to open database",
    );

    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let embedding_store = EmbeddingStore::new(Arc::clone(&conn));
    let semantic = SemanticMemory::new(triple_store, embedding_store);

    let prefix = format!("style:{}:", artifact);

    // Purge embeddings
    let purged = crate::commands::helpers::or_exit(
        semantic.purge_by_prefix(&prefix),
        "Failed to purge embeddings",
    );

    eprintln!("Purged {} embeddings with prefix '{}'", purged, prefix);

    // Remove cache directory and corpus YAML from disk
    let corpus_dir = std::path::PathBuf::from(format!("./{}", artifact));
    if corpus_dir.exists() {
        let cache_dir = std::path::PathBuf::from("./.cache");
        // Remove cached content files for this author
        if cache_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&cache_dir)
        {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(artifact)
                    && name_str.ends_with(".txt")
                    && let Err(e) = std::fs::remove_file(entry.path())
                {
                    eprintln!(
                        "Warning: failed to remove cache file '{}': {e}",
                        entry.path().display()
                    );
                }
            }
        }

        // Remove corpus.yaml
        let yaml_path = corpus_dir.join("corpus.yaml");
        if yaml_path.exists() {
            if let Err(e) = std::fs::remove_file(&yaml_path) {
                eprintln!("Warning: failed to remove '{}': {e}", yaml_path.display());
            } else {
                eprintln!("Removed {}", yaml_path.display());
            }
        }

        // Remove directory if empty
        if let Ok(entries) = std::fs::read_dir(&corpus_dir)
            && entries.count() == 0
        {
            if let Err(e) = std::fs::remove_dir(&corpus_dir) {
                eprintln!(
                    "Warning: failed to remove directory '{}': {e}",
                    corpus_dir.display()
                );
            } else {
                eprintln!("Removed directory {}", corpus_dir.display());
            }
        }
    }

    eprintln!("Done. Style corpus '{}' removed.", artifact);
}
