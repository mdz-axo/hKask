//! hKask MCP Communication — thin MCP wrapper over core communication crate.
//!
//! Re-exports the core communication types and adds MCP-specific TTS tools.
//! All Matrix operations delegate to `hkask-communication`.

// Re-export core communication types for backward compatibility
pub use hkask_communication::agent_registration;
pub use hkask_communication::listener;
pub use hkask_communication::matrix;
