//! CRDT key for federation semantic h_mems — EAV content hash.
//!
//! Uses the same BLAKE3 EAV hash as `recall_dedup::eav_hash()`.
//! Same entity+attribute+value → same key → automatic convergence.

use hkask_storage::HMem;

/// CRDT key for semantic h_mems. Content-addressed via EAV hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FederationHMemKey {
    /// BLAKE3 hash of (entity || \\x00 || attribute || \\x00 || canonical_value).
    eav_hash: [u8; 32],
}

impl FederationHMemKey {
    /// Create a key from a HMem using the same hash as recall_dedup.
    pub fn from_h_mem(h_mem: &HMem) -> Self {
        Self {
            eav_hash: hkask_memory::recall_dedup::eav_hash(h_mem),
        }
    }

    /// Create a key directly from pre-computed EAV hash bytes.
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self { eav_hash: hash }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::HMem;
    use hkask_types::WebID;

    fn make_h_mem(entity: &str, attr: &str, val: &str, owner: WebID) -> HMem {
        HMem::new(entity, attr, serde_json::Value::String(val.into()), owner)
    }

    #[test]
    fn same_eav_produces_same_key() {
        let owner = WebID::from_persona(b"test");
        let t1 = make_h_mem("sensor1", "temperature", "25", owner);
        let t2 = make_h_mem("sensor1", "temperature", "25", owner);
        // Different HMemId, same EAV → same key
        assert_eq!(
            FederationHMemKey::from_h_mem(&t1).eav_hash,
            FederationHMemKey::from_h_mem(&t2).eav_hash,
        );
    }

    #[test]
    fn different_value_produces_different_key() {
        let owner = WebID::from_persona(b"test");
        let t1 = make_h_mem("sensor1", "temperature", "25", owner);
        let t2 = make_h_mem("sensor1", "temperature", "26", owner);
        assert_ne!(
            FederationHMemKey::from_h_mem(&t1).eav_hash,
            FederationHMemKey::from_h_mem(&t2).eav_hash,
        );
    }

    #[test]
    fn metadata_ignored() {
        let owner = WebID::from_persona(b"test");
        let t1 = make_h_mem("x", "y", "z", owner);
        let mut t2 = t1.clone();
        t2.confidence = 0.5.into(); // different confidence
        assert_eq!(
            FederationHMemKey::from_h_mem(&t1).eav_hash,
            FederationHMemKey::from_h_mem(&t2).eav_hash,
        );
    }
}
