//! Federation-specific CNS span identifiers.

use hkask_types::observable_span::ObservableSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FederationSpan {
    CrdtMerge,
    LinkEstablished,
    LinkLost,
    LinkDegraded,
    MemberLeft,
    InviteSent,
    InviteReceived,
    InviteAccepted,
    InviteRejected,
    InviteExpired,
    LinkPaused,
    LinkResumed,
    MemberRevoked,
    Dissolved,
    RegistrySync,
    ArtifactSync,
    ConduitRoute,
    ConduitRouteLost,
    CrdtConflict,
}

impl FederationSpan {
    pub fn as_str(&self) -> &'static str {
        match self {
            FederationSpan::CrdtMerge => "cns.federation.crdt_merge",
            FederationSpan::LinkEstablished => "cns.federation.link_established",
            FederationSpan::LinkLost => "cns.federation.link_lost",
            FederationSpan::LinkDegraded => "cns.federation.link_degraded",
            FederationSpan::MemberLeft => "cns.federation.member_left",
            FederationSpan::InviteSent => "cns.federation.invite_sent",
            FederationSpan::InviteReceived => "cns.federation.invite_received",
            FederationSpan::InviteAccepted => "cns.federation.invite_accepted",
            FederationSpan::InviteRejected => "cns.federation.invite_rejected",
            FederationSpan::InviteExpired => "cns.federation.invite_expired",
            FederationSpan::LinkPaused => "cns.federation.link_paused",
            FederationSpan::LinkResumed => "cns.federation.link_resumed",
            FederationSpan::MemberRevoked => "cns.federation.member_revoked",
            FederationSpan::Dissolved => "cns.federation.dissolved",
            FederationSpan::RegistrySync => "cns.federation.registry_sync",
            FederationSpan::ArtifactSync => "cns.federation.artifact_sync",
            FederationSpan::ConduitRoute => "cns.federation.conduit_route",
            FederationSpan::ConduitRouteLost => "cns.federation.conduit_route_lost",
            FederationSpan::CrdtConflict => "cns.federation.crdt_conflict",
        }
    }
}

impl std::fmt::Display for FederationSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for FederationSpan {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cns.federation.crdt_merge" => Ok(FederationSpan::CrdtMerge),
            "cns.federation.link_established" => Ok(FederationSpan::LinkEstablished),
            "cns.federation.link_lost" => Ok(FederationSpan::LinkLost),
            "cns.federation.link_degraded" => Ok(FederationSpan::LinkDegraded),
            "cns.federation.member_left" => Ok(FederationSpan::MemberLeft),
            "cns.federation.invite_sent" => Ok(FederationSpan::InviteSent),
            "cns.federation.invite_received" => Ok(FederationSpan::InviteReceived),
            "cns.federation.invite_accepted" => Ok(FederationSpan::InviteAccepted),
            "cns.federation.invite_rejected" => Ok(FederationSpan::InviteRejected),
            "cns.federation.invite_expired" => Ok(FederationSpan::InviteExpired),
            "cns.federation.link_paused" => Ok(FederationSpan::LinkPaused),
            "cns.federation.link_resumed" => Ok(FederationSpan::LinkResumed),
            "cns.federation.member_revoked" => Ok(FederationSpan::MemberRevoked),
            "cns.federation.dissolved" => Ok(FederationSpan::Dissolved),
            "cns.federation.registry_sync" => Ok(FederationSpan::RegistrySync),
            "cns.federation.artifact_sync" => Ok(FederationSpan::ArtifactSync),
            "cns.federation.conduit_route" => Ok(FederationSpan::ConduitRoute),
            "cns.federation.conduit_route_lost" => Ok(FederationSpan::ConduitRouteLost),
            "cns.federation.crdt_conflict" => Ok(FederationSpan::CrdtConflict),
            _ => Err(()),
        }
    }
}

impl ObservableSpan for FederationSpan {
    fn as_str(&self) -> &'static str {
        FederationSpan::as_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::SpanNamespace;

    #[test]
    fn federation_span_namespaces_are_canonical() {
        let all = vec![
            FederationSpan::CrdtMerge,
            FederationSpan::LinkEstablished,
            FederationSpan::LinkLost,
            FederationSpan::LinkDegraded,
            FederationSpan::MemberLeft,
            FederationSpan::InviteSent,
            FederationSpan::InviteReceived,
            FederationSpan::InviteAccepted,
            FederationSpan::InviteRejected,
            FederationSpan::InviteExpired,
            FederationSpan::LinkPaused,
            FederationSpan::LinkResumed,
            FederationSpan::MemberRevoked,
            FederationSpan::Dissolved,
            FederationSpan::RegistrySync,
            FederationSpan::ArtifactSync,
            FederationSpan::ConduitRoute,
            FederationSpan::ConduitRouteLost,
            FederationSpan::CrdtConflict,
        ];
        for span in all {
            let ns = SpanNamespace::new(span.as_str()).unwrap();
            assert_eq!(
                ns.as_str(),
                span.as_str(),
                "FederationSpan::as_str() must match CANONICAL_NAMESPACES"
            );
        }
    }
}
