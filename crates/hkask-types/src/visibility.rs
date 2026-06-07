//! Visibility types — Loop 6 (Cybernetics): access classification
//
//! Visibility is a Cybernetics concern — the Access Guard (6.1) enforces
//! visibility boundaries. Public/Shared/Sovereign data categories determine
//! what each loop can read or write.
//
//! Defines the three-tier visibility model for artifacts within hKask.
//! Access control enforcement is delegated to the `CapabilityToken` primitive
//! in the `capability` module (ADR-022-T08: single capability primitive).
//
//! # Visibility Model
//
//! | Level | Meaning | Enforcement |
//! |-------|---------|-------------|
//! | Private | Owner-only access | `SovereigntyChecker` + `CapabilityToken` |
//! | Public | Universal access | No capability required |
//! | Shared | Capability-gated access | `CapabilityToken` with correct resource/action |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::WebID;

/// Visibility level for artifacts
/// Loop: Cybernetics
///
/// Classification enum used by `SovereigntyChecker`, `Goal`, and
/// `Triple` to categorize data accessibility. The actual enforcement
/// is delegated to the `CapabilityToken` primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Private,
    Public,
    Shared,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Public => "public",
            Visibility::Shared => "shared",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "private" | "Private" => Some(Visibility::Private),
            "public" | "Public" => Some(Visibility::Public),
            "shared" | "Shared" => Some(Visibility::Shared),
            _ => None,
        }
    }

    #[allow(dead_code)] // reserved for future crate-internal use
    pub(crate) fn is_private(&self) -> bool {
        matches!(self, Visibility::Private)
    }

    #[allow(dead_code)] // reserved for future crate-internal use
    pub(crate) fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, Visibility::Shared)
    }
}

/// Access control grouping for triples and other stored artifacts.
///
/// Bundles the three fields that always appear together:
/// `perspective`, `visibility`, and `owner_webid`. This value object
/// replaces the repeated pattern of passing these three as separate
/// parameters (Fowler H10: Group Data Clump Into Object).
///
/// Canonical constructors:
/// - `AccessControl::new(owner)` — default: private, no perspective
/// - `AccessControl::episodic(perspective, owner)` — private, perspective-bound
/// - `AccessControl::semantic(owner)` — shared, no perspective
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessControl {
    pub perspective: Option<WebID>,
    pub visibility: Visibility,
    pub owner_webid: WebID,
}

impl AccessControl {
    /// Create a default access control: private, no perspective, owned by `owner`.
    pub fn new(owner: WebID) -> Self {
        Self {
            perspective: None,
            visibility: Visibility::Private,
            owner_webid: owner,
        }
    }

    /// Create an episodic (perspective-bound) access control: private, owned by `owner`.
    pub fn episodic(perspective: WebID, owner: WebID) -> Self {
        Self {
            perspective: Some(perspective),
            visibility: Visibility::Private,
            owner_webid: owner,
        }
    }

    /// Create a semantic (shared, perspective-free) access control.
    pub fn semantic(owner: WebID) -> Self {
        Self {
            perspective: None,
            visibility: Visibility::Shared,
            owner_webid: owner,
        }
    }

    /// Convert to semantic access control: strip perspective, set visibility to Shared.
    pub fn to_semantic(&self) -> Self {
        Self {
            perspective: None,
            visibility: Visibility::Shared,
            owner_webid: self.owner_webid,
        }
    }

    /// Is this an episodic (perspective-bound) access control?
    pub fn is_episodic(&self) -> bool {
        self.perspective.is_some()
    }

    /// Is this a semantic (shared, perspective-free) access control?
    pub fn is_semantic(&self) -> bool {
        self.perspective.is_none() && self.visibility == Visibility::Shared
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_perspective(mut self, perspective: WebID) -> Self {
        self.perspective = Some(perspective);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }
}

/// Confidence value for triples, clamped to [0.0, 1.0].
///
/// Replaces bare `f64` confidence values with a type-safe newtype
/// (Fowler: Replace Primitive with Object). Confidence is always
/// within [0.0, 1.0] — values outside this range are clamped on
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Confidence(f64);

impl Confidence {
    /// Create a confidence value, clamping to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Full confidence (1.0) — the default for new triples.
    pub fn full() -> Self {
        Self(1.0)
    }

    /// Zero confidence (0.0).
    #[allow(dead_code)] // reserved for future crate-internal use
    pub(crate) fn zero() -> Self {
        Self(0.0)
    }

    /// Get the raw f64 value.
    #[allow(dead_code)] // reserved for future crate-internal use
    pub(crate) fn into_inner(self) -> f64 {
        self.0
    }

    /// Get the raw f64 value by reference.
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Apply exponential decay: `confidence * exp(-rate * time)`.
    ///
    /// Used by Episodic Loop for Bayesian confidence decay.
    /// The result is clamped to [0.0, 1.0].
    pub fn decay(&self, rate: f64, time: f64) -> Self {
        Self((self.0 * (-rate * time).exp()).clamp(0.0, 1.0))
    }
}

impl From<f64> for Confidence {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<Confidence> for f64 {
    fn from(c: Confidence) -> Self {
        c.0
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

/// Temporal bounds for bitemporal triples and events.
///
/// Groups `valid_from` and `valid_to` into a single value object,
/// replacing the repeated pattern of passing these as separate parameters
/// (Fowler H10: Group Data Clump Into Object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalBounds {
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

impl TemporalBounds {
    /// Create temporal bounds starting now with no end (current/active).
    pub fn now() -> Self {
        Self {
            valid_from: Utc::now(),
            valid_to: None,
        }
    }

    /// Create temporal bounds with explicit start and optional end.
    pub fn new(valid_from: DateTime<Utc>, valid_to: Option<DateTime<Utc>>) -> Self {
        Self {
            valid_from,
            valid_to,
        }
    }

    /// Is this triple currently active (no end time)?
    pub fn is_current(&self) -> bool {
        self.valid_to.is_none()
    }

    /// Supercede: create new bounds with valid_to set to now.
    pub fn superseded(&self) -> Self {
        Self {
            valid_from: self.valid_from,
            valid_to: Some(Utc::now()),
        }
    }
}
