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

/// OCAPBoundary — Capability boundary for curation decisions
///
/// The Curator must master normative behavior to maintain the OCAP boundary.
/// Within the OCAP boundary, The Curator creates non-normative potential.
/// Authority is expressed via CapabilityToken — no token, no authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OCAPBoundary {
    /// The capability being bounded
    pub capability: String,
    /// Whether this boundary is enforced
    pub enforced: bool,
}

impl OCAPBoundary {
    pub fn explicit(capability: String) -> Self {
        Self {
            capability,
            enforced: true,
        }
    }

    pub fn denied(capability: String) -> Self {
        Self {
            capability,
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
