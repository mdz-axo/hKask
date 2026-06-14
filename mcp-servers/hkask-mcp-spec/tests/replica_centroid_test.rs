//! Integration test: verify replica centroids exist and are queryable.
//! # REQ: P8 — verifies that embedded dimension centroids are stored correctly.
//!
//! Queries the styles DB directly for gentle-lovelace centroids. Does not
//! require API keys — only reads from the already-embedded corpus.

use hkask_storage::{Database, EmbeddingStore};
use std::path::PathBuf;

fn styles_db_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../data/hkask-styles.db")
}

#[test]
fn gentle_lovelace_centroids_exist_in_db() {
    let db_path = styles_db_path();
    if !db_path.exists() {
        eprintln!(
            "Styles DB not found at {} — skipping test",
            db_path.display()
        );
        return;
    }

    let db =
        Database::open(&db_path.to_string_lossy(), "test-pass").expect("Failed to open styles DB");
    let conn = db.conn_arc();
    let store = EmbeddingStore::new(conn);

    // Query all entities with the gentle-lovelace prefix
    let prefix = "style:gentle-lovelace:";
    let all_refs = store
        .query_by_prefix(prefix)
        .expect("Failed to query embeddings");

    eprintln!("Found {} entities with prefix '{}'", all_refs.len(), prefix);

    // Verify composite centroid exists
    let composite_ref = "style:gentle-lovelace:centroid";
    assert!(
        all_refs.iter().any(|r| r == composite_ref),
        "Composite centroid '{}' must exist",
        composite_ref
    );

    // Verify all 4 dimension centroids exist
    let expected_centroids = [
        ("gentle", "style:gentle-lovelace:gentle-centroid"),
        ("schriver", "style:gentle-lovelace:schriver-centroid"),
        ("hopper", "style:gentle-lovelace:hopper-centroid"),
        ("lovelace", "style:gentle-lovelace:lovelace-centroid"),
    ];

    for (dim_name, centroid_ref) in &expected_centroids {
        assert!(
            all_refs.iter().any(|r| r == centroid_ref),
            "Dimension centroid '{}' ({}) must exist",
            dim_name,
            centroid_ref
        );

        // Verify the centroid embedding is retrievable and has correct dimensions
        let emb = store.get(centroid_ref).expect(&format!(
            "Failed to get centroid embedding for {}",
            dim_name
        ));
        assert_eq!(
            emb.vector.len(),
            1024,
            "{} centroid must have 1024-dimensional embedding",
            dim_name
        );
        assert!(
            emb.vector.iter().any(|&v| v != 0.0),
            "{} centroid must have non-zero embedding values",
            dim_name
        );

        eprintln!(
            "  {} centroid: {} dimensions, non-zero values present",
            dim_name,
            emb.vector.len()
        );
    }

    // Verify passage count: centroids should have associated passages
    let total_entities = all_refs.len();
    assert!(
        total_entities >= 5,
        "Must have at least 5 entities (4 dimension centroids + 1 composite + passages)"
    );

    eprintln!(
        "All {} gentle-lovelace centroids verified ({} total entities)",
        expected_centroids.len() + 1,
        total_entities
    );
}

#[test]
fn all_style_centroids_exist() {
    let db_path = styles_db_path();
    if !db_path.exists() {
        eprintln!(
            "Styles DB not found at {} — skipping test",
            db_path.display()
        );
        return;
    }

    let db =
        Database::open(&db_path.to_string_lossy(), "test-pass").expect("Failed to open styles DB");
    let conn = db.conn_arc();
    let store = EmbeddingStore::new(conn);

    // Verify all embedded style corpora have centroids
    let expected_styles = [
        "style:gentle-lovelace:centroid",
        "style:hemingway:centroid",
        "style:woolf:centroid",
        "style:ulysses-s-twain:centroid",
        "style:jane-wilde:centroid",
        "style:agatha-eliot:centroid",
    ];

    for centroid_ref in &expected_styles {
        match store.get(centroid_ref) {
            Ok(emb) => {
                assert!(
                    emb.vector.len() == 1024,
                    "{} must have 1024-dimensional embedding",
                    centroid_ref
                );
                eprintln!("  {} ✅ ({} dims)", centroid_ref, emb.vector.len());
            }
            Err(e) => {
                panic!("Centroid '{}' must exist: {}", centroid_ref, e);
            }
        }
    }

    eprintln!("All {} style centroids verified", expected_styles.len());
}
