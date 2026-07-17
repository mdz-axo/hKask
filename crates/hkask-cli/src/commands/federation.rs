//! Federation commands — delegates to FederationService.

use hkask_federation::service::FederationService;
use hkask_ports::federation::FederationDispatch;
use std::sync::Arc;

use crate::block_on;
use crate::cli::FederationAction;

/// Run federation commands from the CLI.
///
/// Federation operations require the running server to have federation
/// enabled and initialized. The link manager is extracted from the
/// AgentService context.
pub fn run_federation(rt: &tokio::runtime::Runtime, action: FederationAction) {
    tracing::info!(target: "hkask.cli", operation = "federation", action = ?action, "CNS");

    let ctx = crate::commands::helpers::build_agent_service();
    let link_manager: Option<&Arc<dyn FederationDispatch>> = ctx.infra().federation.as_ref();

    let Some(lm) = link_manager else {
        eprintln!("Federation error: federation is not enabled on this server.");
        eprintln!("Set HKASK_FEDERATION_ENABLED=1 in your environment or configuration.");
        std::process::exit(1);
    };

    match action {
        FederationAction::Invite {
            peer_replica,
            peer_server_domain,
            peer_matrix_domain,
            peer_curator_matrix_id,
            message,
        } => {
            block_on!(
                rt,
                FederationService::invite(
                    lm,
                    peer_replica.clone(),
                    peer_server_domain.clone(),
                    peer_matrix_domain.clone(),
                    peer_curator_matrix_id.clone(),
                    message.clone(),
                ),
                "Invite failed"
            );
            println!("Invitation sent to {peer_replica}.");
        }

        FederationAction::Accept { invitation_id } => {
            block_on!(
                rt,
                FederationService::accept(lm, invitation_id.clone()),
                "Accept failed"
            );
            println!("Invitation from {invitation_id} accepted. Link established.");
        }

        FederationAction::Reject {
            invitation_id,
            reason,
        } => {
            let reason_str = reason.clone().unwrap_or_else(|| "rejected by admin".into());
            block_on!(
                rt,
                FederationService::reject(lm, invitation_id.clone(), reason.clone()),
                "Reject failed"
            );
            println!("Invitation from {invitation_id} rejected: {reason_str}");
        }

        FederationAction::Pause {
            peer_replica,
            reason,
        } => {
            block_on!(
                rt,
                FederationService::pause(lm, peer_replica.clone(), reason.clone()),
                "Pause failed"
            );
            println!("Sync with {peer_replica} paused: {reason}");
        }

        FederationAction::Resume { peer_replica } => {
            block_on!(
                rt,
                FederationService::resume(lm, peer_replica.clone()),
                "Resume failed"
            );
            println!("Sync with {peer_replica} resumed.");
        }

        FederationAction::Revoke {
            peer_replica,
            reason,
        } => {
            block_on!(
                rt,
                FederationService::revoke(lm, peer_replica.clone(), reason.clone()),
                "Revoke failed"
            );
            println!("Peer {peer_replica} revoked: {reason}");
        }

        FederationAction::Leave { reason } => {
            block_on!(
                rt,
                FederationService::leave(lm, reason.clone()),
                "Leave failed"
            );
            println!("Left federation: {reason}");
        }

        FederationAction::Dissolve { reason } => {
            block_on!(
                rt,
                FederationService::dissolve(lm, reason.clone()),
                "Dissolve failed"
            );
            println!("Federation dissolved: {reason}");
        }

        FederationAction::Status => {
            rt.block_on(async {
                let peers = lm.linked_peers().await;
                if peers.is_empty() {
                    println!("No active federation links.");
                } else {
                    println!("Active federation links ({})", peers.len());
                    for peer in &peers {
                        if let Some(state_name) = lm.link_state(peer).await {
                            println!("  {peer}: {state_name}");
                        } else {
                            println!("  {peer}: unknown");
                        }
                    }
                }
            });
        }
    }
}
