//! Skill polarity type — cybernetic role in a bundle or skill registry

use serde::{Deserialize, Serialize};

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

/// Skill polarity — cybernetic role in a bundle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SkillPolarity {
    Generative,
    Evaluative,
    Regulative,
    Procedural,
}

// as_str pre:  self is a valid SkillPolarity variant
// as_str post: returns PascalCase string ("Generative", "Evaluative", "Regulative", "Procedural")
// parse_str pre:  s is PascalCase or snake_case (e.g. "Generative"/"generative")
// parse_str post: returns Some(SkillPolarity) if s matches; None otherwise
enum_str_ops!(SkillPolarity, {
    Generative => ("Generative", "generative"),
    Evaluative => ("Evaluative", "evaluative"),
    Regulative => ("Regulative", "regulative"),
    Procedural => ("Procedural", "procedural"),
});
impl SkillPolarity {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SkillPolarity variant
    /// post: returns true if self is Generative (divergent/creative role); false otherwise
    pub fn is_divergent(&self) -> bool {
        matches!(self, Self::Generative)
    }
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SkillPolarity variant
    /// post: returns true if self is Evaluative (convergent/critical role); false otherwise
    pub fn is_convergent(&self) -> bool {
        matches!(self, Self::Evaluative)
    }
}
