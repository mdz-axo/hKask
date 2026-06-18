//! Bundle composition types — conflicts and complementarities between skills

use serde::{Deserialize, Serialize};

use crate::template_type::TemplateType;

/// Generates `as_str()` and `parse_str()` for a PascalCase enum.
macro_rules! enum_str_ops {
    ($ty:ident, { $($variant:ident => ($pascal:literal, $snake:literal)),+ $(,)? }) => {
        impl $ty {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($ty::$variant => $pascal),+
                }
            }
            pub fn parse_str(s: &str) -> Option<Self> {
                match s {
                    $($pascal | $snake => Some($ty::$variant)),+,
                    _ => None,
                }
            }
        }
    };
}

/// What kind of conflict exists between two skills
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ConflictType {
    CancelOut,
    ContradictoryDirective,
    OrderingCollision,
    ResourceContention,
}

// REQ: TYP-240 (as_str), TYP-241 (parse_str)
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
// as_str pre:  self is a valid ConflictType variant
// as_str post: returns PascalCase string ("CancelOut", "ContradictoryDirective", "OrderingCollision", "ResourceContention")
// parse_str pre:  s is PascalCase or snake_case
// parse_str post: returns Some(ConflictType) if s matches; None otherwise
enum_str_ops!(ConflictType, {
    CancelOut => ("CancelOut", "cancel_out"),
    ContradictoryDirective => ("ContradictoryDirective", "contradictory_directive"),
    OrderingCollision => ("OrderingCollision", "ordering_collision"),
    ResourceContention => ("ResourceContention", "resource_contention"),
});

/// How to resolve a declared conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ConflictResolution {
    DomainSeparation,
    PhaseSeparation,
    SpecificityOverride,
    ManifestOverride,
    UserIntent,
}

// REQ: TYP-242 (as_str), TYP-243 (parse_str)
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
// as_str pre:  self is a valid ConflictResolution variant
// as_str post: returns PascalCase string ("DomainSeparation", "PhaseSeparation", "SpecificityOverride", "ManifestOverride", "UserIntent")
// parse_str pre:  s is PascalCase or snake_case
// parse_str post: returns Some(ConflictResolution) if s matches; None otherwise
enum_str_ops!(ConflictResolution, {
    DomainSeparation => ("DomainSeparation", "domain_separation"),
    PhaseSeparation => ("PhaseSeparation", "phase_separation"),
    SpecificityOverride => ("SpecificityOverride", "specificity_override"),
    ManifestOverride => ("ManifestOverride", "manifest_override"),
    UserIntent => ("UserIntent", "user_intent"),
});

/// How two skills enhance each other
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ComplementarityType {
    SequentialFeed,
    ParallelAmplify,
    CrossDomainEnhance,
}

// REQ: TYP-244 (as_str), TYP-245 (parse_str)
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
// as_str pre:  self is a valid ComplementarityType variant
// as_str post: returns PascalCase string ("SequentialFeed", "ParallelAmplify", "CrossDomainEnhance")
// parse_str pre:  s is PascalCase or snake_case
// parse_str post: returns Some(ComplementarityType) if s matches; None otherwise
enum_str_ops!(ComplementarityType, {
    SequentialFeed => ("SequentialFeed", "sequential_feed"),
    ParallelAmplify => ("ParallelAmplify", "parallel_amplify"),
    CrossDomainEnhance => ("CrossDomainEnhance", "cross_domain_enhance"),
});

/// A declared conflict between exactly two skills in a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleConflict {
    pub skills: Vec<String>,
    pub domain: TemplateType,
    pub conflict_type: ConflictType,
    pub resolution: ConflictResolution,
    pub resolution_detail: String,
}

impl BundleConflict {
    /// REQ: TYP-246
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  self.conflict_type is a valid ConflictType variant
    /// post: returns the PascalCase string representation of the conflict type
    pub fn conflict_type_str(&self) -> &'static str {
        self.conflict_type.as_str()
    }
    /// REQ: TYP-247
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  self.resolution is a valid ConflictResolution variant
    /// post: returns the PascalCase string representation of the resolution strategy
    pub fn resolution_str(&self) -> &'static str {
        self.resolution.as_str()
    }
}

/// A declared complementarity between exactly two skills in a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleComplementarity {
    pub skills: Vec<String>,
    pub complementarity_type: ComplementarityType,
    pub detail: String,
}

impl BundleComplementarity {
    /// REQ: TYP-248
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  self.complementarity_type is a valid ComplementarityType variant
    /// post: returns the PascalCase string representation of the complementarity type
    pub fn complementarity_type_str(&self) -> &'static str {
        self.complementarity_type.as_str()
    }
}
