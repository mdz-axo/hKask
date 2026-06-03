//! Curation types for hKask — The Curator and OCAP boundaries

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// CuratorId — Unique identifier for The Curator (single instance)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CuratorId(pub Uuid);

impl CuratorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// The one true Curator — system-wide singleton
    pub fn system() -> Self {
        // Deterministic UUID for the single Curator instance
        // Using a valid UUID v4 hex string
        Self(Uuid::parse_str("c000ca00-0000-4000-8000-000000000001").expect("valid curator UUID"))
    }
}

impl Default for CuratorId {
    fn default() -> Self {
        Self::system()
    }
}

impl std::fmt::Display for CuratorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
// Note: CuratorId kept manual due to special system() method

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
    /// Defer decision (needs more information)
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

/// AuthorityLevel — OCAP authority classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityLevel {
    /// Explicit authority (OCAP-enforced)
    Explicit,
    /// Implicit authority (assumed, not verified)
    Implicit,
    /// Denied authority (explicitly revoked)
    Denied,
}

impl std::fmt::Display for AuthorityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorityLevel::Explicit => write!(f, "explicit"),
            AuthorityLevel::Implicit => write!(f, "implicit"),
            AuthorityLevel::Denied => write!(f, "denied"),
        }
    }
}

/// OCAPBoundary — Capability boundary for curation decisions
///
/// The Curator must master normative behavior to maintain the OCAP boundary.
/// Within the OCAP boundary, The Curator creates non-normative potential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OCAPBoundary {
    /// The capability being bounded
    pub capability: String,
    /// Authority level for this capability
    pub authority: AuthorityLevel,
    /// Whether this boundary is enforced
    pub enforced: bool,
}

impl OCAPBoundary {
    pub(crate) fn new(capability: String, authority: AuthorityLevel) -> Self {
        Self {
            capability,
            authority,
            enforced: true,
        }
    }

    pub fn explicit(capability: String) -> Self {
        Self::new(capability, AuthorityLevel::Explicit)
    }

    pub fn denied(capability: String) -> Self {
        Self::new(capability, AuthorityLevel::Denied)
    }
}
