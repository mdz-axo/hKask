//! Test Ports - Inbound ports for hKask testing infrastructure
//!
//! These ports provide standardized interfaces for test fixtures and mocks,
//! implementing the hexagonal ports and adapters pattern.

pub mod test_fixture;

pub use test_fixture::*;
