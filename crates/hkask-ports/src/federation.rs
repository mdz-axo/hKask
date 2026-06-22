//! Federation port traits — hexagonal abstractions for federation infrastructure.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type ReplicaId = String;

/// Minimal triple representation for federation sync — avoids depending on hkask-storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedTriple {
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationMessage {
    SyncRequest {
        version_vector: HashMap<ReplicaId, u64>,
    },
    SyncResponse {
        deltas: FederationDelta,
        version_vector: HashMap<ReplicaId, u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FederationDelta {
    pub triples: Vec<FederatedTriple>,
    pub triples_added: u64,
    pub triples_removed: u64,
    pub latency_ms: u64,
}

#[async_trait::async_trait]
pub trait FederationTransport: Send + Sync {
    async fn send(
        &self,
        peer: &ReplicaId,
        message: FederationMessage,
    ) -> Result<(), FederationTransportError>;
    async fn recv(&self) -> Result<(ReplicaId, FederationMessage), FederationTransportError>;
    fn simulate_partition(&self, _peer: &ReplicaId) {}
    fn heal_partition(&self, _peer: &ReplicaId) {}
}

#[derive(Debug, thiserror::Error)]
pub enum FederationTransportError {
    #[error("peer not found: {0}")]
    PeerNotFound(ReplicaId),
    #[error("peer partitioned: {0}")]
    PeerPartitioned(ReplicaId),
    #[error("transport error: {0}")]
    Transport(String),
}

pub trait FederationSyncPort: Send + Sync {
    fn query_public_since(
        &self,
        cursor: u64,
        limit: usize,
    ) -> Result<Vec<FederatedTriple>, FederationSyncError>;
    fn insert_federated(
        &self,
        triple: &FederatedTriple,
        source: &ReplicaId,
    ) -> Result<(), FederationSyncError>;
    fn cursor_for(&self, source: &ReplicaId) -> u64;
    fn advance_cursor(&self, source: &ReplicaId, cursor: u64);
}

#[derive(Debug, thiserror::Error)]
pub enum FederationSyncError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("not found")]
    NotFound,
}
