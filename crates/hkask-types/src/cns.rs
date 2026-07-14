//! Core CNS (Cybernetic Nervous System) types for hKask
//!
//! Core spans: cns.tool.*, cns.inference.*, cns.agent_pod.*, cns.gas.*,
//! cns.curation.*, cns.heal.*, cns.memory.encode.*
//!
//! Domain-specific spans have moved to their respective domain crates.
//! All namespace strings are registered in CANONICAL_NAMESPACES (event.rs).

use serde::{Deserialize, Serialize};

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
#[repr(u8)]
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
    /// Session-level EMA of domain variety (survives window resets).
    /// 0.0 when no domains have been tracked.
    pub variety_ema: f64,
}

/// Regulation loop health — the Curator's window into regulatory effectiveness.
///
/// Aggregated from `ImpactReport` decisions across regulation cycles.
/// Enables the metacognition loop to answer: "are our regulatory actions working?"
#[derive(Debug, Clone, Default)]
pub struct RegulationHealth {
    /// Total regulation cycles recorded.
    pub total_cycles: u64,
    /// Actions accepted (improved or within noise tolerance).
    pub accepted: u64,
    /// Actions staged for review (moderately ineffective).
    pub staged: u64,
    /// Actions blocked (severely counterproductive).
    pub blocked: u64,
}

impl RegulationHealth {
    /// Ratio of accepted actions to total (0.0–1.0). 1.0 if no actions recorded.
    pub fn effectiveness(&self) -> f64 {
        let total = self.accepted + self.staged + self.blocked;
        if total == 0 {
            1.0
        } else {
            self.accepted as f64 / total as f64
        }
    }
}

// ── CnsSpan — Core CNS Span Identifiers ────────────────────────────────────

/// Core CNS span identifiers — spans that are constructed in 2+ crates from
/// different dependency domains (the "cross-cutting concern" test).
///
/// Domain-specific spans (wallet, federation, contracts, QA, metrics, deploy,
/// backup, ACP, curator, etc.) have moved to their respective domain crates
/// as enums implementing [`ObservableSpan`](crate::ObservableSpan).
///
/// All namespace strings — core and domain — are registered in
/// `CANONICAL_NAMESPACES` (in `event.rs`), the single source of truth for
/// what CNS spans exist.
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
    /// Curation loop operations — registry sync, pod sync, directive issuance.
    Curation,
    /// Self-healing operation span. Canonical string: `"cns.heal"`.
    SelfHeal,
    /// Memory encoding operations.
    MemoryEncode,
}

/// Subsystem identifier for `CnsSpan::Tool` — which MCP server emitted the span.
///
/// Derived from the `hkask-mcp-*` server naming convention.
/// Unknown or future servers use `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolSubsystem {
    WebSearch,
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
    Filesystem,
    Curator,
    /// Catch-all for unknown or future MCP servers.
    Other,
}

impl ToolSubsystem {
    /// Map an MCP server name (e.g., "memory", "hkask-mcp-replica") to a ToolSubsystem.
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
            "research" => ToolSubsystem::Research,
            "companies" => ToolSubsystem::Companies,
            "communication" => ToolSubsystem::Communication,
            "fal" | "media" => ToolSubsystem::Media,
            "docproc" => ToolSubsystem::Docproc,
            "training" => ToolSubsystem::Training,
            "replica" => ToolSubsystem::Replica,
            "kanban" => ToolSubsystem::Kanban,
            "curator" => ToolSubsystem::Curator,
            _ => ToolSubsystem::Other,
        }
    }

    /// Canonical string suffix for the subsystem (e.g., `"web_search"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ToolSubsystem::WebSearch => "web_search",
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
            ToolSubsystem::Filesystem => "filesystem",
            ToolSubsystem::Curator => "curator",
            ToolSubsystem::Other => "other",
        }
    }
}

impl CnsSpan {
    /// Emit a typed CNS span event through the `tracing` infrastructure.
    ///
    /// Enforces the canonical CNS emission convention (PRINCIPLES.md §9.2):
    /// - `target` = `"cns"` root namespace (full domain in `cns_domain` field)
    /// - `cns_domain` = `self.as_str()` (e.g. `"cns.tool.media"`)
    /// - `operation` = the verb describing what occurred (e.g. `"invoked"`)
    /// - message = `"CNS"` (required for downstream ν-event parsing)
    ///
    /// Callers that need additional structured fields can attach them by
    /// entering a child [`mod@tracing::span`] before calling `emit()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use hkask_types::cns::{CnsSpan, ToolSubsystem};
    ///
    /// CnsSpan::Tool { subsystem: ToolSubsystem::Media }
    ///     .emit("invoked");
/// ```
    pub fn emit(&self, operation: &str) {
        tracing::info!(
            target: "cns",
            cns_domain = %self.as_str(),
            operation = %operation,
            "CNS",
        );
    }

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
                ToolSubsystem::Filesystem => "cns.tool.filesystem",
                ToolSubsystem::Curator => "cns.tool.curator",
                ToolSubsystem::Other => "cns.tool",
            },
            CnsSpan::Inference => "cns.inference",
            CnsSpan::AgentPod => "cns.agent_pod",
            CnsSpan::Gas => "cns.gas",
            CnsSpan::Curation => "cns.curation",
            CnsSpan::SelfHeal => "cns.heal",
            CnsSpan::MemoryEncode => "cns.memory.encode",
        }
    }
}

impl std::fmt::Display for CnsSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl crate::observable_span::ObservableSpan for CnsSpan {
    fn as_str(&self) -> &'static str {
        CnsSpan::as_str(self)
    }

    fn emit(&self, operation: &str) {
        CnsSpan::emit(self, operation);
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
            "cns.tool.kanban" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Kanban,
            }),
            "cns.tool.memory" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Memory,
            }),
            "cns.tool.companies" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Companies,
            }),
            "cns.tool.docproc" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Docproc,
            }),
            "cns.tool.filesystem" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Filesystem,
            }),
            "cns.tool.curator" => Ok(CnsSpan::Tool {
                subsystem: ToolSubsystem::Curator,
            }),
            "cns.inference" => Ok(CnsSpan::Inference),
            "cns.agent_pod" => Ok(CnsSpan::AgentPod),
            "cns.gas" => Ok(CnsSpan::Gas),
            "cns.curation" => Ok(CnsSpan::Curation),
            "cns.heal" => Ok(CnsSpan::SelfHeal),
            "cns.memory.encode" => Ok(CnsSpan::MemoryEncode),
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
        assert_eq!(CnsSpan::AgentPod.to_string(), "cns.agent_pod");
        assert_eq!(CnsSpan::Gas.to_string(), "cns.gas");
        assert_eq!(CnsSpan::Curation.to_string(), "cns.curation");
        assert_eq!(CnsSpan::SelfHeal.to_string(), "cns.heal");
        assert_eq!(CnsSpan::MemoryEncode.to_string(), "cns.memory.encode");
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
            "cns.tool.web_search",
            "cns.tool.condenser",
            "cns.tool.training",
            "cns.tool.replica",
            "cns.tool.research",
            "cns.tool.communication",
            "cns.tool.registry",
            "cns.tool.wallet",
            "cns.tool.media",
            "cns.tool.kanban",
            "cns.tool.memory",
            "cns.tool.companies",
            "cns.tool.docproc",
            "cns.tool.filesystem",
            "cns.tool.curator",
            "cns.inference",
            "cns.agent_pod",
            "cns.gas",
            "cns.curation",
            "cns.heal",
            "cns.memory.encode",
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
            CnsSpan::Tool {
                subsystem: ToolSubsystem::WebSearch,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Condenser,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Training,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Replica,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Research,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Communication,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Registry,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Wallet,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Media,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Kanban,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Memory,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Companies,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Docproc,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Filesystem,
            },
            CnsSpan::Tool {
                subsystem: ToolSubsystem::Curator,
            },
            CnsSpan::Inference,
            CnsSpan::AgentPod,
            CnsSpan::Gas,
            CnsSpan::Curation,
            CnsSpan::SelfHeal,
            CnsSpan::MemoryEncode,
        ];
        // Round-trip test: Display → FromStr → Display must be identity
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
            let parsed: CnsSpan = s
                .parse()
                .expect("Display output must round-trip via FromStr");
            assert_eq!(
                variant, &parsed,
                "{:?} round-trip mismatch: {} -> {:?}",
                variant, s, parsed
            );
        }
        // Assert count matches enum variant count (7 core + 15 specific ToolSubsystem = 22).
        // If this fails, a new CnsSpan variant was added without updating this test.
        assert!(
            all_variants.len() == 22,
            "CNS span exhaustive test should cover all CnsSpan variants, found {} (expected 22)",
            all_variants.len()
        );
    }

    #[test]
    fn tool_subsystem_display_produces_valid_suffix() {
        assert_eq!(ToolSubsystem::WebSearch.as_str(), "web_search");
        assert_eq!(ToolSubsystem::Other.as_str(), "other");
    }
}
