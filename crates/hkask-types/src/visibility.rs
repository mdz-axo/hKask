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

use serde::{Deserialize, Serialize};

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

    pub fn is_private(&self) -> bool {
        matches!(self, Visibility::Private)
    }

    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, Visibility::Shared)
    }
}
