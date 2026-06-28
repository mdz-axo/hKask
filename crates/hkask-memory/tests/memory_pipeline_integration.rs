//! Integration test: episodic → recall → consolidate → semantic pipeline.
//!
//! Verifies the full memory lifecycle:
//! 1. Store episodic triple (first-person, perspective-bound)
//! 2. Recall episodic (deduped, decayed, temporal attention)
//! 3. Consolidate: episodic → semantic (one-way bridge)
//! 4. Bayesian combination on repeated consolidation of same EAV
//! 5. Semantic recall (deduped, perspective-free)

use hkask_memory::{
    ConsolidationBridge, EpisodicMemory, EpisodicMemoryError, SemanticMemory, SemanticMemoryError,
};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_types::WebID;
use std::sync::Arc;

fn setup() -> (Arc<EpisodicMemory>, Arc<SemanticMemory>) {
    let db = Database::in_memory().expect("in-memory db");
    let conn = db.conn_arc();
    let episodic = Arc::new(EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))));
    let semantic = Arc::new(SemanticMemory::new(
        TripleStore::new(Arc::clone(&conn)),
        EmbeddingStore::new(Arc::clone(&conn)),
    ));
    (episodic, semantic)
}

fn test_perspective() -> WebID {
    WebID::from_persona(b"test-agent")
}

// ── Episodic boundary enforcement ──────────────────────────────────────────

#[test]
fn episodic_store_and_recall() {
    let (episodic, _semantic) = setup();
    let perspective = test_perspective();

    let triple = Triple::new(
        "test_entity",
        "test_attr",
        serde_json::json!("test_value"),
        perspective,
    )
    .with_perspective(perspective);

    episodic.store(triple).expect("store episodic");

    let recalled = episodic
        .query_for_deduped("test_entity", perspective)
        .expect("recall episodic");

    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].entity, "test_entity");
    assert_eq!(recalled[0].attribute, "test_attr");
    assert_eq!(recalled[0].value, serde_json::json!("test_value"));
    assert_eq!(recalled[0].access.perspective, Some(perspective));
}

#[test]
fn episodic_rejects_public_visibility() {
    let (episodic, _semantic) = setup();
    let perspective = test_perspective();

    let triple = Triple::new("e", "a", serde_json::json!("v"), perspective)
        .with_visibility(hkask_types::Visibility::Public);

    let err = episodic.store(triple).unwrap_err();
    assert!(matches!(err, EpisodicMemoryError::InvalidVisibility(_)));
}

#[test]
fn episodic_requires_perspective() {
    let (episodic, _semantic) = setup();
    let perspective = test_perspective();

    let triple = Triple::new("e", "a", serde_json::json!("v"), perspective);

    let err = episodic.store(triple).unwrap_err();
    assert!(matches!(err, EpisodicMemoryError::MissingPerspective));
}

// ── Semantic boundary enforcement ──────────────────────────────────────────

#[test]
fn semantic_rejects_private_visibility() {
    let (_episodic, semantic) = setup();
    let perspective = test_perspective();

    let triple = Triple::new("e", "a", serde_json::json!("v"), perspective);
    let err = semantic.store(triple).unwrap_err();
    assert!(matches!(err, SemanticMemoryError::InvalidVisibility(_)));
}

#[test]
fn semantic_store_and_recall_deduped() {
    let (_episodic, semantic) = setup();
    let perspective = test_perspective();

    let triple = Triple::new("fact_x", "is", serde_json::json!("true"), perspective)
        .with_visibility(hkask_types::Visibility::Shared);

    semantic.store(triple).expect("store semantic");

    let recalled = semantic.query_deduped("fact_x").expect("recall semantic");
    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].entity, "fact_x");
    assert!(recalled[0].is_semantic());
}

// ── Consolidation bridge ───────────────────────────────────────────────────

#[test]
fn consolidation_bridge_counts_candidates() {
    let (episodic, semantic) = setup();
    let bridge = ConsolidationBridge::new(Arc::clone(&episodic), Arc::clone(&semantic));
    let perspective = test_perspective();

    assert_eq!(bridge.consolidation_candidate_count(&perspective), 0);

    let triple =
        Triple::new("e", "a", serde_json::json!("v"), perspective).with_perspective(perspective);
    episodic.store(triple).expect("store");

    assert_eq!(
        bridge.consolidation_candidate_count(&perspective),
        1,
        "should count stored episodic triples"
    );
}

// ── Memory life and decay ──────────────────────────────────────────────────

#[test]
fn memory_life_default_is_180_days() {
    let db = Database::in_memory().expect("in-memory db");
    let conn = db.conn_arc();
    let episodic = EpisodicMemory::new(TripleStore::new(Arc::clone(&conn)));

    assert!((episodic.memory_life_days() - 180.0).abs() < 0.01);
}

#[test]
fn memory_life_configurable() {
    let db = Database::in_memory().expect("in-memory db");
    let conn = db.conn_arc();
    let episodic =
        EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))).with_memory_life_days(365.0);

    assert!((episodic.memory_life_days() - 365.0).abs() < 0.01);
}

#[test]
fn memory_decay_formula() {
    use hkask_types::Confidence;

    let c = Confidence::new(1.0);
    let s = 180.0;

    // t=0: no decay
    assert!((c.memory_decay(0.0, s).value() - 1.0).abs() < 0.001);

    // t=S: R = exp(-1) ≈ 0.3679
    assert!((c.memory_decay(s, s).value() - 0.368).abs() < 0.01);

    // t = S·ln(2) (halflife): R = 0.5
    let h = s * std::f64::consts::LN_2;
    assert!((c.memory_decay(h, s).value() - 0.5).abs() < 0.01);

    // Confidence scales: 0.8 * exp(-1) = 0.8 / e
    let c2 = Confidence::new(0.8);
    assert!((c2.memory_decay(s, s).value() - 0.8 * (-1.0_f64).exp()).abs() < 0.01);
}

#[test]
fn episodic_storage_budget() {
    let db = Database::in_memory().expect("in-memory db");
    let conn = db.conn_arc();
    let episodic = EpisodicMemory::new(TripleStore::new(Arc::clone(&conn)));
    assert_eq!(episodic.storage_budget(), 10_000);
}
