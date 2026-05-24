//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.

pub mod acp;
pub mod acp_transport;
pub mod security_port;
pub mod sovereignty;

pub use acp::AcpPort;
pub use acp_transport::{AcpTransport, AcpWireMessage, AcpWireResponse};
pub use security_port::{RateLimitPort, ValidationError as RateLimitValidationError};
pub use sovereignty::{SovereigntyCheckResult, SovereigntyOperation, SovereigntyPort};
