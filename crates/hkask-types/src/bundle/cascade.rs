//! Cascade phase types — where a step sits in the Pre/Core/Post pipeline

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
            #[allow(dead_code)]
            pub fn parse_str(s: &str) -> Option<Self> {
                match s {
                    $($pascal | $snake => Some($ty::$variant)),+,
                    _ => None,
                }
            }
        }
    };
}

/// Cascade phase — where a step sits in the Pre/Core/Post pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CascadePhase {
    Pre,
    #[default]
    Core,
    Post,
}

// REQ: TYP-249 (as_str), TYP-250 (parse_str)
// expect: "System types preserve semantic identity and are provenance-aware" [P8]
// as_str pre:  self is a valid CascadePhase variant
// as_str post: returns PascalCase string ("Pre", "Core", "Post")
// parse_str pre:  s is PascalCase or lowercase ("Pre"/"pre", "Core"/"core", "Post"/"post")
// parse_str post: returns Some(CascadePhase) if s matches; None otherwise
enum_str_ops!(CascadePhase, {
    Pre => ("Pre", "pre"),
    Core => ("Core", "core"),
    Post => ("Post", "post"),
});
