//! hKask Testing Infrastructure
//!
//! This crate provides test fixtures, mocks, and integration tests for hKask.
//! Tests are excluded from the 30k Rust line budget.

pub mod ports;

pub use ports::test_fixture::*;
