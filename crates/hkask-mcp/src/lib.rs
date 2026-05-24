//! hKask MCP — MCP runtime and dispatch
//!
//! Provides:
//! - MCP runtime for server management
//! - Capability-based dispatch with OCP
//! - Security gateway with rate limiting
//! - Audit logging
//! - Tool discovery and metadata
//! - Adapter container for shared adapter lifecycle
//! - Archival service for git operations

pub mod adapter_container;
pub mod archival_service;
pub mod dispatch;
pub mod runtime;
pub mod security;

pub use adapter_container::AdapterContainer;
pub use archival_service::ArchivalService;
pub use dispatch::McpDispatcher;
pub use runtime::ToolInfo;
pub use runtime::{McpRuntime, McpServer, McpTool};
pub use security::{
    AuditAction, AuditEntry, SecurityError, SecurityGateway, SecurityPolicy, UrlValidationConfig,
    validate_url,
};
