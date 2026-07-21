//! Federation port traits — hexagonal abstractions for federation infrastructure.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type ReplicaId = String;

/// Minimal h_mem representation for federation sync — avoids depending on hkask-storage.
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
    /// Sent by inviter to invitee to request federation linking.
    InvitationRequest {
        from_replica: ReplicaId,
        server_domain: String,
        matrix_domain: String,
        curator_matrix_id: String,
        message: Option<String>,
    },
    /// Sent by invitee in response to an InvitationRequest.
    InvitationResponse {
        accepted: bool,
        from_replica: ReplicaId,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FederationDelta {
    pub h_mems: Vec<FederatedTriple>,
    pub triples_added: u64,
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
    fn cursor_for(&self, source: &ReplicaId) -> u64;
    fn advance_cursor(&self, source: &ReplicaId, cursor: u64);
}

#[derive(Debug, thiserror::Error)]
pub enum FederationSyncError {
    #[error("storage error: {0}")]
    Storage(String),
}

/// Trait for dispatching federation lifecycle operations.
/// Implemented by FederationLinkManager in hkask-federation.
/// Consumed by CuratorAgent to avoid circular dependency.
#[async_trait::async_trait]
pub trait FederationDispatch: Send + Sync {
    async fn register_peer(
        &self,
        replica: ReplicaId,
        server_domain: String,
        matrix_domain: String,
        matrix_id: String,
    );
    async fn invite(
        &self,
        peer: ReplicaId,
        message: Option<String>,
    ) -> Result<(), FederationDispatchError>;
    async fn accept(&self, peer: ReplicaId) -> Result<(), FederationDispatchError>;
    async fn reject(
        &self,
        peer: ReplicaId,
        reason: Option<String>,
    ) -> Result<(), FederationDispatchError>;
    async fn pause(&self, peer: ReplicaId, reason: String) -> Result<(), FederationDispatchError>;
    async fn resume(&self, peer: ReplicaId) -> Result<(), FederationDispatchError>;
    async fn revoke(&self, peer: ReplicaId, reason: String) -> Result<(), FederationDispatchError>;
    async fn leave(&self, reason: String) -> Result<(), FederationDispatchError>;
    /// Dissolve the entire federation — all members leave simultaneously.
    ///
    /// Default implementation delegates to `leave()` with an appropriate reason.
    /// Implementors SHOULD emit `FederationDissolved` rather than `FederationMemberLeft`
    /// Regulation spans when federation-level semantics differ from individual departure.
    async fn dissolve(&self, reason: String) -> Result<(), FederationDispatchError> {
        self.leave(format!("federation dissolved: {reason}")).await
    }
    /// List all currently linked peers.
    async fn linked_peers(&self) -> Vec<ReplicaId>;
    /// Get the current link state name for a peer (e.g., "linked", "paused").
    async fn link_state(&self, peer: &ReplicaId) -> Option<String>;
}

/// Errors that can occur during federation dispatch operations.
///
/// Uses `String` for the message payload because this is a port boundary —
/// the trait hides implementation details of the concrete link manager.
/// Implementors (e.g., `FederationLinkManager`) map their domain errors
/// into this vocabulary via `From` conversion.
#[derive(Debug, thiserror::Error)]
pub enum FederationDispatchError {
    #[error("federation operation failed: {0}")]
    OperationFailed(String),
}
