//! SemanticIndex — Curator's merged view of all pods' public semantic data.
//!
//! Backed by the CuratorPod's own SQLCipher file. Tracks per-source-pod
//! cursors for incremental sync on CNS events.

use hkask_storage::{Triple, TripleStore};
use hkask_types::id::PodID;
use std::collections::HashMap;

/// Merged index of all pods' public semantic triples.
/// Lives on the CuratorPod, backed by its own SQLCipher.
pub struct SemanticIndex {
    /// Triple store backed by CuratorPod's database
    pub store: TripleStore,
    /// Last-seen triple rowid per source pod, for incremental sync
    cursors: HashMap<PodID, u64>,
}

impl SemanticIndex {
    /// Create a new empty index backed by the given TripleStore.
    pub fn new(store: TripleStore) -> Self {
        Self {
            store,
            cursors: HashMap::new(),
        }
    }

    /// Insert a semantic triple from a source pod.
    /// The source_pod is stored in the triple's `access.perspective` field
    /// (reused as provenance metadata — SemantiIndex triples have no episodic perspective).
    /// Returns true on successful insert.
    pub fn insert(
        &mut self,
        triple: &Triple,
        source_pod: PodID,
    ) -> Result<bool, hkask_storage::TripleError> {
        // Attach source pod provenance to the triple before storing.
        // PodID → WebID via UUID extraction (both are Id<T> wrapping Uuid).
        let mut triple = triple.clone();
        let webid = hkask_types::WebID::from_uuid(source_pod.as_uuid());
        triple.access = triple.access.with_perspective(webid);
        self.store.insert(&triple)?;
        Ok(true)
    }

    /// Query all triples for an entity across all source pods.
    pub fn query_by_entity(&self, entity: &str) -> Result<Vec<Triple>, hkask_storage::TripleError> {
        self.store.query_by_entity(entity)
    }

    /// Get the PodID from a triple's source provenance (stored in access.perspective).
    pub fn source_pod_of(triple: &Triple) -> Option<PodID> {
        triple.access.perspective.map(|webid| PodID::from_uuid(webid.as_uuid()))
    }

    /// Query triples by entity and attribute.
    pub fn query_by_entity_attribute(
        &self,
        entity: &str,
        attribute: &str,
    ) -> Result<Vec<Triple>, hkask_storage::TripleError> {
        self.store.query_by_entity_attribute(entity, attribute)
    }

    /// Get the cursor (last-seen triple rowid) for a source pod.
    /// Returns 0 if this pod has never published.
    pub fn cursor_for(&self, pod_id: &PodID) -> u64 {
        self.cursors.get(pod_id).copied().unwrap_or(0)
    }

    /// Advance the cursor for a source pod.
    pub fn advance_cursor(&mut self, pod_id: PodID, rowid: u64) {
        self.cursors.insert(pod_id, rowid);
    }

    /// Number of source pods tracked.
    pub fn source_count(&self) -> usize {
        self.cursors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;
    use hkask_types::{TripleID, Visibility, WebID};

    fn make_store() -> TripleStore {
        let db = Database::in_memory().expect("in-memory db");
        TripleStore::new(db.conn_arc())
    }

    fn make_triple(entity: &str, attribute: &str, value: &str) -> Triple {
        let owner = WebID::from_persona(b"test");
        Triple::new(
            entity,
            attribute,
            serde_json::Value::String(value.into()),
            owner,
        )
        .with_visibility(Visibility::Public)
    }

    #[test]
    fn insert_and_query() {
        let store = make_store();
        let mut index = SemanticIndex::new(store);
        let triple = make_triple("test", "name", "hello");
        let pod_id = PodID::new();

        assert!(index.insert(&triple, pod_id).unwrap());
        let results = index.query_by_entity("test").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attribute, "name");
    }

    #[test]
    fn cursor_tracking() {
        let store = make_store();
        let mut index = SemanticIndex::new(store);
        let pod_id = PodID::new();

        assert_eq!(index.cursor_for(&pod_id), 0);
        index.advance_cursor(pod_id, 42);
        assert_eq!(index.cursor_for(&pod_id), 42);
        assert_eq!(index.source_count(), 1);
    }
}
