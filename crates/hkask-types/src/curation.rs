//! Curation types for hKask — The Curator and OCAP boundaries
//!
//! Per F-SYN-001 (review `findings/SYNTHESIS.md`): the legacy
//! `OcapCapability::String` variant has been removed. All capabilities
//! are now unforgeable typed brands (`OcapTokenKind`).
//!
//! Per F-SYN-002: `OCAPBoundary::enforced: bool` has been removed.
//! An `OCAPBoundary` *is* enforced by construction; the field was a
//! foot-gun that allowed an unenforceable value of the type.

use serde::{Deserialize, Serialize};

/// CurationDecision — The Curator's evaluation of template outputs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationDecision {
    /// Merge output into codebase
    Merge,
    /// Discard output entirely
    Discard,
    /// Request revision from bot
    Revise,
    /// Insufficient information — revisit later
    ///
    /// Operational criterion: `coherence >= 0.5 && coherence < threshold && drift <= drift_threshold`.
    /// Distinguished from Revise by having non-empty goals (unlike Discard) and
    /// drift within tolerance (unlike Revise which needs immediate changes).
    Defer,
}

impl std::fmt::Display for CurationDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurationDecision::Merge => write!(f, "merge"),
            CurationDecision::Discard => write!(f, "discard"),
            CurationDecision::Revise => write!(f, "revise"),
            CurationDecision::Defer => write!(f, "defer"),
        }
    }
}

impl TryFrom<&str> for CurationDecision {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "merge" => Ok(CurationDecision::Merge),
            "discard" => Ok(CurationDecision::Discard),
            "revise" => Ok(CurationDecision::Revise),
            "defer" => Ok(CurationDecision::Defer),
            _ => Err(format!("invalid curation decision: {s}")),
        }
    }
}

/// Token-based capability kinds for OCAP boundaries.
///
/// The closed set of capability *kinds* in hKask. Each variant maps to
/// a ZST token in `crate::capability::tokens`. Adding a new kind
/// requires editing this enum; the type system then ensures every
/// `OcapCapability` is exhaustively handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcapTokenKind {
    /// Curation authority — ConsolidationToken
    Curation,
    /// Cybernetics authority — CyberneticsToken
    Cybernetics,
    /// Spec curation authority
    SpecCurate,
}

impl std::fmt::Display for OcapTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OcapTokenKind::Curation => "curation",
            OcapTokenKind::Cybernetics => "cybernetics",
            OcapTokenKind::SpecCurate => "spec_curate",
        };
        f.write_str(s)
    }
}

/// Parse an `OcapTokenKind` from its canonical snake_case name.
///
/// Returns `None` for unknown names so callers (e.g. MCP tool
/// handlers) can convert untrusted input into a `ToolSpanGuard` error
/// rather than silently accepting it.
pub fn parse_ocap_token_kind(s: &str) -> Option<OcapTokenKind> {
    match s {
        "curation" => Some(OcapTokenKind::Curation),
        "cybernetics" => Some(OcapTokenKind::Cybernetics),
        "spec_curate" => Some(OcapTokenKind::SpecCurate),
        _ => None,
    }
}

/// Capability identifier — typed brand.
///
/// **Removed in this PR (F-SYN-001):** the previous `String(String)`
/// variant, which let any caller mint any capability
/// (`OCAPBoundary::explicit("memory:write:any-webid")`). All
/// capabilities now flow through this enum's only variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OcapCapability(pub OcapTokenKind);

impl std::fmt::Display for OcapCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// OCAPBoundary — Capability boundary for curation decisions
///
/// \[NORMATIVE\] The Curator must master normative behavior to maintain the OCAP boundary. (P4 — Clear Boundaries).
/// Within the OCAP boundary, The Curator creates non-normative potential.
/// Authority is expressed via `OcapTokenKind` — no token, no authority.
///
/// **Removed in this PR (F-SYN-002):** the `enforced: bool` field.
/// An `OCAPBoundary` is enforced by construction; there is no
/// "unenforceable" value of this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OCAPBoundary {
    /// The capability being bounded (a typed brand).
    pub capability: OcapCapability,
}

impl OCAPBoundary {
    /// Create an enforced boundary with a typed token.
    ///
    /// This is the only constructor. There is no `enforced: false`
    /// variant — an `OCAPBoundary` *is* a boundary.
    pub fn token(kind: OcapTokenKind) -> Self {
        Self {
            capability: OcapCapability(kind),
        }
    }

    /// Parse a typed token from a string, returning `None` for unknown
    /// names. Use this to convert untrusted input (e.g. an MCP tool
    /// request field) into a boundary; reject the request on `None`.
    pub fn parse_token(name: &str) -> Option<Self> {
        parse_ocap_token_kind(name).map(Self::token)
    }
}

fn default_coherence_threshold() -> f64 {
    0.7
}
fn default_drift_threshold() -> f64 {
    0.5
}

/// Configurable thresholds for Curation decisions (spec coherence, drift).
///
/// Moved from `hkask-cns` — curation regulates cybernetics, not the other way around.
/// YAML loading remains in `hkask-cns` (requires `serde_yaml_neo`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CurationThresholdConfig {
    #[serde(default = "default_coherence_threshold")]
    pub coherence_threshold: f64,
    #[serde(default = "default_drift_threshold")]
    pub drift_threshold: f64,
}

impl Default for CurationThresholdConfig {
    fn default() -> Self {
        Self {
            coherence_threshold: default_coherence_threshold(),
            drift_threshold: default_drift_threshold(),
        }
    }
}
