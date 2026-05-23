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

/// Filter duplicates and return statistics alongside the deduplicated set.
pub fn dedup_triples_with_stats(triples: Vec<Triple>) -> DedupResult {
    let original_count = triples.len();
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(original_count);

    for triple in triples {
        let hash = eav_hash(&triple);
        if seen.insert(hash) {
            result.push(triple);
        }
    }

    let deduped_count = result.len();
    DedupResult {
        triples: result,
        original_count,
        duplicates_removed: original_count - deduped_count,
    }
}

/// Result of a deduplication operation with statistics.
#[derive(Debug)]
pub struct DedupResult {
    pub triples: Vec<Triple>,
    pub original_count: usize,
    pub duplicates_removed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Triple;
    use hkask_types::WebID;
    use serde_json::json;
    use uuid::Uuid;

    fn test_webid() -> WebID {
        WebID(Uuid::new_v4())
    }

    #[test]
    fn test_eav_hash_identical_triples_same_hash() {
        let t1 = Triple::new("Paris", "capital_of", json!("France"), test_webid());
        let t2 = Triple::new("Paris", "capital_of", json!("France"), test_webid());

        assert_eq!(eav_hash(&t1), eav_hash(&t2));
    }

    #[test]
    fn test_eav_hash_different_value_different_hash() {
        let t1 = Triple::new("Paris", "capital_of", json!("France"), test_webid());
        let t2 = Triple::new("Paris", "capital_of", json!("Germany"), test_webid());

        assert_ne!(eav_hash(&t1), eav_hash(&t2));
    }

    #[test]
    fn test_eav_hash_different_attribute_different_hash() {
        let t1 = Triple::new("Paris", "capital_of", json!("France"), test_webid());
        let t2 = Triple::new("Paris", "located_in", json!("France"), test_webid());

        assert_ne!(eav_hash(&t1), eav_hash(&t2));
    }

    #[test]
    fn test_eav_hash_ignores_metadata() {
        let t1 =
            Triple::new("Paris", "capital_of", json!("France"), test_webid()).with_confidence(0.9);
        let t2 =
            Triple::new("Paris", "capital_of", json!("France"), test_webid()).with_confidence(0.5);

        assert_eq!(eav_hash(&t1), eav_hash(&t2));
    }

    #[test]
    fn test_dedup_triples_removes_duplicates() {
        let triples = vec![
            Triple::new("Paris", "capital_of", json!("France"), test_webid()),
            Triple::new("Berlin", "capital_of", json!("Germany"), test_webid()),
            Triple::new("Paris", "capital_of", json!("France"), test_webid()),
            Triple::new("Tokyo", "capital_of", json!("Japan"), test_webid()),
            Triple::new("Berlin", "capital_of", json!("Germany"), test_webid()),
        ];

        let result = dedup_triples(triples);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].entity, "Paris");
        assert_eq!(result[1].entity, "Berlin");
        assert_eq!(result[2].entity, "Tokyo");
    }

    #[test]
    fn test_dedup_triples_empty_input() {
        let result = dedup_triples(Vec::new());
        assert!(result.is_empty());
    }

    #[test]
    fn test_dedup_triples_no_duplicates() {
        let triples = vec![
            Triple::new("Paris", "capital_of", json!("France"), test_webid()),
            Triple::new("Berlin", "capital_of", json!("Germany"), test_webid()),
        ];

        let result = dedup_triples(triples);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_with_stats() {
        let triples = vec![
            Triple::new("Paris", "capital_of", json!("France"), test_webid()),
            Triple::new("Paris", "capital_of", json!("France"), test_webid()),
            Triple::new("Paris", "capital_of", json!("France"), test_webid()),
        ];

        let result = dedup_triples_with_stats(triples);
        assert_eq!(result.original_count, 3);
        assert_eq!(result.duplicates_removed, 2);
        assert_eq!(result.triples.len(), 1);
    }

    #[test]
    fn test_canonical_value_object_ordering() {
        let v1 = json!({"a": 1, "b": 2});
        let v2 = json!({"b": 2, "a": 1});

        assert_eq!(canonical_value(&v1), canonical_value(&v2));
    }

    #[test]
    fn test_canonical_value_nested() {
        let v = json!({"key": [1, "two", null, true]});
        let s = canonical_value(&v);
        assert_eq!(s, "{key:[1,two,null,true]}");
    }
}
