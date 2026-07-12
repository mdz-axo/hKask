//! InvitationPolicy — controls whether incoming federation invitations are accepted.
//!
//! Default policy defers to admin (P2: affirmative consent). Alternative policies
//! can auto-accept configured peers or apply rate limiting.

use crate::ReplicaId;

/// Decision for an incoming federation invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvitationDecision {
    /// Auto-accept — link establishment proceeds immediately.
    Accept,
    /// Auto-reject with reason.
    Reject { reason: String },
    /// Defer to human admin for manual review (P2 default).
    DeferToAdmin,
}

/// Policy for evaluating incoming federation invitations.
pub trait InvitationPolicy: Send + Sync {
    fn evaluate(
        &self,
        from_replica: &ReplicaId,
        from_server: &str,
        _message: Option<&str>,
    ) -> InvitationDecision;
}

/// Default policy: always defer to admin (P2 affirmative consent).
pub struct ManualInvitationPolicy;

impl InvitationPolicy for ManualInvitationPolicy {
    fn evaluate(
        &self,
        _from_replica: &ReplicaId,
        _from_server: &str,
        _message: Option<&str>,
    ) -> InvitationDecision {
        InvitationDecision::DeferToAdmin
    }
}

/// Auto-accept invitations from a configured allowlist of peers.
pub struct AllowListInvitationPolicy {
    allowed: std::collections::HashSet<ReplicaId>,
}

impl AllowListInvitationPolicy {
    pub fn new(allowed: impl IntoIterator<Item = ReplicaId>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
        }
    }
}

impl InvitationPolicy for AllowListInvitationPolicy {
    fn evaluate(
        &self,
        from_replica: &ReplicaId,
        _from_server: &str,
        _message: Option<&str>,
    ) -> InvitationDecision {
        if self.allowed.contains(from_replica) {
            InvitationDecision::Accept
        } else {
            InvitationDecision::DeferToAdmin
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_policy_defers_to_admin() {
        let policy = ManualInvitationPolicy;
        assert_eq!(
            policy.evaluate(&"beta".into(), "b.example.com", None),
            InvitationDecision::DeferToAdmin
        );
    }

    #[test]
    fn allowlist_accepts_configured_peer() {
        let policy = AllowListInvitationPolicy::new(["beta".into(), "gamma".into()]);
        assert_eq!(
            policy.evaluate(&"beta".into(), "b.example.com", None),
            InvitationDecision::Accept
        );
    }

    #[test]
    fn allowlist_defers_unknown_peer() {
        let policy = AllowListInvitationPolicy::new(["beta".into()]);
        assert_eq!(
            policy.evaluate(&"delta".into(), "d.example.com", None),
            InvitationDecision::DeferToAdmin
        );
    }
}
