//! Triple payload store — maps EAV hash to full Triple data.
//!
//! The OR-Set determines which EAV hashes exist. The PayloadStore
//! stores the rich triple data (confidence, temporal bounds, provenance)
//! and upserts by confidence.

use std::collections::HashMap;

use hkask_storage::Triple;

use crate::crdt::FederationTripleKey;

/// Maps EAV hash → full Triple. OR-Set determines existence.
/// PayloadStore upserts by confidence — higher confidence wins.
pub struct TriplePayloadStore {
    payloads: HashMap<FederationTripleKey, Triple>,
}

impl TriplePayloadStore {
    pub fn new() -> Self {
        Self {
            payloads: HashMap::new(),
        }
    }

    /// Upsert a triple. If the same EAV hash already exists,
    /// keep the one with higher confidence.
    pub fn upsert(&mut self, triple: Triple) {
        let key = FederationTripleKey::from_triple(&triple);
        self.payloads
            .entry(key)
            .and_modify(|existing| {
                if triple.confidence > existing.confidence {
                    *existing = triple.clone();
                }
            })
            .or_insert(triple);
    }

    /// Get the triple for a given key.
    pub fn get(&self, key: &FederationTripleKey) -> Option<&Triple> {
        self.payloads.get(key)
    }

    /// Remove a key from the store.
    pub fn remove(&mut self, key: &FederationTripleKey) {
        self.payloads.remove(key);
    }

    /// Iterate over all stored (key, triple) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&FederationTripleKey, &Triple)> {
        self.payloads.iter()
    }

    pub fn len(&self) -> usize {
        self.payloads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.payloads.is_empty()
    }
}

impl Default for TriplePayloadStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;

    fn make_triple(entity: &str, attr: &str, val: &str, confidence: f64) -> Triple {
        let owner = WebID::from_persona(b"test");
        Triple::new(entity, attr, serde_json::Value::String(val.into()), owner)
            .with_confidence(confidence)
    }

    #[test]
    fn upsert_keeps_higher_confidence() {
        let mut store = TriplePayloadStore::new();
        store.upsert(make_triple("x", "y", "z", 0.5));
        store.upsert(make_triple("x", "y", "z", 0.9));
        let key = FederationTripleKey::from_triple(&make_triple("x", "y", "z", 1.0));
        let stored = store.get(&key).unwrap();
        assert!((stored.confidence.value() - 0.9).abs() < 0.001);
    }

    #[test]
    fn upsert_preserves_higher() {
        let mut store = TriplePayloadStore::new();
        store.upsert(make_triple("x", "y", "z", 0.9));
        store.upsert(make_triple("x", "y", "z", 0.3));
        let key = FederationTripleKey::from_triple(&make_triple("x", "y", "z", 1.0));
        let stored = store.get(&key).unwrap();
        assert!((stored.confidence.value() - 0.9).abs() < 0.001);
    }
}
