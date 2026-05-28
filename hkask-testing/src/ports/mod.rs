//! Test Ports - Inbound ports for hKask testing infrastructure
//!
//! These ports provide standardized interfaces for test fixtures and mocks,
//! implementing the hexagonal ports and adapters pattern.

pub mod temp_storage;
pub mod test_fixture;

pub use temp_storage::*;
pub use test_fixture::*;
