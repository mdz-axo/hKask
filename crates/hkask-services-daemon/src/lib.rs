//! hKask Daemon Handler — pod manager bridge and daemon protocol integration.
//!
//! Extracted from `hkask-services`.
mod adaptive_monitor;
mod daemon_impl;
pub use adaptive_monitor::AdaptiveMonitor;
pub use daemon_impl::ServiceDaemonHandler;
