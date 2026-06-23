//! CNS (Cybernetic Nervous System) types for hKask
//!
//! Namespace: cns.* (canonical observability namespace)
//! Key spans: cns.tool.*, cns.inference.*, cns.agent_pod.*, cns.gas.*, cns.curation.*, cns.sovereignty.*, cns.spec.*

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Domain newtypes (P2.3) ──────────────────────────────────────────────────

/// Communication queue depth for backpressure regulation.
///
/// Newtype wrapper that prevents accidental confusion with other numeric
/// thresholds in `SetPoints` (gas, variety deficit, error rate).
///
/// Defined in hkask-types (substrate crate) because it is shared across
/// hkask-cns (SetPoints, cybernetics loop) and hkask-agents
/// (communication loop).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QueueDepth(pub f64);

impl QueueDepth {
    /// Create a queue depth threshold.
    pub fn new(value: f64) -> Self {
        QueueDepth(value.max(0.0))
    }

    /// Default backpressure threshold: 100 messages.
    pub const DEFAULT_BACKPRESSURE: QueueDepth = QueueDepth(100.0);

    /// Return the raw `f64` value.
    pub fn as_raw(self) -> f64 {
        self.0
    }
}

// Circuit Breaker — States

/// Circuit breaker states
///
/// Defined here (not in hkask-cns) so the `CircuitBreakerPort` trait can
/// reference it without creating an upward dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

// CNS Health — Observability data struct

/// CNS health status
///
/// Pure data struct — construction logic (`cns_health_check`) lives in
/// hkask-cns where it has access to `AlgedonicManager`.
#[derive(Debug, Clone)]
pub struct CnsHealth {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

// ── CnsSpan — Typed CNS Span Identifiers ──────────────────────────────────

/// Typed CNS span identifiers — the authoritative CNS span registry.
///
/// Replaces stringly-typed `&str` constants. Invalid span values
/// are unrepresentable — the type system enforces validity at compile time (P8 — Semantic Grounding).
///
/// Display produces the canonical namespace string (e.g., `"cns.tool"`),
/// matching ν-event serialization and tracing targets.
/// `FromStr` is fallible — only canonical namespaces parse successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CnsSpan {
    /// Tool invocation span. Subsystem tracks which MCP server emitted the span.
    Tool { subsystem: ToolSubsystem },
    /// LLM inference request/response.
    Inference,
    /// Agent pod lifecycle events.
    AgentPod,
    /// Gas (energy) consumption tracking.
    Gas,
    /// Curation loop operations.
    Curation,
    /// Sovereignty boundary checks.
    Sovereignty,
    /// Keystore operations — key derivation, storage, signing (P4 security boundary).
    Keystore,
    /// Adapter (LoRA) lifecycle — store, route, delete (P4/P9 resource governance).
    Adapter,
    /// Backup operations — snapshot, restore, verify, prune (P1 data integrity).
    Backup,
    /// Condenser operations — compression ratio, health (P9 resource management).
    Condenser,
    /// Kata coaching operations — PDCA cycles, automaticity tracking (P3/P9).
    Kata,
    /// Skill lifecycle operations — discovery, loading, activation, drift detection (P5.1/P9).
    Skill,
    /// Specification operations (MDS).
    Spec,
    /// Chat/conversation operations.
    Chat,
    /// Memory encoding operations.
    MemoryEncode,
    /// Wallet balance queries.
    WalletBalance,
    /// Wallet deposit operations.
    WalletDeposit,
    /// Shielded (private) wallet deposits.
    WalletDepositShielded,
    /// Wallet withdrawal operations.
    WalletWithdrawal,
    /// Currency conversion operations.
    WalletConversion,
    /// API key issuance.
    WalletKeyIssued,
    /// API key revocation.
    WalletKeyRevoked,
    /// API key expiration.
    WalletKeyExpired,
    /// API key exhaustion (usage limit reached).
    WalletKeyExhausted,
    /// Blockchain chain errors.
    WalletChainError,
    /// Public seam coverage measurement.
    ArchitectureSeamCoverage,
    /// Public seam drift detection.
    ArchitectureSeamDrift,
    /// Contract violation detected.
    ContractViolated,
    /// Contract coverage measurement.
    ContractCoverage,
    /// Contract proposed by replicant (Phase B2–B4).
    ContractProposed,
    /// Contract accepted by human (Phase B3 consent gate).
    ContractAccepted,
    /// Contract rejected by human (Phase B3 consent gate).
    ContractRejected,
    /// Contract quality violation detected (missing expect:, [P{N}], Constraining:, etc.).
    ContractQualityViolated,
    /// ACP replicant memory size tracking.
    AcpReplicantMemorySize,
    /// ACP IDE connection state change.
    AcpIdeConnectionState,
    /// Multi-user role assignment (admin promotes/demotes).
    RoleAssigned,
    /// Multi-user invite sent.
    InviteSent,
    /// Multi-user invite accepted.
    InviteAccepted,
    /// Semantic triple published — triggers Curator sync.
    SemanticPublished,
    /// CI invariant gate violation — a pattern match failed with a principle anchor.
    CiInvariantViolation,
    /// A cargo-bolero fuzz target caught a failure.
    QaBoleroFailure,
    /// An autonomous repair was attempted (branch created, diff applied).
    QaRepairAttempted,
    /// A repair passed verification (all tests green).
    QaRepairVerified,
    /// Repairs exhausted — human investigation needed.
    QaRepairExhausted,
    /// A mutant survived — test suite has a gap.
    QaMutantSurvived,
    /// Federation CRDT merge — sync convergence event.
    FederationCrdtMerge,
    /// Federation link established between two CuratorPods.
    FederationLinkEstablished,
    /// Federation link lost — peer disconnected or revoked.
    FederationLinkLost,
    /// Federation link degraded — sync timeout (partition or peer death).
    FederationLinkDegraded,
    /// Federation member voluntarily left.
    FederationMemberLeft,
    /// Federation invitation sent to a peer.
    FederationInviteSent,
    /// Federation invitation received from a peer.
    FederationInviteReceived,
    /// Federation invitation accepted by the target.
    FederationInviteAccepted,
    /// Federation invitation rejected by the target.
    FederationInviteRejected,
    /// Federation invitation expired without response.
    FederationInviteExpired,
    /// Federation link temporarily paused (security measure).
    FederationLinkPaused,
    /// Federation link resumed from pause.
    FederationLinkResumed,
    /// Federation member revoked by another member.
    FederationMemberRevoked,
    /// Entire federation dissolved — all links terminated.
    FederationDissolved,
    /// Federation registry sync — user/agent data merged.
    FederationRegistrySync,
    /// Federation artifact sync — public artifact replicated.
    FederationArtifactSync,
    /// Federation Matrix conduit route established.
    FederationConduitRoute,
    /// Federation Matrix conduit route lost.
    FederationConduitRouteLost,
    /// Federation CRDT conflict detected — requires Curator attention.
    FederationCrdtConflict,
    /// Self-healing operation span. Canonical string: `"cns.heal"`.
    SelfHeal,
}

/// Subsystem identifier for `CnsSpan::Tool` — which MCP server emitted the span.
///
/// Derived from the `hkask-mcp-*` server naming convention.
/// Unknown or future servers use `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolSubsystem {
    WebSearch,
    SpecServer,
    Condenser,
    Training,
    Replica,
    Research,
    Communication,
    Registry,
    Wallet,
    Media,
    Kanban,
    Memory,
    Companies,
    Docproc,
    /// Catch-all for unknown or future MCP servers.
    Other,
}

impl ToolSubsystem {
    /// Map an MCP server name (e.g., "memory", "hkask-mcp-spec") to a ToolSubsystem.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  server_name is a non-empty string
    /// post: returns the corresponding ToolSubsystem variant; Other if unknown
    pub fn from_server_name(server_name: &str) -> Self {
        let name = server_name
            .strip_prefix("hkask-mcp-")
            .unwrap_or(server_name);
        match name {
            "memory" => ToolSubsystem::Memory,
            "condenser" => ToolSubsystem::Condenser,
            "spec" => ToolSubsystem::SpecServer,
            "research" => ToolSubsystem::Research,
            "companies" => ToolSubsystem::Companies,
            "communication" => ToolSubsystem::Communication,
            "fal" | "media" => ToolSubsystem::Media,
            "docproc" => ToolSubsystem::Docproc,
            "training" => ToolSubsystem::Training,
            "replica" => ToolSubsystem::Replica,
            "kanban" => ToolSubsystem::Kanban,
            _ => ToolSubsystem::Other,
        }
    }

    /// Canonical string suffix for the subsystem (e.g., `"web_search"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ToolSubsystem::WebSearch => "web_search",
            ToolSubsystem::SpecServer => "spec_server",
            ToolSubsystem::Condenser => "condenser",
            ToolSubsystem::Training => "training",
            ToolSubsystem::Replica => "replica",
            ToolSubsystem::Research => "research",
            ToolSubsystem::Communication => "communication",
            ToolSubsystem::Registry => "registry",
            ToolSubsystem::Wallet => "wallet",
            ToolSubsystem::Media => "media",
            ToolSubsystem::Kanban => "kanban",
            ToolSubsystem::Memory => "memory",
            ToolSubsystem::Companies => "companies",
            ToolSubsystem::Docproc => "docproc",
            ToolSubsystem::Other => "other",
        }
    }
}

impl CnsSpan {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid CnsSpan variant
    /// post: returns the canonical namespace string (e.g. "cns.tool.web_search"); output matches CANONICAL_NAMESPACES byte-for-byte
    ///
    /// This output must match ν-event serialization strings byte-for-byte
    /// (P8 — Semantic Grounding).
    pub fn as_str(&self) -> &'static str {
        match self {
            CnsSpan::Tool { subsystem } => match subsystem {
                ToolSubsystem::WebSearch => "cns.tool.web_search",
                ToolSubsystem::SpecServer => "cns.tool.spec_server",
                ToolSubsystem::Condenser => "cns.tool.condenser",
                ToolSubsystem::Training => "cns.tool.training",
                ToolSubsystem::Replica => "cns.tool.replica",
                ToolSubsystem::Research => "cns.tool.research",
                ToolSubsystem::Communication => "cns.tool.communication",
                ToolSubsystem::Registry => "cns.tool.registry",
                ToolSubsystem::Wallet => "cns.tool.wallet",
                ToolSubsystem::Media => "cns.tool.media",
                ToolSubsystem::Kanban => "cns.tool.kanban",
                ToolSubsystem::Memory => "cns.tool.memory",
                ToolSubsystem::Companies => "cns.tool.companies",
                ToolSubsystem::Docproc => "cns.tool.docproc",
                ToolSubsystem::Other => "cns.tool",
            },
            CnsSpan::Inference => "cns.inference",
            CnsSpan::AgentPod => "cns.agent_pod",
            CnsSpan::Gas => "cns.gas",
            CnsSpan::Curation => "cns.curation",
            CnsSpan::Sovereignty => "cns.sovereignty",
            CnsSpan::Keystore => "cns.keystore",
            CnsSpan::Adapter => "cns.adapter",
            CnsSpan::Backup => "cns.backup",
            CnsSpan::Condenser => "cns.condenser",
            CnsSpan::Kata => "cns.kata",
            CnsSpan::Skill => "cns.skill",
            CnsSpan::Spec => "cns.spec",
            CnsSpan::Chat => "cns.chat",
            CnsSpan::MemoryEncode => "cns.memory.encode",
            CnsSpan::WalletBalance => "cns.wallet.balance",
            CnsSpan::WalletDeposit => "cns.wallet.deposit",
            CnsSpan::WalletDepositShielded => "cns.wallet.deposit_shielded",
            CnsSpan::WalletWithdrawal => "cns.wallet.withdrawal",
            CnsSpan::WalletConversion => "cns.wallet.conversion",
            CnsSpan::WalletKeyIssued => "cns.wallet.key_issued",
            CnsSpan::WalletKeyRevoked => "cns.wallet.key_revoked",
            CnsSpan::WalletKeyExpired => "cns.wallet.key_expired",
            CnsSpan::WalletKeyExhausted => "cns.wallet.key_exhausted",
            CnsSpan::WalletChainError => "cns.wallet.chain_error",
            CnsSpan::ArchitectureSeamCoverage => "cns.architecture.seam.coverage",
            CnsSpan::ArchitectureSeamDrift => "cns.architecture.seam.drift",
            CnsSpan::ContractViolated => "cns.contract.violated",
            CnsSpan::ContractCoverage => "cns.contract.coverage",
            CnsSpan::ContractProposed => "cns.contract.proposed",
            CnsSpan::ContractAccepted => "cns.contract.accepted",
            CnsSpan::ContractRejected => "cns.contract.rejected",
            CnsSpan::ContractQualityViolated => "cns.contract.quality.violated",
            CnsSpan::AcpReplicantMemorySize => "cns.acp.replicant.memory_size",
            CnsSpan::AcpIdeConnectionState => "cns.acp.ide.connection_state",
            CnsSpan::RoleAssigned => "cns.multi.role.assigned",
            CnsSpan::InviteSent => "cns.multi.invite.sent",
            CnsSpan::InviteAccepted => "cns.multi.invite.accepted",
            CnsSpan::SemanticPublished => "cns.semantic.published",
            CnsSpan::CiInvariantViolation => "cns.ci.invariant.violation",
            CnsSpan::QaBoleroFailure => "cns.qa.bolero_failure",
            CnsSpan::QaRepairAttempted => "cns.qa.repair_attempted",
            CnsSpan::QaRepairVerified => "cns.qa.repair_verified",
            CnsSpan::QaRepairExhausted => "cns.qa.repair_exhausted",
            CnsSpan::QaMutantSurvived => "cns.qa.mutant_survived",
            CnsSpan::FederationCrdtMerge => "cns.federation.crdt_merge",
            CnsSpan::FederationLinkEstablished => "cns.federation.link_established",
            CnsSpan::FederationLinkLost => "cns.federation.link_lost",
            CnsSpan::FederationLinkDegraded => "cns.federation.link_degraded",
            CnsSpan::FederationMemberLeft => "cns.federation.member_left",
            CnsSpan::FederationInviteSent => "cns.federation.invite_sent",
            CnsSpan::FederationInviteReceived => "cns.federation.invite_received",
            CnsSpan::FederationInviteAccepted => "cns.federation.invite_accepted",
            CnsSpan::FederationInviteRejected => "cns.federation.invite_rejected",
            CnsSpan::FederationInviteExpired => "cns.federation.invite_expired",
            CnsSpan::FederationLinkPaused => "cns.federation.link_paused",
            CnsSpan::FederationLinkResumed => "cns.federation.link_resumed",
            CnsSpan::FederationMemberRevoked => "cns.federation.member_revoked",
            CnsSpan::FederationDissolved => "cns.federation.dissolved",
            CnsSpan::FederationRegistrySync => "cns.federation.registry_sync",
            CnsSpan::FederationArtifactSync => "cns.federation.artifact_sync",
            CnsSpan::FederationConduitRoute => "cns.federation.conduit_route",
            CnsSpan::FederationConduitRouteLost => "cns.federation.conduit_route_lost",
            CnsSpan::FederationCrdtConflict => "cns.federation.crdt_conflict",
            CnsSpan::SelfHeal => "cns.heal",
        }
    }
}

impl std::fmt::Display for CnsSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CnsSpan {
    type Err = ();

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  s is a string matching a canonical CnsSpan namespace
    /// post: returns Ok(CnsSpan) for canonical strings; Err(()) for unknown strings
    ///
    /// Only strings matching canonical `CnsSpan` namespaces parse
    /// successfully. Unknown strings return `Err(())`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cns.tool" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Other,
            }),
            "cns.tool.web_search" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::WebSearch,
            }),
            "cns.tool.spec_server" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::SpecServer,
            }),
            "cns.tool.condenser" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Condenser,
            }),
            "cns.tool.training" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Training,
            }),
            "cns.tool.replica" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Replica,
            }),
            "cns.tool.research" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Research,
            }),
            "cns.tool.communication" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Communication,
            }),
            "cns.tool.registry" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Registry,
            }),
            "cns.tool.wallet" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Wallet,
            }),
            "cns.tool.kanban" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Kanban,
            }),
            "cns.tool.media" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Media,
            }),
            "cns.inference" => Ok(CnsSpan::Inference),
            "cns.agent_pod" => Ok(CnsSpan::AgentPod),
            "cns.gas" => Ok(CnsSpan::Gas),
            "cns.curation" => Ok(CnsSpan::Curation),
            "cns.sovereignty" => Ok(CnsSpan::Sovereignty),
            "cns.keystore" => Ok(CnsSpan::Keystore),
            "cns.adapter" => Ok(CnsSpan::Adapter),
            "cns.backup" => Ok(CnsSpan::Backup),
            "cns.condenser" => Ok(CnsSpan::Condenser),
            "cns.kata" => Ok(CnsSpan::Kata),
            "cns.skill" => Ok(CnsSpan::Skill),
            "cns.spec" => Ok(CnsSpan::Spec),
            "cns.chat" => Ok(CnsSpan::Chat),
            "cns.memory.encode" => Ok(CnsSpan::MemoryEncode),
            "cns.wallet.balance" => Ok(CnsSpan::WalletBalance),
            "cns.wallet.deposit" => Ok(CnsSpan::WalletDeposit),
            "cns.wallet.deposit_shielded" => Ok(CnsSpan::WalletDepositShielded),
            "cns.wallet.withdrawal" => Ok(CnsSpan::WalletWithdrawal),
            "cns.wallet.conversion" => Ok(CnsSpan::WalletConversion),
            "cns.wallet.key_issued" => Ok(CnsSpan::WalletKeyIssued),
            "cns.wallet.key_revoked" => Ok(CnsSpan::WalletKeyRevoked),
            "cns.wallet.key_expired" => Ok(CnsSpan::WalletKeyExpired),
            "cns.wallet.key_exhausted" => Ok(CnsSpan::WalletKeyExhausted),
            "cns.wallet.chain_error" => Ok(CnsSpan::WalletChainError),
            "cns.architecture.seam.coverage" => Ok(CnsSpan::ArchitectureSeamCoverage),
            "cns.architecture.seam.drift" => Ok(CnsSpan::ArchitectureSeamDrift),
            "cns.contract.violated" => Ok(CnsSpan::ContractViolated),
            "cns.contract.coverage" => Ok(CnsSpan::ContractCoverage),
            "cns.contract.proposed" => Ok(CnsSpan::ContractProposed),
            "cns.contract.accepted" => Ok(CnsSpan::ContractAccepted),
            "cns.contract.rejected" => Ok(CnsSpan::ContractRejected),
            "cns.contract.quality.violated" => Ok(CnsSpan::ContractQualityViolated),
            "cns.acp.replicant.memory_size" => Ok(CnsSpan::AcpReplicantMemorySize),
            "cns.acp.ide.connection_state" => Ok(CnsSpan::AcpIdeConnectionState),
            "cns.multi.role.assigned" => Ok(CnsSpan::RoleAssigned),
            "cns.multi.invite.sent" => Ok(CnsSpan::InviteSent),
            "cns.multi.invite.accepted" => Ok(CnsSpan::InviteAccepted),
            "cns.semantic.published" => Ok(CnsSpan::SemanticPublished),
            "cns.ci.invariant.violation" => Ok(CnsSpan::CiInvariantViolation),
            "cns.qa.bolero_failure" => Ok(CnsSpan::QaBoleroFailure),
            "cns.qa.repair_attempted" => Ok(CnsSpan::QaRepairAttempted),
            "cns.qa.repair_verified" => Ok(CnsSpan::QaRepairVerified),
            "cns.qa.repair_exhausted" => Ok(CnsSpan::QaRepairExhausted),
            "cns.qa.mutant_survived" => Ok(CnsSpan::QaMutantSurvived),
            "cns.federation.crdt_merge" => Ok(CnsSpan::FederationCrdtMerge),
            "cns.federation.link_established" => Ok(CnsSpan::FederationLinkEstablished),
            "cns.federation.link_lost" => Ok(CnsSpan::FederationLinkLost),
            "cns.federation.link_degraded" => Ok(CnsSpan::FederationLinkDegraded),
            "cns.federation.member_left" => Ok(CnsSpan::FederationMemberLeft),
            "cns.federation.invite_sent" => Ok(CnsSpan::FederationInviteSent),
            "cns.federation.invite_received" => Ok(CnsSpan::FederationInviteReceived),
            "cns.federation.invite_accepted" => Ok(CnsSpan::FederationInviteAccepted),
            "cns.federation.invite_rejected" => Ok(CnsSpan::FederationInviteRejected),
            "cns.federation.invite_expired" => Ok(CnsSpan::FederationInviteExpired),
            "cns.federation.link_paused" => Ok(CnsSpan::FederationLinkPaused),
            "cns.federation.link_resumed" => Ok(CnsSpan::FederationLinkResumed),
            "cns.federation.member_revoked" => Ok(CnsSpan::FederationMemberRevoked),
            "cns.federation.dissolved" => Ok(CnsSpan::FederationDissolved),
            "cns.federation.registry_sync" => Ok(CnsSpan::FederationRegistrySync),
            "cns.federation.artifact_sync" => Ok(CnsSpan::FederationArtifactSync),
            "cns.federation.conduit_route" => Ok(CnsSpan::FederationConduitRoute),
            "cns.federation.conduit_route_lost" => Ok(CnsSpan::FederationConduitRouteLost),
            "cns.federation.crdt_conflict" => Ok(CnsSpan::FederationCrdtConflict),
            "cns.heal" => Ok(CnsSpan::SelfHeal),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod cns_span_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn cnsspan_display_produces_canonical_strings() {
        assert_eq!(
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Other
            }
            .to_string(),
            "cns.tool"
        );
        assert_eq!(CnsSpan::Inference.to_string(), "cns.inference");
        assert_eq!(CnsSpan::WalletBalance.to_string(), "cns.wallet.balance");
        assert_eq!(
            CnsSpan::ContractViolated.to_string(),
            "cns.contract.violated"
        );
    }

    #[test]
    fn cnsspan_from_str_rejects_invalid() {
        assert!(CnsSpan::from_str("cns.nonexistent").is_err());
        assert!(CnsSpan::from_str("invalid").is_err());
        assert!(CnsSpan::from_str("").is_err());
        assert!(CnsSpan::from_str("tool").is_err()); // short form not supported
    }

    #[test]
    fn cnsspan_from_str_round_trips() {
        let variants = vec![
            "cns.tool",
            "cns.inference",
            "cns.agent_pod",
            "cns.sovereignty",
            "cns.wallet.balance",
            "cns.contract.violated",
            "cns.heal",
        ];
        for s in variants {
            let span: CnsSpan = s.parse().expect("should parse");
            assert_eq!(span.to_string(), s, "Display should match input");
        }
    }

    #[test]
    fn cnsspan_tool_subsystem_produces_correct_string() {
        assert_eq!(
            CnsSpan::Tool {
                subsystem: ToolSubsystem::WebSearch
            }
            .to_string(),
            "cns.tool.web_search"
        );
        assert_eq!(
            CnsSpan::Tool {
                subsystem: ToolSubsystem::SpecServer
            }
            .to_string(),
            "cns.tool.spec_server"
        );
        assert_eq!(
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Other
            }
            .to_string(),
            "cns.tool"
        );
    }

    #[test]
    fn cnsspan_exhaustive_match_covers_all_canonical() {
        let all_variants = vec![
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Other,
            },
            CnsSpan::Inference,
            CnsSpan::AgentPod,
            CnsSpan::Gas,
            CnsSpan::Curation,
            CnsSpan::Sovereignty,
            CnsSpan::Keystore,
            CnsSpan::Adapter,
            CnsSpan::Backup,
            CnsSpan::Condenser,
            CnsSpan::Kata,
            CnsSpan::Skill,
            CnsSpan::Spec,
            CnsSpan::Chat,
            CnsSpan::MemoryEncode,
            CnsSpan::WalletBalance,
            CnsSpan::WalletDeposit,
            CnsSpan::WalletDepositShielded,
            CnsSpan::WalletWithdrawal,
            CnsSpan::WalletConversion,
            CnsSpan::WalletKeyIssued,
            CnsSpan::WalletKeyRevoked,
            CnsSpan::WalletKeyExpired,
            CnsSpan::WalletKeyExhausted,
            CnsSpan::WalletChainError,
            CnsSpan::ArchitectureSeamCoverage,
            CnsSpan::ArchitectureSeamDrift,
            CnsSpan::ContractViolated,
            CnsSpan::ContractCoverage,
            CnsSpan::ContractProposed,
            CnsSpan::ContractAccepted,
            CnsSpan::ContractRejected,
            CnsSpan::ContractQualityViolated,
            CnsSpan::AcpReplicantMemorySize,
            CnsSpan::AcpIdeConnectionState,
            CnsSpan::RoleAssigned,
            CnsSpan::InviteSent,
            CnsSpan::InviteAccepted,
            CnsSpan::SemanticPublished,
            CnsSpan::CiInvariantViolation,
            CnsSpan::QaBoleroFailure,
            CnsSpan::QaRepairAttempted,
            CnsSpan::QaRepairVerified,
            CnsSpan::QaRepairExhausted,
            CnsSpan::QaMutantSurvived,
            CnsSpan::FederationCrdtMerge,
            CnsSpan::FederationLinkEstablished,
            CnsSpan::FederationLinkLost,
            CnsSpan::FederationLinkDegraded,
            CnsSpan::FederationMemberLeft,
            CnsSpan::FederationInviteSent,
            CnsSpan::FederationInviteReceived,
            CnsSpan::FederationInviteAccepted,
            CnsSpan::FederationInviteRejected,
            CnsSpan::FederationInviteExpired,
            CnsSpan::FederationLinkPaused,
            CnsSpan::FederationLinkResumed,
            CnsSpan::FederationMemberRevoked,
            CnsSpan::FederationDissolved,
            CnsSpan::FederationRegistrySync,
            CnsSpan::FederationArtifactSync,
            CnsSpan::FederationConduitRoute,
            CnsSpan::FederationConduitRouteLost,
            CnsSpan::FederationCrdtConflict,
            CnsSpan::SelfHeal,
        ];
        for variant in &all_variants {
            let s = variant.to_string();
            assert!(
                !s.is_empty(),
                "{:?} should produce non-empty Display",
                variant
            );
            assert!(
                s.starts_with("cns."),
                "{:?} should start with cns.",
                variant
            );
        }
        assert!(
            all_variants.len() >= 20,
            "CNS span exhaustive test should cover at least 20 variants, found {}",
            all_variants.len()
        );
    }

    #[test]
    fn tool_subsystem_display_produces_valid_suffix() {
        assert_eq!(ToolSubsystem::WebSearch.as_str(), "web_search");
        assert_eq!(ToolSubsystem::SpecServer.as_str(), "spec_server");
        assert_eq!(ToolSubsystem::Other.as_str(), "other");
    }
}

// ── Public Seam Inventory (R7.3 watcher) ──

/// Per-crate public seam coverage snapshot.
///
/// Loaded from the machine-readable JSON inventory at startup.
/// R7.3 (CNS bot) tracks these as variety dimensions.
///
/// Field names match the JSON output from `scripts/audit/public-seam-inventory.sh`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamCoverage {
    /// Crate name — "workspace" for the totals aggregate.
    #[serde(default = "default_crate_name")]
    pub crate_name: String,
    pub total_items: u64,
    #[serde(rename = "covered", alias = "covered_items")]
    pub covered_items: u64,
    #[serde(rename = "uncovered", alias = "uncovered_items")]
    pub uncovered_items: u64,
    pub coverage_pct: f64,
    pub req_tests: u64,
    /// High-risk uncovered items — per-crate only, not present in totals.
    #[serde(default)]
    pub high_risk_uncovered: u64,
}

fn default_crate_name() -> String {
    "workspace".into()
}

/// Full public seam inventory, loaded from JSON at startup.
///
/// Generated by `scripts/audit/public-seam-inventory.sh` alongside
/// the human-readable markdown. This is the machine-readable form
/// that CNS ingests for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamInventory {
    /// ISO 8601 generation timestamp
    #[serde(default)]
    pub generated: String,
    /// Workspace-wide aggregate
    pub totals: SeamCoverage,
    /// Per-crate coverage data, keyed by crate name
    pub crates: HashMap<String, SeamCoverage>,
}

/// RetryConfig — Canonical retry configuration for all hKask subsystems
///
/// Combines exponential backoff with retryable status codes.
/// All delays are in milliseconds for serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,
    #[serde(default)]
    pub retryable_status: Vec<u16>,
}

fn default_multiplier() -> f64 {
    2.0
}

impl RetryConfig {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  attempt >= 0; self.initial_delay_ms, self.multiplier, self.max_delay_ms are valid
    /// post: returns the exponential backoff delay in ms, capped at self.max_delay_ms
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32);
        (delay as u64).min(self.max_delay_ms)
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  status is a valid HTTP status code (u16)
    /// post: returns true if status is in the retryable_status list
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_status.contains(&status)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            multiplier: 2.0,
            retryable_status: vec![408, 429, 500, 502, 503, 504],
        }
    }
}

#[cfg(test)]
mod retry_config_tests {
    use super::*;

    fn test_config() -> RetryConfig {
        RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            multiplier: 2.0,
            retryable_status: vec![429, 503],
        }
    }

    #[test]
    fn first_attempt_is_initial_delay() {
        let cfg = test_config();
        assert_eq!(cfg.delay_for_attempt(0), 100);
    }

    #[test]
    fn delay_doubles_each_attempt() {
        let cfg = test_config();
        assert_eq!(cfg.delay_for_attempt(1), 200);
        assert_eq!(cfg.delay_for_attempt(2), 400);
        assert_eq!(cfg.delay_for_attempt(3), 800);
    }

    #[test]
    fn delay_capped_at_max() {
        let cfg = test_config();
        let delay = cfg.delay_for_attempt(10); // 100 * 2^10 = 102400
        assert_eq!(delay, cfg.max_delay_ms);
    }

    #[test]
    fn delay_with_multiplier_one_is_constant() {
        let cfg = RetryConfig {
            multiplier: 1.0,
            ..test_config()
        };
        assert_eq!(cfg.delay_for_attempt(0), 100);
        assert_eq!(cfg.delay_for_attempt(5), 100);
    }

    #[test]
    fn default_config_is_reasonable() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert!(cfg.initial_delay_ms > 0);
        assert!(cfg.max_delay_ms > cfg.initial_delay_ms);
        assert!(!cfg.retryable_status.is_empty());
    }

    #[test]
    fn is_retryable_status_matches() {
        let cfg = test_config();
        assert!(cfg.is_retryable_status(429));
        assert!(cfg.is_retryable_status(503));
        assert!(!cfg.is_retryable_status(200));
        assert!(!cfg.is_retryable_status(404));
    }

    #[test]
    fn default_retryable_statuses() {
        let cfg = RetryConfig::default();
        // Standard retryable HTTP status codes
        assert!(cfg.is_retryable_status(429)); // Too Many Requests
        assert!(cfg.is_retryable_status(503)); // Service Unavailable
        assert!(!cfg.is_retryable_status(200)); // OK
    }

    // ── Regression: mutation corruption guard ────────────────────────────
    // 2026-06-19: cargo-mutants --in-place replaced delay_for_attempt body
    // with `1 /* ~ changed by cargo-mutants ~ */`. This test catches that
    // specific corruption — if the function ever returns a constant, it fails.

    #[test]
    fn regression_delay_for_attempt_is_exponential_not_constant() {
        let cfg = test_config();
        // The mutation changed the body to `1`. Verify it actually computes.
        let d0 = cfg.delay_for_attempt(0); // 100
        let d1 = cfg.delay_for_attempt(1); // 200
        let d2 = cfg.delay_for_attempt(2); // 400
        let d3 = cfg.delay_for_attempt(3); // 800
        assert_eq!(d0, 100, "attempt 0 must be initial_delay_ms, not constant");
        assert_ne!(d1, d0, "delay must change between attempts, not constant");
        assert!(d2 > d1, "delay must grow exponentially");
        assert!(d3 > d2, "delay must grow exponentially");
    }

    // ── Proptest: RetryConfig serialization round-trip ──────

    proptest::proptest! {
        #[test]
        fn retry_config_to_json_round_trip(
            max_retries in 0u32..10u32,
            initial_delay_ms in 0u64..10000u64,
            max_delay_ms in 1000u64..60000u64,
            multiplier in 1.0f64..5.0f64,
        ) {
            let cfg = RetryConfig {
                max_retries,
                initial_delay_ms,
                max_delay_ms,
                multiplier,
                retryable_status: vec![429, 503],
            };
            let json = serde_json::to_string(&cfg).unwrap();
            let parsed: RetryConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.max_retries, cfg.max_retries);
            assert_eq!(parsed.initial_delay_ms, cfg.initial_delay_ms);
            assert_eq!(parsed.max_delay_ms, cfg.max_delay_ms);
        }
    }
}
