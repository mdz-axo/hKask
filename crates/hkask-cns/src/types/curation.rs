//! Curation types — thin re-export module.
//!
//! Re-exports `CurationThresholdConfig` from its canonical home in `hkask-types`.
//! New code imports directly from `hkask_types::curator::*`.

// Re-export curation threshold config from its canonical home.
pub use hkask_types::curator::CurationThresholdConfig;
