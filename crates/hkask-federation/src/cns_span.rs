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
            FederationSpan::CrdtMerge => "reg.federation.crdt_merge",
            FederationSpan::LinkEstablished => "reg.federation.link_established",
            FederationSpan::LinkLost => "reg.federation.link_lost",
            FederationSpan::LinkDegraded => "reg.federation.link_degraded",
            FederationSpan::MemberLeft => "reg.federation.member_left",
            FederationSpan::InviteSent => "reg.federation.invite_sent",
            FederationSpan::InviteReceived => "reg.federation.invite_received",
            FederationSpan::InviteAccepted => "reg.federation.invite_accepted",
            FederationSpan::InviteRejected => "reg.federation.invite_rejected",
            FederationSpan::InviteExpired => "reg.federation.invite_expired",
            FederationSpan::LinkPaused => "reg.federation.link_paused",
            FederationSpan::LinkResumed => "reg.federation.link_resumed",
            FederationSpan::MemberRevoked => "reg.federation.member_revoked",
            FederationSpan::Dissolved => "reg.federation.dissolved",
            FederationSpan::RegistrySync => "reg.federation.registry_sync",
            FederationSpan::ArtifactSync => "reg.federation.artifact_sync",
            FederationSpan::ConduitRoute => "reg.federation.conduit_route",
            FederationSpan::ConduitRouteLost => "reg.federation.conduit_route_lost",
            FederationSpan::CrdtConflict => "reg.federation.crdt_conflict",
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
            "reg.federation.crdt_merge" => Ok(FederationSpan::CrdtMerge),
            "reg.federation.link_established" => Ok(FederationSpan::LinkEstablished),
            "reg.federation.link_lost" => Ok(FederationSpan::LinkLost),
            "reg.federation.link_degraded" => Ok(FederationSpan::LinkDegraded),
            "reg.federation.member_left" => Ok(FederationSpan::MemberLeft),
            "reg.federation.invite_sent" => Ok(FederationSpan::InviteSent),
            "reg.federation.invite_received" => Ok(FederationSpan::InviteReceived),
            "reg.federation.invite_accepted" => Ok(FederationSpan::InviteAccepted),
            "reg.federation.invite_rejected" => Ok(FederationSpan::InviteRejected),
            "reg.federation.invite_expired" => Ok(FederationSpan::InviteExpired),
            "reg.federation.link_paused" => Ok(FederationSpan::LinkPaused),
            "reg.federation.link_resumed" => Ok(FederationSpan::LinkResumed),
            "reg.federation.member_revoked" => Ok(FederationSpan::MemberRevoked),
            "reg.federation.dissolved" => Ok(FederationSpan::Dissolved),
            "reg.federation.registry_sync" => Ok(FederationSpan::RegistrySync),
            "reg.federation.artifact_sync" => Ok(FederationSpan::ArtifactSync),
            "reg.federation.conduit_route" => Ok(FederationSpan::ConduitRoute),
            "reg.federation.conduit_route_lost" => Ok(FederationSpan::ConduitRouteLost),
            "reg.federation.crdt_conflict" => Ok(FederationSpan::CrdtConflict),
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
