//! hKask Testing Infrastructure
//!
//! This crate provides test fixtures, mocks, and integration tests for hKask.
//! Tests are excluded from the 30k Rust line budget.

pub mod integration_tests;
pub mod ports;
pub mod security;
pub mod test_harnesses;

pub use ports::*;
