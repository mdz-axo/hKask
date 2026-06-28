//! FederationService — lifecycle operations for federation links.
//!
//! Delegates to AgentService's FederationLinkManager and FederationSync.
//! Federation directives flow: CLI → FederationService → FederationDispatch → FederationLinkManager.
//!
//! Extracted from hkask-services (ADR-040, 2026-06-27).

use std::sync::Arc;

use hkask_ports::federation::FederationDispatch;

/// Service for federation lifecycle operations.
///
/// Requires `AgentService` to be built with federation components wired
/// (opt-in via `ServiceConfig.federation_enabled`).
pub struct FederationService;

impl FederationService {
    /// Invite a remote server to join the federation.
    pub async fn invite(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
        peer_server_domain: String,
        peer_matrix_domain: String,
        peer_curator_matrix_id: String,
        _message: Option<String>,
    ) -> Result<(), String> {
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
            .map_err(|e| format!("invite failed: {e}"))
    }

    /// Accept a pending federation invitation.
    pub async fn accept(
        link_manager: &Arc<dyn FederationDispatch>,
        invitation_id: String,
    ) -> Result<(), String> {
        link_manager
            .accept(invitation_id)
            .await
            .map_err(|e| format!("accept failed: {e}"))
    }

    /// Reject a pending federation invitation.
    pub async fn reject(
        link_manager: &Arc<dyn FederationDispatch>,
        invitation_id: String,
    ) -> Result<(), String> {
        link_manager
            .reject(invitation_id)
            .await
            .map_err(|e| format!("reject failed: {e}"))
    }

    /// Pause federation sync with a peer (security measure).
    pub async fn pause(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
        reason: String,
    ) -> Result<(), String> {
        link_manager
            .pause(peer_replica, reason)
            .await
            .map_err(|e| format!("pause failed: {e}"))
    }

    /// Resume federation sync with a paused peer.
    pub async fn resume(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
    ) -> Result<(), String> {
        link_manager
            .resume(peer_replica)
            .await
            .map_err(|e| format!("resume failed: {e}"))
    }

    /// Revoke a single member from the federation.
    pub async fn revoke(
        link_manager: &Arc<dyn FederationDispatch>,
        peer_replica: String,
        reason: String,
    ) -> Result<(), String> {
        link_manager
            .revoke(peer_replica, reason)
            .await
            .map_err(|e| format!("revoke failed: {e}"))
    }

    /// Voluntarily leave the federation.
    pub async fn leave(
        link_manager: &Arc<dyn FederationDispatch>,
        reason: String,
    ) -> Result<(), String> {
        link_manager
            .leave(reason)
            .await
            .map_err(|e| format!("leave failed: {e}"))
    }

    /// Dissolve all federation links (alias for leave with dissolve reason).
    pub async fn dissolve(
        link_manager: &Arc<dyn FederationDispatch>,
        reason: String,
    ) -> Result<(), String> {
        link_manager
            .leave(format!("dissolved: {reason}"))
            .await
            .map_err(|e| format!("dissolve failed: {e}"))
    }
}
