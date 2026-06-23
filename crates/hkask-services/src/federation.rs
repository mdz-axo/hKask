//! FederationService — lifecycle operations for federation links.
//!
//! Delegates to AgentService's FederationLinkManager and FederationSync.
//! Federation directives flow: CLI → FederationService → FederationDispatch → FederationLinkManager.

use std::sync::Arc;

use hkask_ports::federation::FederationDispatch;

use crate::ServiceError;

/// Service for federation lifecycle operations.
///
/// Requires `AgentService` to be built with federation components wired
/// (opt-in via `ServiceConfig.federation_enabled`).
pub struct FederationService;

impl FederationService {
    /// Invite a remote server to join the federation.
    ///
    /// pre:  link_manager must be Some (federation enabled)
    /// post: peer transitions to Invited state; CNS event emitted
    pub async fn invite(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
        peer_server_domain: String,
        peer_matrix_domain: String,
        peer_curator_matrix_id: String,
        _message: Option<String>,
    ) -> Result<(), ServiceError> {
        link_manager
            .register_peer(
                peer_replica.clone(),
                peer_server_domain,
                peer_matrix_domain,
                peer_curator_matrix_id,
            )
            .await;
        link_manager
            .invite(peer_replica)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("invite failed: {e}"),
            })
    }

    /// Accept a pending federation invitation.
    ///
    /// pre:  link_manager must be Some
    /// post: peer transitions to Linked state; CNS event emitted
    pub async fn accept(
        link_manager: &Arc<dyn FederationDispatch>,
        invitation_id: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .accept(invitation_id)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("accept failed: {e}"),
            })
    }

    /// Reject a pending federation invitation.
    pub async fn reject(
        link_manager: &Arc<dyn FederationDispatch>,
        invitation_id: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .reject(invitation_id)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("reject failed: {e}"),
            })
    }

    /// Pause federation sync with a peer (security measure).
    pub async fn pause(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
        reason: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .pause(peer_replica, reason)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("pause failed: {e}"),
            })
    }

    /// Resume federation sync with a paused peer.
    pub async fn resume(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .resume(peer_replica)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("resume failed: {e}"),
            })
    }

    /// Revoke a single member from the federation.
    pub async fn revoke(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
        reason: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .revoke(peer_replica, reason)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("revoke failed: {e}"),
            })
    }

    /// Voluntarily leave the federation.
    pub async fn leave(
        link_manager: &Arc<dyn FederationDispatch>,
        reason: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .leave(reason)
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("leave failed: {e}"),
            })
    }

    /// Dissolve all federation links (alias for leave with dissolve reason).
    pub async fn dissolve(
        link_manager: &Arc<dyn FederationDispatch>,
        reason: String,
    ) -> Result<(), ServiceError> {
        link_manager
            .leave(format!("dissolved: {reason}"))
            .await
            .map_err(|e| ServiceError::Federation {
                message: format!("dissolve failed: {e}"),
            })
    }
}
