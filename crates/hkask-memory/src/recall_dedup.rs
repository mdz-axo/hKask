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
