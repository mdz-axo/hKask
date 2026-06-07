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
/// Replaces stringly-typed capability identifiers with typed enum variants.
/// Each variant maps to a ZST token in `crate::capability::tokens`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcapTokenKind {
    /// Curation authority — ConsolidationToken
    Curation,
    /// Cybernetics authority — CyberneticsToken
    Cybernetics, // (future — no production callers yet)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // OcapTokenKind — behavioural properties
    // ------------------------------------------------------------------

    /// All three variants exist and are constructible.
    #[test]
    fn ocap_token_kind_all_variants_exist() {
        let _ = OcapTokenKind::Curation;
        let _ = OcapTokenKind::Cybernetics;
        let _ = OcapTokenKind::SpecCurate;
    }

    /// Copy semantics: assigning to a new binding does not move.
    #[test]
    fn ocap_token_kind_is_copy() {
        let a = OcapTokenKind::Curation;
        let b = a; // copy, not move
        let _c = a; // still usable — proves Copy
        assert_eq!(a, b);
    }

    /// Clone produces an equal value (trivial for Copy types, but verifies the derive).
    #[test]
    fn ocap_token_kind_clone_is_equal() {
        let a = OcapTokenKind::SpecCurate;
        assert_eq!(a.clone(), a);
    }

    /// Debug representation is meaningful and distinguishable.
    #[test]
    fn ocap_token_kind_debug_distinguishes_variants() {
        let debugs = [
            format!("{:?}", OcapTokenKind::Curation),
            format!("{:?}", OcapTokenKind::Cybernetics),
            format!("{:?}", OcapTokenKind::SpecCurate),
        ];
        assert_eq!(debugs, ["Curation", "Cybernetics", "SpecCurate"]);
    }

    /// PartialEq: same variants equal, different variants not equal.
    #[test]
    fn ocap_token_kind_equality_semantics() {
        assert_eq!(OcapTokenKind::Curation, OcapTokenKind::Curation);
        assert_ne!(OcapTokenKind::Curation, OcapTokenKind::Cybernetics);
        assert_ne!(OcapTokenKind::Cybernetics, OcapTokenKind::SpecCurate);
    }

    /// Hash: equal values produce equal hashes (required for HashMap keys).
    #[test]
    fn ocap_token_kind_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let hash = |v: &OcapTokenKind| {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        };
        assert_eq!(
            hash(&OcapTokenKind::Curation),
            hash(&OcapTokenKind::Curation)
        );
        assert_ne!(
            hash(&OcapTokenKind::Curation),
            hash(&OcapTokenKind::Cybernetics)
        );
    }

    /// Serde roundtrip: each variant serializes to snake_case JSON and
    /// deserializes back to the same variant.
    #[test]
    fn ocap_token_kind_serde_roundtrip() {
        for (variant, expected_json) in [
            (OcapTokenKind::Curation, "\"curation\""),
            (OcapTokenKind::Cybernetics, "\"cybernetics\""),
            (OcapTokenKind::SpecCurate, "\"spec_curate\""),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json);
            let back: OcapTokenKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // ------------------------------------------------------------------
    // OcapCapability — behavioural properties
    // ------------------------------------------------------------------

    /// String variant holds and displays the inner string.
    #[test]
    fn ocap_capability_string_displays_inner_value() {
        let cap = OcapCapability::String("curation".into());
        assert_eq!(cap.to_string(), "curation");
        let cap2 = OcapCapability::String("custom_cap".into());
        assert_eq!(cap2.to_string(), "custom_cap");
    }

    /// Token variant Display maps each kind to its canonical name.
    #[test]
    fn ocap_capability_token_display_mapping() {
        assert_eq!(
            OcapCapability::Token(OcapTokenKind::Curation).to_string(),
            "curation"
        );
        assert_eq!(
            OcapCapability::Token(OcapTokenKind::Cybernetics).to_string(),
            "cybernetics"
        );
        assert_eq!(
            OcapCapability::Token(OcapTokenKind::SpecCurate).to_string(),
            "spec_curate"
        );
    }

    /// PartialEq: same variant + same inner value → equal;
    /// different inner values → not equal;
    /// different capability kinds → not equal.
    #[test]
    fn ocap_capability_equality_semantics() {
        // Same string value → equal
        assert_eq!(
            OcapCapability::String("curation".into()),
            OcapCapability::String("curation".into()),
        );
        // Different string values → not equal
        assert_ne!(
            OcapCapability::String("curation".into()),
            OcapCapability::String("cybernetics".into()),
        );
        // Same token kind → equal
        assert_eq!(
            OcapCapability::Token(OcapTokenKind::Curation),
            OcapCapability::Token(OcapTokenKind::Curation),
        );
        // Different token kinds → not equal
        assert_ne!(
            OcapCapability::Token(OcapTokenKind::Curation),
            OcapCapability::Token(OcapTokenKind::Cybernetics),
        );
        // String vs Token with same display name → not equal (different variants)
        assert_ne!(
            OcapCapability::String("curation".into()),
            OcapCapability::Token(OcapTokenKind::Curation),
        );
    }

    /// Serde roundtrip for both OcapCapability variants.
    #[test]
    fn ocap_capability_serde_roundtrip() {
        let string_cap = OcapCapability::String("curation".into());
        let json = serde_json::to_string(&string_cap).unwrap();
        assert_eq!(json, "{\"string\":\"curation\"}");
        let back: OcapCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, string_cap);

        let token_cap = OcapCapability::Token(OcapTokenKind::Cybernetics);
        let json = serde_json::to_string(&token_cap).unwrap();
        assert_eq!(json, "{\"token\":\"cybernetics\"}");
        let back: OcapCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, token_cap);
    }

    // ------------------------------------------------------------------
    // OCAPBoundary — behavioural properties
    // ------------------------------------------------------------------

    /// token() creates an enforced boundary with the given token kind.
    #[test]
    fn ocap_boundary_token_creates_enforced() {
        let b = OCAPBoundary::token(OcapTokenKind::Curation);
        assert_eq!(b.capability, OcapCapability::Token(OcapTokenKind::Curation));
        assert!(b.enforced);

        let b = OCAPBoundary::token(OcapTokenKind::SpecCurate);
        assert_eq!(
            b.capability,
            OcapCapability::Token(OcapTokenKind::SpecCurate)
        );
        assert!(b.enforced);
    }

    /// explicit() creates an enforced boundary with a String capability.
    #[test]
    fn ocap_boundary_explicit_creates_enforced() {
        let b = OCAPBoundary::explicit("curation".into());
        assert_eq!(b.capability, OcapCapability::String("curation".into()));
        assert!(b.enforced);
    }

    /// denied() creates a non-enforced boundary with a String capability.
    #[test]
    fn ocap_boundary_denied_creates_unenforced() {
        let b = OCAPBoundary::denied("curation".into());
        assert_eq!(b.capability, OcapCapability::String("curation".into()));
        assert!(!b.enforced);
    }

    /// Token and explicit boundaries with the same display name are not equal
    /// — they differ in capability variant.
    #[test]
    fn ocap_boundary_token_and_explicit_are_not_equal() {
        let token_boundary = OCAPBoundary::token(OcapTokenKind::Curation);
        let explicit_boundary = OCAPBoundary::explicit("curation".into());
        assert_ne!(token_boundary, explicit_boundary);
    }

    /// Serde roundtrip: token and explicit boundaries serialize/deserialize correctly.
    #[test]
    fn ocap_boundary_serde_roundtrip() {
        // Token boundary
        let token_b = OCAPBoundary::token(OcapTokenKind::Curation);
        let json = serde_json::to_string(&token_b).unwrap();
        let back: OCAPBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, token_b);

        // Explicit boundary (enforced)
        let explicit_b = OCAPBoundary::explicit("curation".into());
        let json = serde_json::to_string(&explicit_b).unwrap();
        let back: OCAPBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, explicit_b);

        // Denied boundary (not enforced)
        let denied_b = OCAPBoundary::denied("curation".into());
        let json = serde_json::to_string(&denied_b).unwrap();
        let back: OCAPBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, denied_b);
        assert!(!back.enforced);
    }

    // ------------------------------------------------------------------
    // CurationThresholdConfig — behavioural properties
    // ------------------------------------------------------------------

    /// Default has coherence_threshold=0.7 and drift_threshold=0.5.
    #[test]
    fn curation_threshold_config_default_values() {
        let cfg = CurationThresholdConfig::default();
        assert!((cfg.coherence_threshold - 0.7).abs() < f64::EPSILON);
        assert!((cfg.drift_threshold - 0.5).abs() < f64::EPSILON);
    }

    /// Custom config can be constructed with different values.
    #[test]
    fn curation_threshold_config_custom_values() {
        let cfg = CurationThresholdConfig {
            coherence_threshold: 0.9,
            drift_threshold: 0.3,
        };
        assert!((cfg.coherence_threshold - 0.9).abs() < f64::EPSILON);
        assert!((cfg.drift_threshold - 0.3).abs() < f64::EPSILON);
    }

    /// Serde deserialization from empty object `{}` yields defaults.
    #[test]
    fn curation_threshold_config_empty_object_yields_defaults() {
        let cfg: CurationThresholdConfig = serde_json::from_str("{}").unwrap();
        assert!((cfg.coherence_threshold - 0.7).abs() < f64::EPSILON);
        assert!((cfg.drift_threshold - 0.5).abs() < f64::EPSILON);
    }

    /// Partial override: providing only one field falls back to default for the other.
    #[test]
    fn curation_threshold_config_partial_override() {
        let cfg: CurationThresholdConfig =
            serde_json::from_str("{\"coherence_threshold\":0.85}").unwrap();
        assert!((cfg.coherence_threshold - 0.85).abs() < f64::EPSILON);
        assert!((cfg.drift_threshold - 0.5).abs() < f64::EPSILON); // default
    }
}
