//! FederationLink — pairwise connection between two CuratorPods.
//!
//! Tracks link state, peer metadata, and transition validation.

use chrono::{DateTime, Utc};

use crate::ReplicaId;

/// Six-state link lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    /// No link exists.
    Isolated,
    /// Invitation sent, awaiting response.
    Invited {
        invited_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    /// Link established, CRDT sync active.
    Linked { established_at: DateTime<Utc> },
    /// Sync suspended intentionally (security measure).
    Paused {
        paused_at: DateTime<Utc>,
        reason: String,
        initiated_by: ReplicaId,
    },
    /// Sync failed — may be partition, peer death, or lost pause notification.
    Degraded {
        degraded_at: DateTime<Utc>,
        failed_attempts: u64,
        last_success_at: DateTime<Utc>,
    },
    /// Permanently terminated.
    Revoked {
        revoked_at: DateTime<Utc>,
        reason: String,
        initiated_by: ReplicaId,
        scope: RevocationScope,
    },
}

/// Scope of a revocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationScope {
    SingleMember,
    VoluntaryDeparture,
}

impl LinkState {
    /// Human-readable name for logging.
    pub fn name(&self) -> &'static str {
        match self {
            LinkState::Isolated => "isolated",
            LinkState::Invited { .. } => "invited",
            LinkState::Linked { .. } => "linked",
            LinkState::Paused { .. } => "paused",
            LinkState::Degraded { .. } => "degraded",
            LinkState::Revoked { .. } => "revoked",
        }
    }
}

/// A federation link to a peer CuratorPod.
pub struct FederationLink {
    pub peer_replica: ReplicaId,
    pub peer_server_domain: String,
    pub peer_matrix_domain: String,
    pub peer_curator_matrix_id: String,
    pub state: LinkState,
}

impl FederationLink {
    /// Create a new isolated link entry for a peer.
    pub fn new(
        peer_replica: ReplicaId,
        server_domain: String,
        matrix_domain: String,
        curator_matrix_id: String,
    ) -> Self {
        Self {
            peer_replica,
            peer_server_domain: server_domain,
            peer_matrix_domain: matrix_domain,
            peer_curator_matrix_id: curator_matrix_id,
            state: LinkState::Isolated,
        }
    }

    /// Attempt to transition to a new state. Returns Ok(()) if valid, Err if invalid.
    ///
    /// Valid transitions per FEDERATION_V2.md §3.2:
    ///   Isolated → Invited
    ///   Invited → Linked | Isolated
    ///   Linked → Paused | Degraded | Revoked
    ///   Paused → Linked | Degraded | Revoked
    ///   Degraded → Linked | Revoked
    ///   Revoked → (terminal)
    pub fn transition_to(&mut self, new_state: LinkState) -> Result<(), LinkError> {
        use LinkState::*;
        let valid = match (&self.state, &new_state) {
            (Isolated, Invited { .. }) => true,
            (Invited { .. }, Linked { .. }) => true,
            (Invited { .. }, Isolated) => true,
            (Linked { .. }, Paused { .. }) => true,
            (Linked { .. }, Degraded { .. }) => true,
            (Linked { .. }, Revoked { .. }) => true,
            (Paused { .. }, Linked { .. }) => true,
            (Paused { .. }, Degraded { .. }) => true,
            (Paused { .. }, Revoked { .. }) => true,
            (Degraded { .. }, Linked { .. }) => true,
            (Degraded { .. }, Revoked { .. }) => true,
            _ if std::mem::discriminant(&self.state) == std::mem::discriminant(&new_state) => true,
            _ => false,
        };
        if valid {
            self.state = new_state;
            Ok(())
        } else {
            Err(LinkError::InvalidTransition {
                from: self.state.name().into(),
                to: new_state.name().into(),
            })
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("invalid transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },
    #[error("peer not found: {0}")]
    PeerNotFound(ReplicaId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_link(replica: &str) -> FederationLink {
        FederationLink::new(
            replica.into(),
            format!("{}.example.com", replica),
            format!("matrix.{}.example.com", replica),
            format!("@curator:{}.example.com", replica),
        )
    }

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn isolated_to_invited_valid() {
        let mut link = make_link("beta");
        assert!(
            link.transition_to(LinkState::Invited {
                invited_at: now(),
                expires_at: now() + chrono::Duration::hours(24),
            })
            .is_ok()
        );
    }

    #[test]
    fn invited_to_linked_valid() {
        let mut link = make_link("beta");
        link.state = LinkState::Invited {
            invited_at: now(),
            expires_at: now() + chrono::Duration::hours(24),
        };
        assert!(
            link.transition_to(LinkState::Linked {
                established_at: now()
            })
            .is_ok()
        );
    }

    #[test]
    fn invited_to_isolated_valid() {
        let mut link = make_link("beta");
        link.state = LinkState::Invited {
            invited_at: now(),
            expires_at: now(),
        };
        assert!(link.transition_to(LinkState::Isolated).is_ok());
    }

    #[test]
    fn linked_to_paused_valid() {
        let mut link = make_link("beta");
        link.state = LinkState::Linked {
            established_at: now(),
        };
        assert!(
            link.transition_to(LinkState::Paused {
                paused_at: now(),
                reason: "debug".into(),
                initiated_by: "alpha".into(),
            })
            .is_ok()
        );
    }

    #[test]
    fn linked_to_degraded_valid() {
        let mut link = make_link("beta");
        link.state = LinkState::Linked {
            established_at: now(),
        };
        assert!(
            link.transition_to(LinkState::Degraded {
                degraded_at: now(),
                failed_attempts: 4,
                last_success_at: now(),
            })
            .is_ok()
        );
    }

    #[test]
    fn isolated_to_linked_invalid() {
        let mut link = make_link("beta");
        assert!(
            link.transition_to(LinkState::Linked {
                established_at: now()
            })
            .is_err()
        );
    }

    #[test]
    fn revoked_is_terminal() {
        let mut link = make_link("beta");
        link.state = LinkState::Revoked {
            revoked_at: now(),
            reason: "test".into(),
            initiated_by: "alpha".into(),
            scope: RevocationScope::SingleMember,
        };
        assert!(
            link.transition_to(LinkState::Linked {
                established_at: now()
            })
            .is_err()
        );
    }

    #[test]
    fn degraded_to_linked_valid() {
        let mut link = make_link("beta");
        link.state = LinkState::Degraded {
            degraded_at: now(),
            failed_attempts: 4,
            last_success_at: now(),
        };
        assert!(
            link.transition_to(LinkState::Linked {
                established_at: now()
            })
            .is_ok()
        );
    }
}
