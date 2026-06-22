//! Federation sync port adapter — bridges FederationSyncPort trait to SemanticIndex.
//!
//! Wraps SemanticIndex and handles ReplicaId ↔ PodID conversion.

use std::sync::{Arc, Mutex};

use hkask_ports::federation::{
    FederatedTriple, FederationSyncError, FederationSyncPort, ReplicaId,
};
use hkask_storage::{Triple, TripleStore};
use hkask_types::Visibility;
use hkask_types::id::PodID;

use crate::curator::SemanticIndex;

/// Adapter that implements FederationSyncPort backed by a SemanticIndex.
/// Converts ReplicaId (federation) ↔ PodID (intra-server).
pub struct SemanticIndexSyncPort {
    index: Arc<Mutex<SemanticIndex>>,
}

impl SemanticIndexSyncPort {
    pub fn new(index: Arc<Mutex<SemanticIndex>>) -> Self {
        Self { index }
    }

    fn replica_to_pod(replica: &ReplicaId) -> PodID {
        // Use a deterministic UUID derived from the replica string
        let hash = blake3::hash(replica.as_bytes());
        let bytes: [u8; 16] = hash.as_bytes()[..16].try_into().unwrap();
        PodID::from_uuid(uuid::Uuid::from_bytes(bytes))
    }
}

impl FederationSyncPort for SemanticIndexSyncPort {
    fn query_public_since(
        &self,
        cursor: u64,
        limit: usize,
    ) -> Result<Vec<FederatedTriple>, FederationSyncError> {
        let index = self
            .index
            .lock()
            .map_err(|e| FederationSyncError::Storage(e.to_string()))?;
        // Use TripleStore's query_by_entity with a wildcard-like approach.
        // For cursor-based incremental sync, we query all active public triples
        // and filter by rowid in the adapter layer.
        let all = index.store.clone().query_by_entity("%").unwrap_or_default();

        let results: Vec<FederatedTriple> = all
            .into_iter()
            .filter(|t| t.access.visibility == Visibility::Public)
            .skip(cursor as usize)
            .take(limit)
            .map(|t| FederatedTriple {
                entity: t.entity,
                attribute: t.attribute,
                value: t.value,
                confidence: t.confidence.value(),
            })
            .collect();
        Ok(results)
    }

    fn insert_federated(
        &self,
        triple: &FederatedTriple,
        source: &ReplicaId,
    ) -> Result<(), FederationSyncError> {
        let mut index = self
            .index
            .lock()
            .map_err(|e| FederationSyncError::Storage(e.to_string()))?;
        let pod_id = Self::replica_to_pod(source);
        let owner = hkask_types::WebID::from_uuid(pod_id.as_uuid());
        let t = Triple::new(
            &triple.entity,
            &triple.attribute,
            triple.value.clone(),
            owner,
        )
        .with_confidence(triple.confidence)
        .with_visibility(Visibility::Public);
        index
            .insert(&t, pod_id)
            .map_err(|e| FederationSyncError::Storage(e.to_string()))?;
        Ok(())
    }

    fn cursor_for(&self, source: &ReplicaId) -> u64 {
        let pod_id = Self::replica_to_pod(source);
        self.index
            .lock()
            .map(|i| i.cursor_for(&pod_id))
            .unwrap_or(0)
    }

    fn advance_cursor(&self, source: &ReplicaId, cursor: u64) {
        let pod_id = Self::replica_to_pod(source);
        if let Ok(mut index) = self.index.lock() {
            index.advance_cursor(pod_id, cursor);
        }
    }
}
