//! Time utilities — Cross-cutting infrastructure
//!
//! P4.3: `now_rfc3339()` consolidates the repeated `Utc::now().to_rfc3339()`
//! pattern across all crates. Lives in `hkask-types` (the foundation crate)
//! so CLI and storage can all use it without circular dependencies.

/// Produce an RFC 3339 timestamp string for the current moment.
///
/// This is the canonical helper for "now as a string" across hKask.
/// Prefer it over inlining `chrono::Utc::now().to_rfc3339()` so that
/// any future change to the timestamp format (e.g., adding nanosecond
/// precision, switching to a different underlying clock) propagates
/// uniformly across crates.
///
/// expect: "System types preserve semantic identity and are provenance-aware"
/// pre:  (none — always callable, no arguments)
/// post: returns a valid RFC 3339 timestamp string for the current UTC moment
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
