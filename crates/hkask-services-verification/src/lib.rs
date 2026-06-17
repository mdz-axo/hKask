//! hKask Verification Service — Magna Carta principle assertion checking.
//!
//! Extracted from `hkask-services`.
mod verification_impl;
pub use verification_impl::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
