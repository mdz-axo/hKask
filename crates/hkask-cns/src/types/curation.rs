//! Curation types — thin re-export module.
//!
//! These types were consolidated into `hkask-types` (single source of truth).
//! This module provides backward-compatible re-exports for existing consumers.
//! New code should import directly from `hkask_types::curator::*`.
//!
//! Per F-SYN-001: the legacy `OcapCapability::String` variant has been removed.
//! Per F-SYN-002: `OCAPBoundary::enforced: bool` has been removed.
//! All capabilities are now unforgeable typed brands (`OcapTokenKind` in hkask-types).

// Re-export curation threshold config from its canonical home.
pub use hkask_types::curator::CurationThresholdConfig;
