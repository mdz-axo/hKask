//! FederationLinkManager — manages link lifecycle (invite, accept, pause, resume, revoke, leave).
//!
//! Owns the link registry. Communicates via FederationTransport.
//! Consumed by CuratorAgent for federation directive dispatch.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::cns_span::FederationSpan;
use hkask_ports::federation::{FederationDispatchError, FederationMessage, FederationTransport};
use hkask_types::event::{CyclePhase, RegulationRecord, RegulationSink, Span, SpanNamespace};

use crate::ReplicaId;
use crate::sync::invitation_policy::{InvitationPolicy, ManualInvitationPolicy};
use crate::sync::link::{FederationLink, LinkError, LinkState};

/// Manages federation link lifecycle.
pub struct FederationLinkManager {
    links: RwLock<HashMap<ReplicaId, FederationLink>>,
    transport: Arc<dyn FederationTransport>,
    local_replica: ReplicaId,
    event_sink: Arc<dyn RegulationSink>,
    /// Policy for evaluating incoming federation invitations (P2 consent).
    invitation_policy: Box<dyn InvitationPolicy>,
}

impl FederationLinkManager {
    pub fn new(
        local_replica: ReplicaId,
        transport: Arc<dyn FederationTransport>,
        event_sink: Arc<dyn RegulationSink>,
    ) -> Self {
        Self {
            links: RwLock::new(HashMap::new()),
            transport,
            local_replica,
            event_sink,
            invitation_policy: Box::new(ManualInvitationPolicy),
        }
    }

    /// Override the invitation policy (default: ManualInvitationPolicy).
    pub fn with_policy(mut self, policy: Box<dyn InvitationPolicy>) -> Self {
        self.invitation_policy = policy;
        self
    }

    /// Register a peer that may later be invited or linked.
    pub async fn register_peer(&self, link: FederationLink) {
        self.links
            .write()
            .await
            .insert(link.peer_replica.clone(), link);
    }

    /// Transition a peer's link to Invited state and send invitation message.
    pub async fn invite(&self, peer: ReplicaId) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        let now = Utc::now();
        link.transition_to(LinkState::Invited {
            invited_at: now,
            expires_at: now + chrono::Duration::hours(24),
        })?;
        // Send invitation over transport to the peer
        let msg = FederationMessage::InvitationRequest {
            from_replica: self.local_replica.clone(),
            server_domain: link.peer_server_domain.clone(),
            matrix_domain: link.peer_matrix_domain.clone(),
            curator_matrix_id: link.peer_curator_matrix_id.clone(),
            message: None,
        };
        let _ = self.transport.send(&peer, msg).await;
        self.emit_cns(FederationSpan::InviteSent, &peer);
        Ok(())
    }

    /// Accept an invitation from a peer, transitioning to Linked, and notify.
    pub async fn accept(&self, peer: ReplicaId) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Linked {
            established_at: Utc::now(),
        })?;
        // Notify peer of acceptance
        let msg = FederationMessage::InvitationResponse {
            accepted: true,
            from_replica: self.local_replica.clone(),
            reason: None,
        };
        let _ = self.transport.send(&peer, msg).await;
        self.emit_cns(FederationSpan::LinkEstablished, &peer);
        Ok(())
    }

    /// Reject an invitation, returning to Isolated, and notify.
    pub async fn reject(&self, peer: ReplicaId) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Isolated)?;
        // Notify peer of rejection
        let msg = FederationMessage::InvitationResponse {
            accepted: false,
            from_replica: self.local_replica.clone(),
            reason: Some("rejected by admin".into()),
        };
        let _ = self.transport.send(&peer, msg).await;
        self.emit_cns(FederationSpan::InviteRejected, &peer);
        Ok(())
    }

    /// Pause sync with a peer (security measure).
    pub async fn pause(&self, peer: ReplicaId, reason: String) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Paused {
            paused_at: Utc::now(),
            reason,
            initiated_by: self.local_replica.clone(),
        })?;
        self.emit_cns(FederationSpan::LinkPaused, &peer);
        Ok(())
    }

    /// Resume sync with a paused peer.
    pub async fn resume(&self, peer: ReplicaId) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Linked {
            established_at: Utc::now(),
        })?;
        self.emit_cns(FederationSpan::LinkResumed, &peer);
        Ok(())
    }

    /// Permanently revoke a peer.
    pub async fn revoke(&self, peer: ReplicaId, reason: String) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Revoked {
            revoked_at: Utc::now(),
            reason,
            initiated_by: self.local_replica.clone(),
            scope: crate::sync::link::RevocationScope::SingleMember,
        })?;
        self.emit_cns(FederationSpan::MemberRevoked, &peer);
        Ok(())
    }

    /// Voluntarily leave the federation.
    pub async fn leave(&self, reason: String) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let now = Utc::now();
        for (replica, link) in links.iter_mut() {
            if !matches!(link.state, LinkState::Revoked { .. }) {
                link.transition_to(LinkState::Revoked {
                    revoked_at: now,
                    reason: reason.clone(),
                    initiated_by: self.local_replica.clone(),
                    scope: crate::sync::link::RevocationScope::VoluntaryDeparture,
                })?;
                self.emit_cns(FederationSpan::MemberLeft, replica);
            }
        }
        Ok(())
    }

    /// Query a peer's current link state.
    pub async fn link_state(&self, peer: &ReplicaId) -> Option<LinkState> {
        self.links.read().await.get(peer).map(|l| l.state.clone())
    }

    /// List all linked peers.
    pub async fn linked_peers(&self) -> Vec<ReplicaId> {
        self.links
            .read()
            .await
            .iter()
            .filter(|(_, l)| matches!(l.state, LinkState::Linked { .. }))
            .map(|(r, _)| r.clone())
            .collect()
    }

    fn emit_cns(&self, span: FederationSpan, peer: &ReplicaId) {
        let s = Span::new(
            SpanNamespace::from_observable(&span).expect("domain span must be canonical"),
            "federation",
        );
        let event = RegulationRecord::new(
            hkask_types::WebID::from_persona(b"curator"),
            s,
            CyclePhase::Act,
            serde_json::json!({"peer": peer, "replica": self.local_replica}),
            0,
        );
        let _ = self.event_sink.persist(&event);
    }
}

#[async_trait::async_trait]
impl hkask_ports::federation::FederationDispatch for FederationLinkManager {
    async fn register_peer(
        &self,
        replica: ReplicaId,
        server_domain: String,
        matrix_domain: String,
        matrix_id: String,
    ) {
        let link = crate::sync::link::FederationLink::new(
            replica,
            server_domain,
            matrix_domain,
            matrix_id,
        );
        self.links
            .write()
            .await
            .insert(link.peer_replica.clone(), link);
    }

    async fn invite(
        &self,
        peer: ReplicaId,
        _message: Option<String>,
    ) -> Result<(), FederationDispatchError> {
        self.invite(peer)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn accept(&self, peer: ReplicaId) -> Result<(), FederationDispatchError> {
        self.accept(peer)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn reject(
        &self,
        peer: ReplicaId,
        _reason: Option<String>,
    ) -> Result<(), FederationDispatchError> {
        self.reject(peer)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn pause(&self, peer: ReplicaId, reason: String) -> Result<(), FederationDispatchError> {
        self.pause(peer, reason)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn resume(&self, peer: ReplicaId) -> Result<(), FederationDispatchError> {
        self.resume(peer)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn revoke(&self, peer: ReplicaId, reason: String) -> Result<(), FederationDispatchError> {
        self.revoke(peer, reason)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn leave(&self, reason: String) -> Result<(), FederationDispatchError> {
        self.leave(reason)
            .await
            .map_err(|e| FederationDispatchError::OperationFailed(e.to_string()))
    }

    async fn linked_peers(&self) -> Vec<ReplicaId> {
        self.linked_peers().await
    }

    async fn link_state(&self, peer: &ReplicaId) -> Option<String> {
        self.link_state(peer).await.map(|s| s.name().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::transport::InMemoryFederationTransport;
    use hkask_types::event::RegulationSink;

    struct NoopSink;
    impl RegulationSink for NoopSink {
        fn persist(&self, _event: &RegulationRecord) -> Result<(), hkask_types::InfrastructureError> {
            Ok(())
        }
    }

    fn make_manager() -> FederationLinkManager {
        let transport = InMemoryFederationTransport::new_shared();
        let t = InMemoryFederationTransport::for_replica(&transport, "alpha".into());
        FederationLinkManager::new("alpha".into(), Arc::new(t), Arc::new(NoopSink))
    }

    #[tokio::test]
    async fn invite_and_accept() {
        let mgr = make_manager();
        mgr.register_peer(FederationLink::new(
            "beta".into(),
            "b.example.com".into(),
            "matrix.b.example.com".into(),
            "@curator:b.example.com".into(),
        ))
        .await;

        mgr.invite("beta".into()).await.unwrap();
        assert!(matches!(
            mgr.link_state(&"beta".into()).await.unwrap(),
            LinkState::Invited { .. }
        ));

        mgr.accept("beta".into()).await.unwrap();
        assert!(matches!(
            mgr.link_state(&"beta".into()).await.unwrap(),
            LinkState::Linked { .. }
        ));
    }

    #[tokio::test]
    async fn pause_and_resume() {
        let mgr = make_manager();
        mgr.register_peer(FederationLink::new(
            "beta".into(),
            "b.example.com".into(),
            "matrix.b.example.com".into(),
            "@curator:b.example.com".into(),
        ))
        .await;
        mgr.invite("beta".into()).await.unwrap();
        mgr.accept("beta".into()).await.unwrap();

        mgr.pause("beta".into(), "debug".into()).await.unwrap();
        assert!(matches!(
            mgr.link_state(&"beta".into()).await.unwrap(),
            LinkState::Paused { .. }
        ));

        mgr.resume("beta".into()).await.unwrap();
        assert!(matches!(
            mgr.link_state(&"beta".into()).await.unwrap(),
            LinkState::Linked { .. }
        ));
    }

    #[tokio::test]
    async fn revoke_from_linked() {
        let mgr = make_manager();
        mgr.register_peer(FederationLink::new(
            "beta".into(),
            "b.example.com".into(),
            "matrix.b.example.com".into(),
            "@curator:b.example.com".into(),
        ))
        .await;
        mgr.invite("beta".into()).await.unwrap();
        mgr.accept("beta".into()).await.unwrap();

        mgr.revoke("beta".into(), "security".into()).await.unwrap();
        assert!(matches!(
            mgr.link_state(&"beta".into()).await.unwrap(),
            LinkState::Revoked { .. }
        ));
    }
}
