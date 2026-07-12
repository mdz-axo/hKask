//! Shared macros for the hKask type system.
//!
//! Centralized here to eliminate the 4× duplication of `enum_str_ops!`
//! across `hkask-types`, `hkask-templates`, and `hkask-storage`.

/// Generates `as_str()` and `parse_str()` for a PascalCase enum.
///
/// `as_str()` returns the PascalCase variant name as `&'static str`.
/// `parse_str()` accepts both PascalCase and snake_case strings.
///
/// # Example
///
/// ```ignore
/// enum_str_ops!(SkillPolarity, {
///     Generative => ("Generative", "generative"),
///     Evaluative => ("Evaluative", "evaluative"),
/// });
/// ```
#[macro_export]
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
