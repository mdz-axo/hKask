//! hKask Onboarding Service — secrets, Matrix registration, agent setup.
//!
//! Extracted from `hkask-services`.
mod onboarding_impl;
pub use onboarding_impl::{
    MatrixRegistrationResult, OnboardingService, RegistryHandle, ReplicantContactConfig,
    ResolvedSecrets, SignInOutcome, conduit_ensure_healthy, conduit_health_check,
};
