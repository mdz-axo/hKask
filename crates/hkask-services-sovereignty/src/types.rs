//! User sovereignty and affirmative consent types
//!
//! Canonical types are defined in `hkask-types::curation`.
//! This crate re-exports them and provides the service layer.

pub use hkask_types::curation::{
    BoundaryClassification, DataSovereigntyBoundary, UserSovereigntyState,
};
pub use hkask_types::{DataCategory, SovereigntyId, Visibility};
