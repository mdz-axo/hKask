//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.

pub mod acp;
pub mod sovereignty;

pub use acp::AcpPort;
pub use sovereignty::{SovereigntyCheckResult, SovereigntyOperation, SovereigntyPort};
