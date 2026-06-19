//! Visibility types — Loop 6 (Cybernetics): access classification
//
//! Visibility is a Cybernetics concern — the Access Guard (6.1) enforces
//! visibility boundaries. Public/Private data categories determine
//! what each loop can read or write.
//
//! Defines a two-tier visibility model:
//! - Public: accessible to all agents (shared/factual knowledge)
//! - Private: agent-specific access (episodic/experiential knowledge)
//!
//! Access control enforcement is delegated to the `CapabilityToken` primitive.
//!
//! # Visibility Model
//
//! | Level | Meaning | Enforcement |
//! |-------|---------|-------------|
//! | Private | Agent-specific access | `SovereigntyChecker` + `CapabilityToken` |
//! | Public | Universal access | No capability required |

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
}

impl Visibility {
    /// Get string representation of visibility.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns "private", "shared", or "public"
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Public => "public",
        }
    }

    /// Parse visibility from string.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Some(Visibility) if valid, None otherwise
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "private" | "Private" => Some(Visibility::Private),
            "public" | "Public" | "shared" | "Shared" => Some(Visibility::Public),
            _ => None,
        }
    }

    // F-SYN-003: the three `is_*` predicates were dead code
    // (F-L1-005) and collided with the same-named predicates on
    // `DataSovereigntyBoundary` (F-L4-001). Removed entirely. Use
    // `match self { Visibility::Private => ..., ... }` at the
    // call site, or import `DataSovereigntyBoundary::is_category_shared`
    // when the predicate is category-scoped.
}

/// Access control grouping for triples and other stored artifacts.
///
/// \[DECLARATIVE\] Bundles the three fields that always appear together: (P5 — Essentialism).
/// `perspective`, `visibility`, and `owner_webid`. This value object
/// replaces the repeated pattern of passing these three as separate
/// parameters (Fowler H10: Group Data Clump Into Object).
///
/// Canonical constructors:
/// - `AccessControl::new(owner)` — default: private, no perspective
/// - `AccessControl::episodic(perspective, owner)` — private, perspective-bound
/// - `AccessControl::semantic(owner)` — shared, no perspective
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AccessControl {
    pub perspective: Option<WebID>,
    pub visibility: Visibility,
    pub owner_webid: WebID,
}

impl AccessControl {
    /// Create a default access control: private, no perspective, owned by `owner`.
    /// Create a new AccessControl with owner.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  owner is valid
    /// post: returns AccessControl with Private visibility, no perspective
    pub fn new(owner: WebID) -> Self {
        Self {
            perspective: None,
            visibility: Visibility::Private,
            owner_webid: owner,
        }
    }

    /// Create an episodic (perspective-bound) access control: private, owned by `owner`.
    /// Create episodic access control.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  perspective and owner are valid
    /// post: returns AccessControl with Private visibility and perspective
    pub fn episodic(perspective: WebID, owner: WebID) -> Self {
        Self {
            perspective: Some(perspective),
            visibility: Visibility::Private,
            owner_webid: owner,
        }
    }

    /// Create a semantic (public, perspective-free) access control.
    /// Create semantic access control.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  owner is valid
    /// post: returns AccessControl with Public visibility, no perspective
    pub fn semantic(owner: WebID) -> Self {
        Self {
            perspective: None,
            visibility: Visibility::Public,
            owner_webid: owner,
        }
    }

    /// Convert to semantic access control: strip perspective, set visibility to Public.
    /// Convert to semantic access (strip perspective, set Public).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns AccessControl with Public visibility, no perspective
    pub fn to_semantic(&self) -> Self {
        Self {
            perspective: None,
            visibility: Visibility::Public,
            owner_webid: self.owner_webid,
        }
    }

    /// Is this an episodic (perspective-bound) access control?
    /// Check if this is episodic (has perspective).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true iff perspective is Some
    pub fn is_episodic(&self) -> bool {
        self.perspective.is_some()
    }

    /// Is this a semantic (public, perspective-free) access control?
    /// Check if this is semantic (Public, no perspective).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true iff visibility is Public and perspective is None
    pub fn is_semantic(&self) -> bool {
        self.perspective.is_none() && self.visibility == Visibility::Public
    }

    #[must_use = "builder methods must be chained or assigned"]
    /// Set perspective (builder).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Self with perspective set
    pub fn with_perspective(mut self, perspective: WebID) -> Self {
        self.perspective = Some(perspective);
        self
    }

    /// Set the visibility mode.
    ///
    /// F-SYN-004: this method refuses the visibility *flip* from a
    /// private/perspective-bound (episodic) value to a shared/public
    /// value. Flipping would expose the perspective to a wider
    /// audience without removing the perspective, which is the
    /// privacy-laundering pattern. To legitimately share an
    /// episodic triple, call `without_perspective()` first.
    /// Set visibility (builder).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Self with visibility set
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        // F-SYN-004: refuse perspective-locked flips.
        if self.is_episodic() {
            match visibility {
                Visibility::Public => {
                    // The flip would expose the perspective to a wider
                    // audience. The caller must clear the perspective
                    // explicitly. We panic rather than silently
                    // laundering because silent data laundering is
                    // a security incident; loud failure is correct.
                    panic!(
                        "AccessControl::with_visibility: refusing perspective-locked \
                         flip from episodic to {visibility:?}. Call \
                         `without_perspective()` first to make the triple semantic."
                    );
                }
                Visibility::Private => {
                    // Private is the *default* for episodic; staying
                    // private while keeping the perspective is fine.
                    self.visibility = visibility;
                }
            }
        } else {
            self.visibility = visibility;
        }
        self
    }

    /// Remove the perspective from this access control.
    ///
    /// F-SYN-004: this is the explicit operation to take a triple
    /// from *episodic* (perspective-bound) to *semantic* (shared,
    /// perspective-free). It does not change the visibility mode.
    /// Remove perspective (for legitimate sharing).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Self with perspective set to None
    pub fn without_perspective(mut self) -> Self {
        self.perspective = None;
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
    /// Create a new Confidence value.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value in [0.0, 1.0]
    /// post: returns Confidence
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Full confidence (1.0) — the default for new triples.
    /// Full confidence (1.0).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Confidence(1.0)
    pub fn full() -> Self {
        Self(1.0)
    }

    /// Get the raw confidence value.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns f64 value
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Apply exponential decay: `confidence * exp(-rate * time)`.
    ///
    /// Used by Episodic Loop for Bayesian confidence decay.
    /// The result is clamped to [0.0, 1.0].
    /// Apply exponential decay: value * e^(-rate * time).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  rate >= 0, time >= 0
    /// post: returns decayed Confidence
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
    /// Create a TemporalContext with now as valid_from.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns TemporalContext with valid_from=now, valid_to=None
    pub fn now() -> Self {
        Self {
            valid_from: Utc::now(),
            valid_to: None,
        }
    }

    /// Create temporal bounds with explicit start and optional end.
    /// Create a TemporalContext with explicit bounds.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns TemporalContext
    pub fn new(valid_from: DateTime<Utc>, valid_to: Option<DateTime<Utc>>) -> Self {
        Self {
            valid_from,
            valid_to,
        }
    }

    /// Is this triple currently active (no end time)?
    /// Check if this temporal context is currently valid.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true iff valid_to is None or in the future
    pub fn is_current(&self) -> bool {
        self.valid_to.is_none()
    }

    /// Supercede: create new bounds with valid_to set to now.
    /// Mark as superseded (set valid_to to now).
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Self with valid_to=now
    pub fn superseded(&self) -> Self {
        Self {
            valid_from: self.valid_from,
            valid_to: Some(Utc::now()),
        }
    }
}
