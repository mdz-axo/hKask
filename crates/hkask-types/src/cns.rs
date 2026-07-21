//! Core CNS (Cybernetic Nervous System) types for hKask
//!
//! Core spans: cns.tool.*, cns.inference.*, cns.fusion.*, cns.agent_pod.*,
//! cns.gas.*, cns.curation.*, cns.heal.*, cns.memory.encode.*
//!
//! Domain-specific spans have moved to their respective domain crates.
//!
//! `CANONICAL_NAMESPACES` (in `event.rs`) is the single source of truth for
//! **canonical** CNS spans — the essential, ν-event-eligible spans that are
//! `SpanNamespace`-validated, `SpanCategory`-categorized, and loop-connected.
//! The `cns.*` prefix is reserved for canonical spans: every `cns.*` tracing
//! target MUST be registered in `CANONICAL_NAMESPACES`. **Performative**
//! telemetry (per PRINCIPLES §9.1) uses `hkask.*` tracing targets (e.g.
//! `hkask.cli`, `hkask.training.job.submit`), NOT `cns.*`; those are observability
//! logs, not loop variables, and `SpanNamespace::new` rejects them.

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
pub struct LedgerHealth {
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

// ── RegulationSpan — Core CNS Span Identifiers ────────────────────────────────────

/// Core CNS span identifiers — spans that are constructed in 2+ crates from
/// different dependency domains (the "cross-cutting concern" test).
///
/// Domain-specific spans (wallet, federation, contracts, QA, metrics, deploy,
/// backup, ACP, curator, etc.) have moved to their respective domain crates
/// as enums implementing [`ObservableSpan`](crate::ObservableSpan).
///
/// `CANONICAL_NAMESPACES` (in `event.rs`) is the single source of truth for
/// **canonical** CNS spans — essential spans that are `SpanNamespace`-validated,
/// `SpanCategory`-categorized, and connected to a cybernetic loop. The `cns.*`
/// prefix is reserved for these canonical spans: every `cns.*` tracing target
/// MUST be registered. Per PRINCIPLES §9.1, performative telemetry uses
/// `hkask.*` tracing targets (not `cns.*`); those are observability logs, not
/// loop variables, and `SpanNamespace::new` rejects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegulationSpan {
    /// Tool invocation span. Subsystem tracks which MCP server emitted the span.
    Tool { subsystem: ToolSubsystem },
    /// LLM inference request/response.
    Inference,
    /// Multi-model fusion deliberation (panel dispatch + judge orchestration).
    /// Distinct from `Inference` so fusion rounds, convergence, and panel/judge
    /// cost are independently observable (PRINCIPLES.md §9.1).
    Fusion,
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

/// Subsystem identifier for `RegulationSpan::Tool` — which MCP server emitted the span.
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

impl RegulationSpan {
    /// Emit a typed CNS span event through the `tracing` infrastructure.
    ///
    /// Enforces the canonical CNS emission convention (PRINCIPLES.md §9.2):
    /// - `target` = `"cns"` root namespace (full domain in `reg_domain` field)
    /// - `reg_domain` = `self.as_str()` (e.g. `"cns.tool.media"`)
    /// - `operation` = the verb describing what occurred (e.g. `"invoked"`)
    /// - message = `"CNS"` (required for downstream ν-event parsing)
    ///
    /// Callers that need additional structured fields can attach them by
    /// entering a child [`mod@tracing::span`] before calling `emit()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use hkask_types::cns::{RegulationSpan, ToolSubsystem};
    ///
    /// RegulationSpan::Tool { subsystem: ToolSubsystem::Media }
    ///     .emit("invoked");
    /// ```
    pub fn emit(&self, operation: &str) {
        tracing::info!(
            target: "reg",
            reg_domain = %self.as_str(),
            operation = %operation,
            "CNS",
        );
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid RegulationSpan variant
    /// post: returns the canonical namespace string (e.g. "cns.tool.web_search"); output matches CANONICAL_NAMESPACES byte-for-byte
    ///
    /// This output must match ν-event serialization strings byte-for-byte
    /// (P8 — Semantic Grounding).
    pub fn as_str(&self) -> &'static str {
        match self {
            RegulationSpan::Tool { subsystem } => match subsystem {
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
            RegulationSpan::Inference => "cns.inference",
            RegulationSpan::Fusion => "cns.fusion",
            RegulationSpan::AgentPod => "cns.agent_pod",
            RegulationSpan::Gas => "cns.gas",
            RegulationSpan::Curation => "cns.curation",
            RegulationSpan::SelfHeal => "cns.heal",
            RegulationSpan::MemoryEncode => "cns.memory.encode",
        }
    }
}

impl std::fmt::Display for RegulationSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl crate::observable_span::ObservableSpan for RegulationSpan {
    fn as_str(&self) -> &'static str {
        RegulationSpan::as_str(self)
    }

    fn emit(&self, operation: &str) {
        RegulationSpan::emit(self, operation);
    }
}

impl std::str::FromStr for RegulationSpan {
    type Err = ();

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  s is a string matching a canonical RegulationSpan namespace
    /// post: returns Ok(RegulationSpan) for canonical strings; Err(()) for unknown strings
    ///
    /// Only strings matching canonical `RegulationSpan` namespaces parse
    /// successfully. Unknown strings return `Err(())`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cns.tool" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Other,
            }),
            "cns.tool.web_search" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::WebSearch,
            }),
            "cns.tool.condenser" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Condenser,
            }),
            "cns.tool.training" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Training,
            }),
            "cns.tool.replica" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Replica,
            }),
            "cns.tool.research" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Research,
            }),
            "cns.tool.communication" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Communication,
            }),
            "cns.tool.registry" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Registry,
            }),
            "cns.tool.wallet" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Wallet,
            }),
            "cns.tool.media" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Media,
            }),
            "cns.tool.kanban" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Kanban,
            }),
            "cns.tool.memory" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Memory,
            }),
            "cns.tool.companies" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Companies,
            }),
            "cns.tool.docproc" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Docproc,
            }),
            "cns.tool.filesystem" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Filesystem,
            }),
            "cns.tool.curator" => Ok(RegulationSpan::Tool {
                subsystem: ToolSubsystem::Curator,
            }),
            "cns.inference" => Ok(RegulationSpan::Inference),
            "cns.fusion" => Ok(RegulationSpan::Fusion),
            "cns.agent_pod" => Ok(RegulationSpan::AgentPod),
            "cns.gas" => Ok(RegulationSpan::Gas),
            "cns.curation" => Ok(RegulationSpan::Curation),
            "cns.heal" => Ok(RegulationSpan::SelfHeal),
            "cns.memory.encode" => Ok(RegulationSpan::MemoryEncode),
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
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Other
            }
            .to_string(),
            "cns.tool"
        );
        assert_eq!(RegulationSpan::Inference.to_string(), "cns.inference");
        assert_eq!(RegulationSpan::AgentPod.to_string(), "cns.agent_pod");
        assert_eq!(RegulationSpan::Gas.to_string(), "cns.gas");
        assert_eq!(RegulationSpan::Curation.to_string(), "cns.curation");
        assert_eq!(RegulationSpan::SelfHeal.to_string(), "cns.heal");
        assert_eq!(RegulationSpan::MemoryEncode.to_string(), "cns.memory.encode");
    }

    #[test]
    fn cnsspan_from_str_rejects_invalid() {
        assert!(RegulationSpan::from_str("cns.nonexistent").is_err());
        assert!(RegulationSpan::from_str("invalid").is_err());
        assert!(RegulationSpan::from_str("").is_err());
        assert!(RegulationSpan::from_str("tool").is_err()); // short form not supported
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
            let span: RegulationSpan = s.parse().expect("should parse");
            assert_eq!(span.to_string(), s, "Display should match input");
        }
    }

    #[test]
    fn cnsspan_tool_subsystem_produces_correct_string() {
        assert_eq!(
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::WebSearch
            }
            .to_string(),
            "cns.tool.web_search"
        );
        assert_eq!(
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Other
            }
            .to_string(),
            "cns.tool"
        );
    }

    #[test]
    fn cnsspan_exhaustive_match_covers_all_canonical() {
        let all_variants = vec![
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Other,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::WebSearch,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Condenser,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Training,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Replica,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Research,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Communication,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Registry,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Wallet,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Media,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Kanban,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Memory,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Companies,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Docproc,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Filesystem,
            },
            RegulationSpan::Tool {
                subsystem: ToolSubsystem::Curator,
            },
            RegulationSpan::Inference,
            RegulationSpan::Fusion,
            RegulationSpan::AgentPod,
            RegulationSpan::Gas,
            RegulationSpan::Curation,
            RegulationSpan::SelfHeal,
            RegulationSpan::MemoryEncode,
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
            let parsed: RegulationSpan = s
                .parse()
                .expect("Display output must round-trip via FromStr");
            assert_eq!(
                variant, &parsed,
                "{:?} round-trip mismatch: {} -> {:?}",
                variant, s, parsed
            );
        }
        // Assert count matches enum variant count (8 core + 15 specific ToolSubsystem = 23).
        // If this fails, a new RegulationSpan variant was added without updating this test.
        assert!(
            all_variants.len() == 23,
            "CNS span exhaustive test should cover all RegulationSpan variants, found {} (expected 23)",
            all_variants.len()
        );
    }

    #[test]
    fn tool_subsystem_display_produces_valid_suffix() {
        assert_eq!(ToolSubsystem::WebSearch.as_str(), "web_search");
        assert_eq!(ToolSubsystem::Other.as_str(), "other");
    }
}
