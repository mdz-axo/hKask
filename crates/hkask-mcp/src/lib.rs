//! hKask MCP — MCP runtime and dispatch
//!
//! Provides:
//! - MCP runtime for server management
//! - Capability-based dispatch with OCP
//! - Security gateway with rate limiting
//! - Audit logging
//! - Tool discovery and metadata

pub mod dispatch;
pub mod runtime;
pub mod security;

pub use dispatch::McpDispatcher;
pub use runtime::ToolInfo;
pub use runtime::{McpRuntime, McpServer, McpTool};
pub use security::{AuditAction, AuditEntry, SecurityGateway, SecurityPolicy};
