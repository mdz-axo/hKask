//! hKask MCP — MCP runtime and dispatch
//!
//! Provides:
//! - MCP runtime for server management
//! - Capability-based dispatch with OCP
//! - Security gateway with rate limiting
//! - Audit logging

pub mod dispatch;
pub mod runtime;
pub mod security;

pub use dispatch::McpDispatcher;
pub use runtime::{McpRuntime, McpServer, McpTool};
pub use security::{AuditAction, AuditEntry, SecurityGateway, SecurityPolicy};
