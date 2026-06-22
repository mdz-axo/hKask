//! FederationLinkManager — manages link lifecycle (invite, accept, pause, resume, revoke, leave).
//!
//! Owns the link registry. Communicates via FederationTransport.
//! Consumed by CuratorAgent for federation directive dispatch.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use hkask_ports::federation::FederationTransport;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};

use crate::ReplicaId;
use crate::sync::link::{FederationLink, LinkError, LinkState};

/// Manages federation link lifecycle.
pub struct FederationLinkManager {
    links: RwLock<HashMap<ReplicaId, FederationLink>>,
    transport: Arc<dyn FederationTransport>,
    local_replica: ReplicaId,
    event_sink: Arc<dyn NuEventSink>,
}

impl FederationLinkManager {
    pub fn new(
        local_replica: ReplicaId,
        transport: Arc<dyn FederationTransport>,
        event_sink: Arc<dyn NuEventSink>,
    ) -> Self {
        Self {
            links: RwLock::new(HashMap::new()),
            transport,
            local_replica,
            event_sink,
        }
    }

    /// Register a peer that may later be invited or linked.
    pub async fn register_peer(&self, link: FederationLink) {
        self.links
            .write()
            .await
            .insert(link.peer_replica.clone(), link);
    }

    /// Transition a peer's link to Invited state.
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
        self.emit_cns(CnsSpan::FederationInviteSent, &peer);
        Ok(())
    }

    /// Accept an invitation from a peer, transitioning to Linked.
    pub async fn accept(&self, peer: ReplicaId) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Linked {
            established_at: Utc::now(),
        })?;
        self.emit_cns(CnsSpan::FederationLinkEstablished, &peer);
        Ok(())
    }

    /// Reject an invitation, returning to Isolated.
    pub async fn reject(&self, peer: ReplicaId) -> Result<(), LinkError> {
        let mut links = self.links.write().await;
        let link = links
            .get_mut(&peer)
            .ok_or(LinkError::PeerNotFound(peer.clone()))?;
        link.transition_to(LinkState::Isolated)?;
        self.emit_cns(CnsSpan::FederationInviteRejected, &peer);
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
        self.emit_cns(CnsSpan::FederationLinkPaused, &peer);
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
        self.emit_cns(CnsSpan::FederationLinkResumed, &peer);
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
        self.emit_cns(CnsSpan::FederationMemberRevoked, &peer);
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
                self.emit_cns(CnsSpan::FederationMemberLeft, replica);
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

    fn emit_cns(&self, span: CnsSpan, peer: &ReplicaId) {
        let s = Span::new(SpanNamespace::from(span), "federation");
        let event = NuEvent::new(
            hkask_types::WebID::from_persona(b"curator"),
            s,
            Phase::Act,
            serde_json::json!({"peer": peer, "replica": self.local_replica}),
            0,
        );
        let _ = self.event_sink.persist(&event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::transport::InMemoryFederationTransport;
    use hkask_types::event::NuEventSink;

    struct NoopSink;
    impl NuEventSink for NoopSink {
        fn persist(&self, _event: &NuEvent) -> Result<(), hkask_types::InfrastructureError> {
            Ok(())
        }
    }

    fn make_manager() -> FederationLinkManager {
        let transport = InMemoryFederationTransport::new();
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
