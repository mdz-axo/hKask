//! hKask Daemon Handler — pod manager bridge and daemon protocol integration.
//!
//! Extracted from `hkask-services`.
mod daemon_impl;
pub use daemon_impl::ServiceDaemonHandler;
