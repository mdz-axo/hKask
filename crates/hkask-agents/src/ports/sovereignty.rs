//! Sovereignty Port — Data types for sovereignty checking
//!
//! The canonical `SovereigntyOperation` and `SovereigntyCheckResult` definitions
//! live in `hkask_types::sovereignty`. This module re-exports them for backward
//! compatibility and extends them with the full port trait.

pub use hkask_types::sovereignty::{SovereigntyCheckResult, SovereigntyOperation};
