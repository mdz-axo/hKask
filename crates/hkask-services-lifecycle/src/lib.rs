//! hKask Lifecycle Service — server health, startup/shutdown coordination.
//!
//! Extracted from `hkask-services`.
mod lifecycle_impl;
pub use lifecycle_impl::{
    LifecycleError, ServerHealth, ServerLifecycle, ServerLifecycleConfig, run_lifecycle,
};
