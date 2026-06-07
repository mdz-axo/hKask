//! Recall deduplication — entity-attribute-value hash strategy
//!
//! Implements the `entity_attribute_value_hash` deduplication strategy
//! declared in `standing-ensemble-session.yaml`. Filters duplicate triples
//! at recall time by computing a BLAKE3 hash of the canonical EAV content.
//!
//! This is Layer 1 of the three-layer DRY system:
//! - Layer 1: Memory recall dedup (this module)
//! - Layer 2: Session message dedup (hkask-ensemble/src/chat_dedup.rs)
//! - Layer 3: Prompt assembly dedup (hkask-templates/src/context_assembly.rs)

use hkask_storage::Triple;
use std::collections::HashSet;

/// Compute a canonical content hash for a triple using the EAV strategy.
///
/// The hash covers entity + attribute + canonical value, intentionally
/// excluding metadata (timestamps, confidence, perspective, visibility)
/// so that the same factual content stored at different times or with
/// different confidence levels is recognized as a duplicate.
pub fn eav_hash(triple: &Triple) -> [u8; 32] {
    let canonical = format!(
        "{}\x00{}\x00{}",
        triple.entity,
        triple.attribute,
        canonical_value(&triple.value)
    );
    hkask_types::blake3_hash(canonical.as_bytes())
}

/// Produce a deterministic string representation of a JSON value for hashing.
fn canonical_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonical_value).collect();
            format!("[{}]", parts.join(","))
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| format!("{}:{}", k, canonical_value(&map[*k])))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

/// Filter duplicate triples from a recall result set.
///
/// Returns only the first occurrence of each unique EAV content.
/// Preserves the original ordering (first-seen wins).
pub fn dedup_triples(triples: Vec<Triple>) -> Vec<Triple> {
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(triples.len());

    for triple in triples {
        let hash = eav_hash(&triple);
        if seen.insert(hash) {
            result.push(triple);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::id::WebID;
    use hkask_types::{Confidence, Visibility};
    use serde_json::{Value, json};

    fn make_triple(entity: &str, attribute: &str, value: Value) -> Triple {
        Triple::new(entity, attribute, value, WebID::new())
    }

    // ── eav_hash ───────────────────────────────────────────────────────────

    // P8 invariant: same EAV content produces same hash (deterministic)
    #[test]
    fn eav_hash_deterministic() {
        let t = make_triple("cat", "color", json!("black"));
        let h1 = eav_hash(&t);
        let h2 = eav_hash(&t);
        assert_eq!(h1, h2, "same triple must produce same hash");
    }

    // P8 invariant: different content produces different hash
    #[test]
    fn eav_hash_different_content_different_hash() {
        let t1 = make_triple("cat", "color", json!("black"));
        let t2 = make_triple("cat", "color", json!("white"));
        assert_ne!(
            eav_hash(&t1),
            eav_hash(&t2),
            "different value must produce different hash"
        );
    }

    // P8 invariant: different entity produces different hash
    #[test]
    fn eav_hash_different_entity_different_hash() {
        let t1 = make_triple("cat", "color", json!("black"));
        let t2 = make_triple("dog", "color", json!("black"));
        assert_ne!(
            eav_hash(&t1),
            eav_hash(&t2),
            "different entity must produce different hash"
        );
    }

    // P8 invariant: different attribute produces different hash
    #[test]
    fn eav_hash_different_attribute_different_hash() {
        let t1 = make_triple("cat", "color", json!("black"));
        let t2 = make_triple("cat", "size", json!("black"));
        assert_ne!(
            eav_hash(&t1),
            eav_hash(&t2),
            "different attribute must produce different hash"
        );
    }

    // P8 invariant: metadata differences (confidence, temporal, visibility) do NOT affect hash
    #[test]
    fn eav_hash_ignores_metadata() {
        let webid = WebID::new();
        let t1 = Triple::new("cat", "color", json!("black"), webid)
            .with_confidence(Confidence::full())
            .with_visibility(Visibility::Public);
        let t2 = Triple::new("cat", "color", json!("black"), webid)
            .with_confidence(Confidence::new(0.5))
            .with_visibility(Visibility::Private);
        assert_eq!(
            eav_hash(&t1),
            eav_hash(&t2),
            "metadata must not affect EAV hash"
        );
    }

    // ── canonical_value ────────────────────────────────────────────────────

    // P8 invariant: JSON object keys are sorted for deterministic canonical form
    #[test]
    fn canonical_value_object_sorts_keys() {
        let t1 = make_triple("e", "a", json!({"b": 1, "a": 2}));
        let t2 = make_triple("e", "a", json!({"a": 2, "b": 1}));
        assert_eq!(
            eav_hash(&t1),
            eav_hash(&t2),
            "key order must not affect hash"
        );
    }

    // P8 invariant: nested objects are also canonically sorted
    #[test]
    fn canonical_value_nested_object_sorts_keys() {
        let t1 = make_triple("e", "a", json!({"z": {"b": 1, "a": 2}, "a": 1}));
        let t2 = make_triple("e", "a", json!({"a": 1, "z": {"a": 2, "b": 1}}));
        assert_eq!(
            eav_hash(&t1),
            eav_hash(&t2),
            "nested key order must not affect hash"
        );
    }

    // ── dedup_triples ───────────────────────────────────────────────────────

    // P8 invariant: unique triples are all preserved
    #[test]
    fn dedup_triples_preserves_unique() {
        let triples = vec![
            make_triple("cat", "color", json!("black")),
            make_triple("dog", "size", json!("large")),
        ];
        let result = dedup_triples(triples);
        assert_eq!(result.len(), 2, "unique triples must all be preserved");
    }

    // P8 invariant: duplicate EAV content is removed
    #[test]
    fn dedup_triples_removes_duplicates() {
        let triples = vec![
            make_triple("cat", "color", json!("black")),
            make_triple("cat", "color", json!("black")),
        ];
        let result = dedup_triples(triples);
        assert_eq!(result.len(), 1, "duplicate EAV must be removed");
    }

    // P8 invariant: first-seen wins (ordering preserved)
    #[test]
    fn dedup_triples_first_seen_wins() {
        let t1 = make_triple("cat", "color", json!("black"));
        let t2 = make_triple("cat", "color", json!("black"));
        let t3 = make_triple("dog", "color", json!("brown"));
        let triples = vec![t1.clone(), t2, t3];
        let result = dedup_triples(triples);
        assert_eq!(result.len(), 2, "first-seen must win");
        assert_eq!(result[0].entity, "cat", "first entity must be cat");
        assert_eq!(result[1].entity, "dog", "second entity must be dog");
    }

    // P8 invariant: empty input returns empty output
    #[test]
    fn dedup_triples_empty_input() {
        let result: Vec<Triple> = dedup_triples(vec![]);
        assert!(result.is_empty(), "empty input must return empty");
    }
}
