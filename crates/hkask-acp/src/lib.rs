//! hkask-acp — ACP (Agent Client Protocol) replicant library.
//!
//! Re-exports the public API for integration testing and external consumers.

pub mod main_impl;

pub use main_impl::HkaskAcpAgent;
pub use main_impl::SessionState;
