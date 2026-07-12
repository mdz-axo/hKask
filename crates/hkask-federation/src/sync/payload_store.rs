//! HMem payload store — maps EAV hash to full HMem data.
//!
//! The OR-Set determines which EAV hashes exist. The PayloadStore
//! stores the rich h_mem data (confidence, temporal bounds, provenance)
//! and upserts by confidence.

use std::collections::HashMap;

use hkask_storage::HMem;

use crate::crdt::FederationHMemKey;

/// Maps EAV hash → full HMem. OR-Set determines existence.
/// PayloadStore upserts by confidence — higher confidence wins.
pub struct HMemPayloadStore {
    payloads: HashMap<FederationHMemKey, HMem>,
}

impl HMemPayloadStore {
    pub fn new() -> Self {
        Self {
            payloads: HashMap::new(),
        }
    }

    /// Upsert a h_mem. If the same EAV hash already exists,
    /// keep the one with higher confidence.
    pub fn upsert(&mut self, h_mem: HMem) {
        let key = FederationHMemKey::from_h_mem(&h_mem);
        self.payloads
            .entry(key)
            .and_modify(|existing| {
                if h_mem.confidence > existing.confidence {
                    *existing = h_mem.clone();
                }
            })
            .or_insert(h_mem);
    }

    /// Get the h_mem for a given key.
    pub fn get(&self, key: &FederationHMemKey) -> Option<&HMem> {
        self.payloads.get(key)
    }

    /// Remove a key from the store.
    pub fn remove(&mut self, key: &FederationHMemKey) {
        self.payloads.remove(key);
    }

    /// Iterate over all stored (key, h_mem) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&FederationHMemKey, &HMem)> {
        self.payloads.iter()
    }

    pub fn len(&self) -> usize {
        self.payloads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.payloads.is_empty()
    }
}

impl Default for HMemPayloadStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;

    fn make_h_mem(entity: &str, attr: &str, val: &str, confidence: f64) -> HMem {
        let owner = WebID::from_persona(b"test");
        HMem::new(entity, attr, serde_json::Value::String(val.into()), owner)
            .with_confidence(confidence)
    }

    #[test]
    fn upsert_keeps_higher_confidence() {
        let mut store = HMemPayloadStore::new();
        store.upsert(make_h_mem("x", "y", "z", 0.5));
        store.upsert(make_h_mem("x", "y", "z", 0.9));
        let key = FederationHMemKey::from_h_mem(&make_h_mem("x", "y", "z", 1.0));
        let stored = store.get(&key).unwrap();
        assert!((stored.confidence.value() - 0.9).abs() < 0.001);
    }

    #[test]
    fn upsert_preserves_higher() {
        let mut store = HMemPayloadStore::new();
        store.upsert(make_h_mem("x", "y", "z", 0.9));
        store.upsert(make_h_mem("x", "y", "z", 0.3));
        let key = FederationHMemKey::from_h_mem(&make_h_mem("x", "y", "z", 1.0));
        let stored = store.get(&key).unwrap();
        assert!((stored.confidence.value() - 0.9).abs() < 0.001);
    }
}
