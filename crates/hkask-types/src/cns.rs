//! CNS (Cybernetic Nervous System) types for hKask
//
//! Namespace: cns.* (canonical observability namespace)
//! Key spans: cns.tool.*, cns.prompt.*, cns.inference.*, cns.agent_pod.*, cns.connector.*, cns.pipeline.*, cns.gas.*, cns.review.*, cns.template.*, cns.curation.*, cns.variety.*, cns.sovereignty.*, cns.goal.*, cns.spec.*

// G2 Justification: This module exposes 8 public items because it defines CNS types — CnsSpan (51 variants), ToolSubsystem, QueueDepth, CircuitState, CnsHealth, SeamCoverage, SeamInventory, RetryConfig. CnsSpan alone carries 51 canonical namespace variants. Submodule split planned for v0.28.0.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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

impl fmt::Display for QueueDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "depth={:.0}", self.0)
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
/// [NORMATIVE] Replaces stringly-typed `&str` constants. Invalid span values
/// are unrepresentable — the type system enforces validity at compile time (P8 — Semantic Grounding).
///
/// [DECLARATIVE] `Display` produces the canonical namespace string (e.g., `"cns.tool"`),
/// preserving backward compatibility with existing tracing targets and ν-event serialization.
/// `FromStr` is fallible — only canonical namespaces parse successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CnsSpan {
    // ── Top-level spans ─────────────────────────────────────────────────
    /// Tool invocation span. Subsystem tracks which MCP server emitted the span.
    Tool { subsystem: ToolSubsystem },
    /// Prompt construction and framing.
    Prompt,
    /// LLM inference request/response.
    Inference,
    /// Agent pod lifecycle events.
    AgentPod,
    /// External connector operations (Matrix, HTTP, etc.).
    Connector,
    /// Pipeline execution (multi-step workflows).
    Pipeline,
    /// Gas (energy) consumption tracking.
    Gas,
    /// Review and audit operations.
    Review,
    /// Template registration and application.
    Template,
    /// Curation loop operations.
    Curation,
    /// Variety counter updates.
    Variety,
    /// Sovereignty boundary checks.
    Sovereignty,
    /// Goal lifecycle operations.
    Goal,
    /// Specification operations (MDS).
    Spec,
    /// Test execution and coverage.
    Test,
    /// Chat/conversation operations.
    Chat,

    // ── Hierarchical spans (defined in `CnsSpan`) ─────────────────────────
    /// Cybernetic backpressure signals.
    CyberneticsBackpressure,
    /// Cybernetic cadence/timing signals.
    CyberneticsCadence,
    /// CNS set point adjustments.
    SetPoint,
    /// Memory encoding operations.
    MemoryEncode,
    /// Memory budget tracking.
    MemoryBudget,

    // ── Wallet spans ────────────────────────────────────────────────────
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
    /// Treasury operations.
    WalletTreasury,
    /// Blockchain chain errors.
    WalletChainError,
    /// Privacy shield operations.
    WalletPrivacyShield,
    /// Privacy unshield operations.
    WalletPrivacyUnshield,
    /// Privacy-related errors.
    WalletPrivacyError,

    // ── Lazy Universe spans (P5 grounding) ──────────────────────────────
    /// Context condenser compression ratio.
    CondenserCompressionRatio,
    /// Evolution energy delta (least-action tracking).
    EvolutionEnergyDelta,
    /// Architecture module depth measurement.
    ArchitectureModuleDepth,

    // ── Architecture health spans ───────────────────────────────────────
    /// Public seam coverage measurement.
    ArchitectureSeamCoverage,
    /// Public seam drift detection.
    ArchitectureSeamDrift,

    // ── Improv spans (composable interaction grammar) ────────────────────
    /// Active improv mode.
    ImprovModeActive,
    /// Plussing ratio in improv interactions.
    ImprovPlussingRatio,
    /// Freestyle coherence measurement.
    ImprovFreestyleCoherence,
    /// Ensemble coherence measurement.
    ImprovEnsembleCoherence,
    /// Kata improv effectiveness.
    KataImprovEffectiveness,
    /// Improv cascade depth.
    ImprovCascadeDepth,

    // ── Outcome quality spans ───────────────────────────────────────────
    /// Tool outcome tracking (success/failure).
    OutcomeTool,
    /// Inference outcome tracking.
    OutcomeInference,
    /// Memory outcome tracking.
    OutcomeMemory,

    // ── Contract discipline spans (Testing Discipline §9.3) ─────────────
    /// Contract violation detected.
    ContractViolated,
    /// Contract coverage measurement.
    ContractCoverage,
}

/// Subsystem identifier for `CnsSpan::Tool` — which MCP server emitted the span.
///
/// [DECLARATIVE] Derived from the `hkask-mcp-*` server naming convention.
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
    /// Catch-all for unknown or future MCP servers.
    Other,
}

impl ToolSubsystem {
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
            ToolSubsystem::Other => "other",
        }
    }
}

impl std::fmt::Display for ToolSubsystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl CnsSpan {
    /// Produce the canonical namespace string (e.g., `"cns.tool.web_search"`).
    ///
    /// [NORMATIVE] This output must match the existing `CANONICAL_NAMESPACES` strings
    /// byte-for-byte to preserve backward compatibility with ν-event serialization
    /// and tracing targets (P8 — Semantic Grounding).
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
                ToolSubsystem::Other => "cns.tool",
            },
            CnsSpan::Prompt => "cns.prompt",
            CnsSpan::Inference => "cns.inference",
            CnsSpan::AgentPod => "cns.agent_pod",
            CnsSpan::Connector => "cns.connector",
            CnsSpan::Pipeline => "cns.pipeline",
            CnsSpan::Gas => "cns.gas",
            CnsSpan::Review => "cns.review",
            CnsSpan::Template => "cns.template",
            CnsSpan::Curation => "cns.curation",
            CnsSpan::Variety => "cns.variety",
            CnsSpan::Sovereignty => "cns.sovereignty",
            CnsSpan::Goal => "cns.goal",
            CnsSpan::Spec => "cns.spec",
            CnsSpan::Test => "cns.test",
            CnsSpan::Chat => "cns.chat",
            CnsSpan::CyberneticsBackpressure => "cns.cybernetics.backpressure",
            CnsSpan::CyberneticsCadence => "cns.cybernetics.cadence",
            CnsSpan::SetPoint => "cns.set_point",
            CnsSpan::MemoryEncode => "cns.memory.encode",
            CnsSpan::MemoryBudget => "cns.memory.budget",
            CnsSpan::WalletBalance => "cns.wallet.balance",
            CnsSpan::WalletDeposit => "cns.wallet.deposit",
            CnsSpan::WalletDepositShielded => "cns.wallet.deposit_shielded",
            CnsSpan::WalletWithdrawal => "cns.wallet.withdrawal",
            CnsSpan::WalletConversion => "cns.wallet.conversion",
            CnsSpan::WalletKeyIssued => "cns.wallet.key_issued",
            CnsSpan::WalletKeyRevoked => "cns.wallet.key_revoked",
            CnsSpan::WalletKeyExpired => "cns.wallet.key_expired",
            CnsSpan::WalletKeyExhausted => "cns.wallet.key_exhausted",
            CnsSpan::WalletTreasury => "cns.wallet.treasury",
            CnsSpan::WalletChainError => "cns.wallet.chain_error",
            CnsSpan::WalletPrivacyShield => "cns.wallet.privacy.shield",
            CnsSpan::WalletPrivacyUnshield => "cns.wallet.privacy.unshield",
            CnsSpan::WalletPrivacyError => "cns.wallet.privacy_error",
            CnsSpan::CondenserCompressionRatio => "cns.condenser.compression_ratio",
            CnsSpan::EvolutionEnergyDelta => "cns.evolution.energy_delta",
            CnsSpan::ArchitectureModuleDepth => "cns.architecture.module_depth",
            CnsSpan::ArchitectureSeamCoverage => "cns.architecture.seam.coverage",
            CnsSpan::ArchitectureSeamDrift => "cns.architecture.seam.drift",
            CnsSpan::ImprovModeActive => "cns.improv.mode.active",
            CnsSpan::ImprovPlussingRatio => "cns.improv.plussing.ratio",
            CnsSpan::ImprovFreestyleCoherence => "cns.improv.freestyle.coherence",
            CnsSpan::ImprovEnsembleCoherence => "cns.improv.ensemble.coherence",
            CnsSpan::KataImprovEffectiveness => "cns.kata.improv.effectiveness",
            CnsSpan::ImprovCascadeDepth => "cns.improv.cascade.depth",
            CnsSpan::OutcomeTool => "cns.outcome.tool",
            CnsSpan::OutcomeInference => "cns.outcome.inference",
            CnsSpan::OutcomeMemory => "cns.outcome.memory",
            CnsSpan::ContractViolated => "cns.contract.violated",
            CnsSpan::ContractCoverage => "cns.contract.coverage",
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

    /// Parse a canonical namespace string into a `CnsSpan` variant.
    ///
    /// [NORMATIVE] Only strings matching canonical `CnsSpan` namespaces parse
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
            "cns.tool.media" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Media,
            }),
            "cns.prompt" => Ok(CnsSpan::Prompt),
            "cns.inference" => Ok(CnsSpan::Inference),
            "cns.agent_pod" => Ok(CnsSpan::AgentPod),
            "cns.connector" => Ok(CnsSpan::Connector),
            "cns.pipeline" => Ok(CnsSpan::Pipeline),
            "cns.gas" => Ok(CnsSpan::Gas),
            "cns.review" => Ok(CnsSpan::Review),
            "cns.template" => Ok(CnsSpan::Template),
            "cns.curation" => Ok(CnsSpan::Curation),
            "cns.variety" => Ok(CnsSpan::Variety),
            "cns.sovereignty" => Ok(CnsSpan::Sovereignty),
            "cns.goal" => Ok(CnsSpan::Goal),
            "cns.spec" => Ok(CnsSpan::Spec),
            "cns.test" => Ok(CnsSpan::Test),
            "cns.chat" => Ok(CnsSpan::Chat),
            "cns.cybernetics.backpressure" => Ok(CnsSpan::CyberneticsBackpressure),
            "cns.cybernetics.cadence" => Ok(CnsSpan::CyberneticsCadence),
            "cns.set_point" => Ok(CnsSpan::SetPoint),
            "cns.memory.encode" => Ok(CnsSpan::MemoryEncode),
            "cns.memory.budget" => Ok(CnsSpan::MemoryBudget),
            "cns.wallet.balance" => Ok(CnsSpan::WalletBalance),
            "cns.wallet.deposit" => Ok(CnsSpan::WalletDeposit),
            "cns.wallet.deposit_shielded" => Ok(CnsSpan::WalletDepositShielded),
            "cns.wallet.withdrawal" => Ok(CnsSpan::WalletWithdrawal),
            "cns.wallet.conversion" => Ok(CnsSpan::WalletConversion),
            "cns.wallet.key_issued" => Ok(CnsSpan::WalletKeyIssued),
            "cns.wallet.key_revoked" => Ok(CnsSpan::WalletKeyRevoked),
            "cns.wallet.key_expired" => Ok(CnsSpan::WalletKeyExpired),
            "cns.wallet.key_exhausted" => Ok(CnsSpan::WalletKeyExhausted),
            "cns.wallet.treasury" => Ok(CnsSpan::WalletTreasury),
            "cns.wallet.chain_error" => Ok(CnsSpan::WalletChainError),
            "cns.wallet.privacy.shield" => Ok(CnsSpan::WalletPrivacyShield),
            "cns.wallet.privacy.unshield" => Ok(CnsSpan::WalletPrivacyUnshield),
            "cns.wallet.privacy_error" => Ok(CnsSpan::WalletPrivacyError),
            "cns.condenser.compression_ratio" => Ok(CnsSpan::CondenserCompressionRatio),
            "cns.evolution.energy_delta" => Ok(CnsSpan::EvolutionEnergyDelta),
            "cns.architecture.module_depth" => Ok(CnsSpan::ArchitectureModuleDepth),
            "cns.architecture.seam.coverage" => Ok(CnsSpan::ArchitectureSeamCoverage),
            "cns.architecture.seam.drift" => Ok(CnsSpan::ArchitectureSeamDrift),
            "cns.improv.mode.active" => Ok(CnsSpan::ImprovModeActive),
            "cns.improv.plussing.ratio" => Ok(CnsSpan::ImprovPlussingRatio),
            "cns.improv.freestyle.coherence" => Ok(CnsSpan::ImprovFreestyleCoherence),
            "cns.improv.ensemble.coherence" => Ok(CnsSpan::ImprovEnsembleCoherence),
            "cns.kata.improv.effectiveness" => Ok(CnsSpan::KataImprovEffectiveness),
            "cns.improv.cascade.depth" => Ok(CnsSpan::ImprovCascadeDepth),
            "cns.outcome.tool" => Ok(CnsSpan::OutcomeTool),
            "cns.outcome.inference" => Ok(CnsSpan::OutcomeInference),
            "cns.outcome.memory" => Ok(CnsSpan::OutcomeMemory),
            "cns.contract.violated" => Ok(CnsSpan::ContractViolated),
            "cns.contract.coverage" => Ok(CnsSpan::ContractCoverage),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod cns_span_tests {
    use super::*;
    use std::str::FromStr;

    // REQ: cns-span-001 — CnsSpan Display produces canonical namespace strings
    #[test]
    fn cnsspan_display_produces_canonical_strings() {
        assert_eq!(
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Other
            }
            .to_string(),
            "cns.tool"
        );
        assert_eq!(CnsSpan::Prompt.to_string(), "cns.prompt");
        assert_eq!(CnsSpan::Inference.to_string(), "cns.inference");
        assert_eq!(
            CnsSpan::CyberneticsBackpressure.to_string(),
            "cns.cybernetics.backpressure"
        );
        assert_eq!(CnsSpan::WalletBalance.to_string(), "cns.wallet.balance");
        assert_eq!(
            CnsSpan::ContractViolated.to_string(),
            "cns.contract.violated"
        );
    }

    // REQ: cns-span-002 — CnsSpan FromStr rejects invalid span identifiers
    #[test]
    fn cnsspan_from_str_rejects_invalid() {
        assert!(CnsSpan::from_str("cns.nonexistent").is_err());
        assert!(CnsSpan::from_str("invalid").is_err());
        assert!(CnsSpan::from_str("").is_err());
        assert!(CnsSpan::from_str("tool").is_err()); // short form not supported
    }

    // REQ: cns-span-003 — CnsSpan FromStr round-trips through Display
    #[test]
    fn cnsspan_from_str_round_trips() {
        let variants = vec![
            "cns.tool",
            "cns.prompt",
            "cns.inference",
            "cns.agent_pod",
            "cns.variety",
            "cns.sovereignty",
            "cns.cybernetics.backpressure",
            "cns.wallet.balance",
            "cns.condenser.compression_ratio",
            "cns.contract.violated",
        ];
        for s in variants {
            let span: CnsSpan = s.parse().expect("should parse");
            assert_eq!(span.to_string(), s, "Display should match input");
        }
    }

    // REQ: cns-span-004 — CnsSpan Tool with subsystem produces correct string
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

    // REQ: cns-span-005 — CnsSpan exhaustive match covers all canonical namespaces
    #[test]
    fn cnsspan_exhaustive_match_covers_all_canonical() {
        // Every variant must produce a non-empty Display string
        let all_variants = vec![
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Other,
            },
            CnsSpan::Prompt,
            CnsSpan::Inference,
            CnsSpan::AgentPod,
            CnsSpan::Connector,
            CnsSpan::Pipeline,
            CnsSpan::Gas,
            CnsSpan::Review,
            CnsSpan::Template,
            CnsSpan::Curation,
            CnsSpan::Variety,
            CnsSpan::Sovereignty,
            CnsSpan::Goal,
            CnsSpan::Spec,
            CnsSpan::Test,
            CnsSpan::Chat,
            CnsSpan::CyberneticsBackpressure,
            CnsSpan::CyberneticsCadence,
            CnsSpan::SetPoint,
            CnsSpan::MemoryEncode,
            CnsSpan::MemoryBudget,
            CnsSpan::WalletBalance,
            CnsSpan::WalletDeposit,
            CnsSpan::WalletDepositShielded,
            CnsSpan::WalletWithdrawal,
            CnsSpan::WalletConversion,
            CnsSpan::WalletKeyIssued,
            CnsSpan::WalletKeyRevoked,
            CnsSpan::WalletKeyExpired,
            CnsSpan::WalletKeyExhausted,
            CnsSpan::WalletTreasury,
            CnsSpan::WalletChainError,
            CnsSpan::WalletPrivacyShield,
            CnsSpan::WalletPrivacyUnshield,
            CnsSpan::WalletPrivacyError,
            CnsSpan::CondenserCompressionRatio,
            CnsSpan::EvolutionEnergyDelta,
            CnsSpan::ArchitectureModuleDepth,
            CnsSpan::ArchitectureSeamCoverage,
            CnsSpan::ArchitectureSeamDrift,
            CnsSpan::ImprovModeActive,
            CnsSpan::ImprovPlussingRatio,
            CnsSpan::ImprovFreestyleCoherence,
            CnsSpan::ImprovEnsembleCoherence,
            CnsSpan::KataImprovEffectiveness,
            CnsSpan::ImprovCascadeDepth,
            CnsSpan::OutcomeTool,
            CnsSpan::OutcomeInference,
            CnsSpan::OutcomeMemory,
            CnsSpan::ContractViolated,
            CnsSpan::ContractCoverage,
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
        // Verify count matches CANONICAL_NAMESPACES (excluding tool subsystem variants)
        // 51 variants total
        assert_eq!(all_variants.len(), 51);
    }

    // REQ: cns-span-006 — ToolSubsystem Display produces valid subsystem suffix
    #[test]
    fn tool_subsystem_display_produces_valid_suffix() {
        assert_eq!(ToolSubsystem::WebSearch.to_string(), "web_search");
        assert_eq!(ToolSubsystem::SpecServer.to_string(), "spec_server");
        assert_eq!(ToolSubsystem::Other.to_string(), "other");
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
    #[serde(rename = "covered")]
    pub covered_items: u64,
    #[serde(rename = "uncovered")]
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
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = (self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32)) as u64;
        delay.min(self.max_delay_ms)
    }

    /// Check if a status code is retryable
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
