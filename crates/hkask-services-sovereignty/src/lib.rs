//! hKask Sovereignty Service — consent revocation and data boundary enforcement.
//!
//! Extracted from `hkask-services`.
pub mod types;
mod sovereignty_impl;
pub use sovereignty_impl::SovereigntyService;
pub use types::{DataSovereigntyBoundary, UserSovereigntyState, BoundaryClassification};
