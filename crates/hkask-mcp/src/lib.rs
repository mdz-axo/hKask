//! hKask MCP — MCP runtime and dispatch
//!
//! Provides:
//! - MCP runtime for server management
//! - Capability-based dispatch with OCP
//! - URL validation for tool endpoints
//! - Adapter container for shared adapter lifecycle
//! - Server scaffolding (McpToolError, McpToolOutput, CredentialRequirement, ServerContext, run_stdio_server)

pub mod adapter_container;
pub mod dispatch; // Tool dispatch through GovernedTool membrane (includes RawMcpToolPort)
pub mod git_cas;
pub mod runtime;
pub(crate) mod security;
pub mod server;

pub(crate) use adapter_container::AdapterContainer;
pub use dispatch::McpDispatcher;
pub use dispatch::RawMcpToolPort;
pub use git_cas::{GitCasAdapter, GixCasAdapter};
pub use hkask_types::ports::ToolInfo;
pub use runtime::{McpRuntime, McpServer, McpTool, ServerStartError};
pub use server::{
    CredentialRequirement, ServerContext, api_get, api_put, load_dotenv, resolve_credential,
    run_stdio_server, run_stdio_server_with_preloaded, validate_identifier,
};

/// Run an MCP server with stdio transport.
///
/// This is the canonical entry point for all hKask MCP servers.
/// Each server's `main.rs` should call this directly.
pub async fn run_server<S, F>(
    name: &str,
    version: &str,
    factory: F,
    credentials: Vec<CredentialRequirement>,
) -> anyhow::Result<()>
where
    S: rmcp::ServiceExt<rmcp::RoleServer>,
    S: rmcp::Service<rmcp::RoleServer>,
    F: FnOnce(ServerContext) -> anyhow::Result<S>,
{
    run_stdio_server(name, version, factory, credentials).await
}

/// Run an MCP server with preloaded .env credentials.
pub async fn run_server_with_preloaded<S, F>(
    name: &str,
    version: &str,
    factory: F,
    credentials: Vec<CredentialRequirement>,
    preloaded: std::collections::HashMap<String, String>,
) -> anyhow::Result<()>
where
    S: rmcp::ServiceExt<rmcp::RoleServer>,
    S: rmcp::Service<rmcp::RoleServer>,
    F: FnOnce(ServerContext) -> anyhow::Result<S>,
{
    run_stdio_server_with_preloaded(name, version, factory, credentials, preloaded).await
}

/// Macro to validate an identifier field and return early on error.
///
/// Eliminates the repeated 3-line pattern:
/// ```ignore
/// if let Err(e) = validate_identifier("field", &value, 256) {
///     return span.error(e.kind, e.to_json_string());
/// }
/// ```
///
/// Usage:
/// ```ignore
/// validate_field!(span, "session_id", &session_id, 256);
/// ```
#[macro_export]
macro_rules! validate_field {
    ($span:expr, $name:expr, $value:expr, $max_len:expr) => {
        if let Err(e) = $crate::validate_identifier($name, $value, $max_len) {
            return $span.error(e.kind, e.to_json_string());
        }
    };
}
