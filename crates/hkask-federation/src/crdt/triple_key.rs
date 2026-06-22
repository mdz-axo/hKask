//! CRDT key for federation semantic triples — EAV content hash.
//!
//! Uses the same BLAKE3 EAV hash as `recall_dedup::eav_hash()`.
//! Same entity+attribute+value → same key → automatic convergence.

use hkask_storage::Triple;

/// CRDT key for semantic triples. Content-addressed via EAV hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FederationTripleKey {
    /// BLAKE3 hash of (entity || \\x00 || attribute || \\x00 || canonical_value).
    eav_hash: [u8; 32],
}

impl FederationTripleKey {
    /// Create a key from a Triple using the same hash as recall_dedup.
    pub fn from_triple(triple: &Triple) -> Self {
        Self {
            eav_hash: hkask_memory::recall_dedup::eav_hash(triple),
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
    use hkask_storage::Triple;
    use hkask_types::WebID;

    fn make_triple(entity: &str, attr: &str, val: &str, owner: WebID) -> Triple {
        Triple::new(entity, attr, serde_json::Value::String(val.into()), owner)
    }

    #[test]
    fn same_eav_produces_same_key() {
        let owner = WebID::from_persona(b"test");
        let t1 = make_triple("sensor1", "temperature", "25", owner);
        let t2 = make_triple("sensor1", "temperature", "25", owner);
        // Different TripleID, same EAV → same key
        assert_eq!(
            FederationTripleKey::from_triple(&t1).eav_hash,
            FederationTripleKey::from_triple(&t2).eav_hash,
        );
    }

    #[test]
    fn different_value_produces_different_key() {
        let owner = WebID::from_persona(b"test");
        let t1 = make_triple("sensor1", "temperature", "25", owner);
        let t2 = make_triple("sensor1", "temperature", "26", owner);
        assert_ne!(
            FederationTripleKey::from_triple(&t1).eav_hash,
            FederationTripleKey::from_triple(&t2).eav_hash,
        );
    }

    #[test]
    fn metadata_ignored() {
        let owner = WebID::from_persona(b"test");
        let t1 = make_triple("x", "y", "z", owner);
        let mut t2 = t1.clone();
        t2.confidence = 0.5.into(); // different confidence
        assert_eq!(
            FederationTripleKey::from_triple(&t1).eav_hash,
            FederationTripleKey::from_triple(&t2).eav_hash,
        );
    }
}
