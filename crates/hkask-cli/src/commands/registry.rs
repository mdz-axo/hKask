//! Registry management commands — `kask list` and `kask rm`.
//!
//! General artifact management across registries (styles, bots, templates, etc.).
//! `kask list <registry>` lists artifacts. `kask rm <registry>-<artifact>` removes.

use hkask_memory::SemanticMemory;
use hkask_ports::RegistryIndex;
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_templates::SqliteRegistry;

use std::sync::Arc;

/// List artifacts in a registry.
///
/// Supported registries:
/// - `styles` — lists all built style corpora (queries `style:*:centroid` in EmbeddingStore)
/// - `templates` — lists all registered templates
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; template_registry is a valid SqliteRegistry; registry is "styles" or "templates"
/// post: lists artifacts in the specified registry; prints results or error for unknown registries
pub fn run_list(template_registry: &SqliteRegistry, registry: String) {
    match registry.as_str() {
        "styles" => list_styles(),
        "templates" => list_templates(template_registry),
        other => {
            eprintln!("Unknown registry: '{other}'");
            eprintln!("Supported: styles, templates");
            std::process::exit(1);
        }
    }
}

/// Remove an artifact from a registry.
///
/// Format: `<registry>-<artifact>` (hyphen-separated).
/// Example: `styles-hemingway` removes the Hemingway style corpus.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; template_registry is a mutable SqliteRegistry; target is "registry-artifact" format
/// post: removes the specified artifact from the registry; purges embeddings, triples, and disk artifacts
pub fn run_rm(
    template_registry: &mut SqliteRegistry,
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
        "styles" => remove_style(artifact, db_path, passphrase),
        "templates" => remove_template(template_registry, artifact),
        other => {
            eprintln!("Unknown registry: '{other}'");
            eprintln!("Supported: styles, templates");
            std::process::exit(1);
        }
    }
}

// ── Styles registry ──────────────────────────────────────────────────────────

fn list_styles() {
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

fn remove_style(artifact: &str, db_path: Option<String>, passphrase: Option<String>) {
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
    let semantic = SemanticMemory::new(
        TripleStore::new(Arc::clone(&conn)),
        EmbeddingStore::new(Arc::clone(&conn)),
    );

    let prefix = format!("style:{}:", artifact);

    // Purge embeddings
    let purged = crate::commands::helpers::or_exit(
        semantic.purge_by_prefix(&prefix),
        "Failed to purge embeddings",
    );

    // Purge triples
    let triples_purged = crate::commands::helpers::or_exit(
        triple_store.delete_by_entity_prefix(&prefix),
        "Failed to purge triples",
    );

    // Check if anything actually exists before claiming removal
    let corpus_dir = std::path::PathBuf::from(format!("./{}", artifact));
    let yaml_path = corpus_dir.join("corpus.yaml");
    let has_disk_artifacts = corpus_dir.exists() || yaml_path.exists();

    if purged == 0 && triples_purged == 0 && !has_disk_artifacts {
        eprintln!("No style corpus '{}' found.", artifact);
        return;
    }

    eprintln!(
        "Purged {} embeddings and {} triples with prefix '{}'",
        purged, triples_purged, prefix
    );

    // Remove cache directory and corpus YAML from disk
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

// ── Templates registry ──────────────────────────────────────────────────────

fn list_templates(registry: &SqliteRegistry) {
    let entries = registry.list(None);
    if entries.is_empty() {
        eprintln!("No templates registered.");
        return;
    }
    eprintln!("Templates:");
    for entry in &entries {
        eprintln!(
            "  {} — {} ({})",
            entry.id,
            if entry.name.is_empty() {
                "(unnamed)"
            } else {
                &entry.name
            },
            entry.template_type.as_str()
        );
    }
    eprintln!("{} template(s) total.", entries.len());
}

fn remove_template(registry: &mut SqliteRegistry, artifact: &str) {
    match registry.delete_entry(artifact) {
        Some(entry) => {
            eprintln!(
                "Removed template '{}' ({})",
                entry.id,
                if entry.name.is_empty() {
                    "(unnamed)"
                } else {
                    &entry.name
                }
            );
        }
        None => {
            eprintln!("No template '{}' found.", artifact);
        }
    }
}
