//! FederationService — lifecycle operations for federation links.
//!
//! Delegates to AgentService's FederationLinkManager and FederationSync.
//! Federation directives flow: CLI → FederationService → FederationDispatch → FederationLinkManager.

use std::sync::Arc;

use hkask_ports::federation::FederationDispatch;
use hkask_types::curator::CuratorDirective;

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

    /// Dispatch any CuratorDirective that is federation-related.
    ///
    /// This is the unified dispatch for CLI-issued federation directives.
    /// Non-federation directives return Ok(()) silently.
    pub async fn dispatch_directive(
        link_manager: &Arc<dyn FederationDispatch>,
        directive: &CuratorDirective,
    ) -> Result<(), ServiceError> {
        use CuratorDirective::*;
        match directive {
            InviteToFederation {
                peer_replica,
                peer_server_domain,
                peer_matrix_domain,
                peer_curator_matrix_id,
                message,
            } => {
                Self::invite(
                    link_manager,
                    peer_replica.clone(),
                    peer_server_domain.clone(),
                    peer_matrix_domain.clone(),
                    peer_curator_matrix_id.clone(),
                    message.clone(),
                )
                .await
            }
            AcceptFederationInvite { invitation_id } => {
                Self::accept(link_manager, invitation_id.clone()).await
            }
            RejectFederationInvite {
                invitation_id,
                reason: _,
            } => Self::reject(link_manager, invitation_id.clone()).await,
            PauseFederationLink {
                peer_replica,
                reason,
            } => Self::pause(link_manager, peer_replica.clone(), reason.clone()).await,
            ResumeFederationLink { peer_replica } => {
                Self::resume(link_manager, peer_replica.clone()).await
            }
            RevokeFederationMember {
                peer_replica,
                reason,
            } => Self::revoke(link_manager, peer_replica.clone(), reason.clone()).await,
            LeaveFederation { reason } => Self::leave(link_manager, reason.clone()).await,
            DissolveFederation { reason } => Self::dissolve(link_manager, reason.clone()).await,
            _ => Ok(()), // Not a federation directive — no-op
        }
    }
}
