//! Curation types for hKask — The Curator and OCAP boundaries

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
}

impl std::fmt::Display for CurationDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurationDecision::Merge => write!(f, "merge"),
            CurationDecision::Discard => write!(f, "discard"),
            CurationDecision::Revise => write!(f, "revise"),
        }
    }
}

/// Token-based capability kinds for OCAP boundaries.
///
/// Replaces stringly-typed capability identifiers with typed enum variants.
/// Each variant maps to a ZST token in `crate::capability::tokens`.
#[allow(dead_code)] // TODO: wire when Curation/Cybernetics tokens are adopted by consumers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcapTokenKind {
    /// Curation authority — ConsolidationToken
    Curation,
    /// Cybernetics authority — CyberneticsToken (future)
    Cybernetics,
    /// Spec curation authority
    SpecCurate,
}

/// Capability identifier — typed token or legacy string.
///
/// New code should use `OcapCapability::Token(OcapTokenKind)` instead of
/// `OcapCapability::String(String)`. The string variant exists for backward
/// compatibility with existing persisted records.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcapCapability {
    /// Legacy string-based capability identifier
    #[serde(rename = "string")]
    String(String),
    /// Typed token-based capability identifier
    #[serde(rename = "token")]
    Token(OcapTokenKind),
}

impl std::fmt::Display for OcapCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcapCapability::String(s) => write!(f, "{}", s),
            OcapCapability::Token(kind) => write!(
                f,
                "{}",
                match kind {
                    OcapTokenKind::Curation => "curation",
                    OcapTokenKind::Cybernetics => "cybernetics",
                    OcapTokenKind::SpecCurate => "spec_curate",
                }
            ),
        }
    }
}

/// OCAPBoundary — Capability boundary for curation decisions
///
/// The Curator must master normative behavior to maintain the OCAP boundary.
/// Within the OCAP boundary, The Curator creates non-normative potential.
/// Authority is expressed via CapabilityToken — no token, no authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OCAPBoundary {
    /// The capability being bounded — either a typed token or a legacy string
    pub capability: OcapCapability,
    /// Whether this boundary is enforced
    pub enforced: bool,
}

impl OCAPBoundary {
    /// Create an enforced boundary with a typed token.
    ///
    /// Preferred over `explicit()` for new code — the typed token
    /// prevents stringly-typed capability mismatches.
    pub fn token(kind: OcapTokenKind) -> Self {
        Self {
            capability: OcapCapability::Token(kind),
            enforced: true,
        }
    }

    /// Create an enforced boundary with a legacy string capability.
    ///
    /// Prefer `token()` for new code. `explicit()` exists for backward
    /// compatibility with existing consumers.
    pub fn explicit(capability: String) -> Self {
        Self {
            capability: OcapCapability::String(capability),
            enforced: true,
        }
    }

    pub fn denied(capability: String) -> Self {
        Self {
            capability: OcapCapability::String(capability),
            enforced: false,
        }
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
/// YAML loading remains in `hkask-cns` (requires `serde_yaml`).
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
