//! hKask Sovereignty Service — consent revocation and data boundary enforcement.
//!
//! Extracted from `hkask-services`.
mod sovereignty_impl;
pub mod types;
pub use sovereignty_impl::SovereigntyService;
pub use types::{
    BoundaryClassification, DataCategory, DataSovereigntyBoundary, UserSovereigntyState,
};
