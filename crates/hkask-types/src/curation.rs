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
/// The Curator must master normative behavior to maintain the OCAP boundary.
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

    /// Display maps each kind to its canonical snake_case name.
    #[test]
    fn ocap_token_kind_display_matches_canonical_name() {
        assert_eq!(OcapTokenKind::Curation.to_string(), "curation");
        assert_eq!(OcapTokenKind::Cybernetics.to_string(), "cybernetics");
        assert_eq!(OcapTokenKind::SpecCurate.to_string(), "spec_curate");
    }

    /// `parse_ocap_token_kind` is the inverse of `Display` for every variant.
    #[test]
    fn parse_ocap_token_kind_round_trips_display() {
        for kind in [
            OcapTokenKind::Curation,
            OcapTokenKind::Cybernetics,
            OcapTokenKind::SpecCurate,
        ] {
            let s = kind.to_string();
            assert_eq!(parse_ocap_token_kind(&s), Some(kind));
        }
    }

    /// `parse_ocap_token_kind` rejects arbitrary attacker input.
    /// This is the F-SYN-001 attack scenario: a string that
    /// *would have* minted any capability under the old `String`
    /// variant now fails to parse, returning `None`.
    #[test]
    fn parse_ocap_token_kind_rejects_attacker_string() {
        for attack in [
            "memory:write:any-webid",
            "memory:write",
            "*",
            "",
            "Memory:Write", // case-sensitive
            "spec_curate ", // trailing space
            " spec_curate", // leading space
        ] {
            assert_eq!(
                parse_ocap_token_kind(attack),
                None,
                "attack input `{attack}` must not parse"
            );
        }
    }

    // ------------------------------------------------------------------
    // OcapCapability — behavioural properties
    // ------------------------------------------------------------------

    /// `OcapCapability` is a single-variant newtype; equality and Display
    /// are derived from the inner `OcapTokenKind`.
    #[test]
    fn ocap_capability_equality_and_display() {
        let a = OcapCapability(OcapTokenKind::Curation);
        let b = OcapCapability(OcapTokenKind::Curation);
        let c = OcapCapability(OcapTokenKind::SpecCurate);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.to_string(), "curation");
        assert_eq!(c.to_string(), "spec_curate");
    }

    /// Serde roundtrip: a token-based capability serializes to its
    /// canonical snake_case name (transparent).
    #[test]
    fn ocap_capability_serde_roundtrip() {
        let cap = OcapCapability(OcapTokenKind::Cybernetics);
        let json = serde_json::to_string(&cap).unwrap();
        assert_eq!(json, "\"cybernetics\"");
        let back: OcapCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cap);
    }

    // ------------------------------------------------------------------
    // OCAPBoundary — behavioural properties
    // ------------------------------------------------------------------

    /// `token()` creates a boundary with the given token kind.
    #[test]
    fn ocap_boundary_token_creates() {
        let b = OCAPBoundary::token(OcapTokenKind::Curation);
        assert_eq!(b.capability, OcapCapability(OcapTokenKind::Curation));

        let b = OCAPBoundary::token(OcapTokenKind::SpecCurate);
        assert_eq!(b.capability, OcapCapability(OcapTokenKind::SpecCurate));
    }

    /// `parse_token` round-trips every known name.
    #[test]
    fn ocap_boundary_parse_token_known_names() {
        assert_eq!(
            OCAPBoundary::parse_token("curation"),
            Some(OCAPBoundary::token(OcapTokenKind::Curation))
        );
        assert_eq!(
            OCAPBoundary::parse_token("cybernetics"),
            Some(OCAPBoundary::token(OcapTokenKind::Cybernetics))
        );
        assert_eq!(
            OCAPBoundary::parse_token("spec_curate"),
            Some(OCAPBoundary::token(OcapTokenKind::SpecCurate))
        );
    }

    /// `parse_token` rejects unknown names (F-SYN-001 attack surface).
    #[test]
    fn ocap_boundary_parse_token_rejects_unknown() {
        for s in ["", "unknown", "memory:write:any-webid", "SpecCurate"] {
            assert_eq!(
                OCAPBoundary::parse_token(s),
                None,
                "unknown input `{s}` must not parse"
            );
        }
    }

    /// Two boundaries with the same capability are equal.
    #[test]
    fn ocap_boundary_equality() {
        let a = OCAPBoundary::token(OcapTokenKind::Curation);
        let b = OCAPBoundary::token(OcapTokenKind::Curation);
        let c = OCAPBoundary::token(OcapTokenKind::SpecCurate);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// Serde roundtrip: a boundary with a token capability
    /// serializes and deserializes correctly.
    #[test]
    fn ocap_boundary_serde_roundtrip() {
        let b = OCAPBoundary::token(OcapTokenKind::Curation);
        let json = serde_json::to_string(&b).unwrap();
        let back: OCAPBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }

    /// **F-SYN-001 red→green invariant:** an attacker can no longer
    /// construct an `OCAPBoundary` with a forged capability. The old
    /// `OCAPBoundary::explicit(String)` and
    /// `OcapCapability::String(String)` paths are gone. This test
    /// asserts the attack fails: `parse_token` returns `None`.
    ///
    /// Compile-fail counterpart (manual check): the following does
    /// not compile after F-SYN-001 lands:
    ///
    /// ```ignore
    /// let _ = OCAPBoundary::explicit("memory:write:any-webid".into());
    /// // error[E0599]: no function or associated item named `explicit`
    /// ```
    #[test]
    fn f_syn_001_attack_scenario_does_not_parse() {
        let attack = "memory:write:any-webid";
        // Pre-fix: this would have constructed an OCAPBoundary with
        // OcapCapability::String("memory:write:any-webid") — a
        // forgeable capability.
        // Post-fix: parse_token returns None; the call site must
        // reject the request.
        assert!(
            OCAPBoundary::parse_token(attack).is_none(),
            "F-SYN-001 invariant: the attack input must not parse into a boundary"
        );
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
