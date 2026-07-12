//! hKask Onboarding Service — secrets, Matrix registration, agent setup.
//!
//! Extracted from `hkask-services`.

// Used via derive macros (serde/thiserror/async_trait) — invisible to unused_crate_dependencies lint
#![allow(unused_crate_dependencies)]
mod onboarding_impl;
pub use onboarding_impl::matrix::derive_matrix_localparts;
pub use onboarding_impl::{
    MatrixRegistrationResult, OnboardingService, RegistryHandle, ReplicantContactConfig,
    ResolvedSecrets, SignInOutcome, conduit_ensure_healthy, conduit_health_check,
};
