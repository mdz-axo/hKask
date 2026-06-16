//! Bundle composition types — conflicts and complementarities between skills

use serde::{Deserialize, Serialize};

use crate::lexicon::TemplateType;

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
    /// String representation of the conflict type (PascalCase).
    pub fn conflict_type_str(&self) -> &'static str {
        self.conflict_type.as_str()
    }
    /// String representation of the resolution strategy (PascalCase).
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
    /// String representation of the complementarity type (PascalCase).
    pub fn complementarity_type_str(&self) -> &'static str {
        self.complementarity_type.as_str()
    }
}
