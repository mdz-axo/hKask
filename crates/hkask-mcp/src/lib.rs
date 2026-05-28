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
//! - Server scaffolding (McpToolError, McpToolOutput, CredentialRequirement, ServerContext, run_stdio_server)

pub mod adapter_container;
pub mod archival_service;
pub mod dispatch;
pub mod runtime;
pub mod security;
pub mod server;
pub mod supervisor;
pub mod transport;

pub use adapter_container::AdapterContainer;
pub use archival_service::ArchivalService;
pub use dispatch::McpDispatcher;
pub use runtime::ToolInfo;
pub use runtime::{McpRuntime, McpServer, McpTool};
pub use security::{
    AuditAction, AuditEntry, SecurityError, SecurityGateway, SecurityPolicy, UrlValidationConfig,
    validate_url,
};
pub use server::{
    AuthConfig, CredentialRequirement, McpToolError, McpToolOutput, ServerContext, ToolSpanGuard,
    api_get, api_post, build_authenticated_client, classify_http_error, emit_tool_span,
    emit_tool_span_with_caller, resolve_credential, run_stdio_server, validate_identifier,
    validate_tool_url,
};
pub use supervisor::{McpSupervisor, RestartPolicy, ServerConfig, ServerStatus, SupervisionError};
pub use transport::{HttpMcpTransport, InProcessMcpTransport, McpTransport, StdioMcpTransport};
