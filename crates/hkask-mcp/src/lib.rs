//! hKask MCP — MCP runtime and dispatch
//!
//! Provides:
//! - MCP runtime for server management
//! - Capability-based dispatch with OCP
//! - URL validation for tool endpoints
//! - Adapter container for shared adapter lifecycle
//! - Server scaffolding (McpToolError, McpToolOutput, CredentialRequirement, ServerContext, run_stdio_server)

pub mod adapter_container;
pub mod dispatch; // Loop 1 (inference tool dispatch)
pub mod governor; // Loop 6 (capability governance)
pub mod raw_tool_port; // Ungoverned tool executor (inner port for GovernedTool)
pub mod runtime;
pub mod security;
pub mod server;

pub use adapter_container::AdapterContainer;
pub use dispatch::McpDispatcher;
pub use raw_tool_port::RawMcpToolPort;
pub use runtime::{McpRuntime, McpServer, McpTool};
pub(crate) use security::UrlValidationConfig;
pub(crate) use security::validate_url;
pub use server::{
    CredentialRequirement, McpToolError, McpToolOutput, ServerContext, ToolSpanGuard, api_get,
    api_post, api_put, classify_http_error, emit_tool_span_with_caller, resolve_credential,
    run_stdio_server, validate_identifier, validate_tool_url,
};

/// Macro to eliminate MCP server boilerplate
///
/// Generates a complete `main()` function for an MCP server with stdio transport.
///
/// # Examples
///
/// Simple server with no credentials:
/// ```ignore
/// mcp_server_main!("hkask-mcp-spec", SpecServer);
/// ```
///
/// Server with required credentials:
/// ```ignore
/// mcp_server_main!(
///     "hkask-mcp-ocap",
///     OcapServer,
///     credentials: vec![
///         CredentialRequirement::required("HKASK_OCAP_SECRET", "OCAP signing secret")
///     ]
/// );
/// ```
///
/// Server with custom factory:
/// ```ignore
/// mcp_server_main!(
///     "hkask-mcp-custom",
///     factory: |ctx: ServerContext| {
///         let config = load_config()?;
///         Ok(CustomServer::new(ctx.webid, config))
///     }
/// );
/// ```
#[macro_export]
macro_rules! mcp_server_main {
    // Simple case: server name and type, no credentials
    // NB: All servers' new(webid) must return anyhow::Result<Self>
    ($name:expr, $server_type:ty) => {
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            $crate::run_stdio_server(
                $name,
                env!("CARGO_PKG_VERSION"),
                |ctx: $crate::ServerContext| <$server_type>::new(ctx.webid),
                vec![],
            )
            .await
        }
    };

    // With credentials
    // NB: All servers' new(webid) must return anyhow::Result<Self>
    ($name:expr, $server_type:ty, credentials: $creds:expr) => {
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            $crate::run_stdio_server(
                $name,
                env!("CARGO_PKG_VERSION"),
                |ctx: $crate::ServerContext| <$server_type>::new(ctx.webid),
                $creds,
            )
            .await
        }
    };

    // Custom factory
    ($name:expr, factory: $factory:expr) => {
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            $crate::run_stdio_server($name, env!("CARGO_PKG_VERSION"), $factory, vec![]).await
        }
    };

    // Custom factory with credentials
    ($name:expr, factory: $factory:expr, credentials: $creds:expr) => {
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            $crate::run_stdio_server($name, env!("CARGO_PKG_VERSION"), $factory, $creds).await
        }
    };
}
